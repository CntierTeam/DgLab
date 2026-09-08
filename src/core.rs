//! Core worker: Command → DeviceSession → Event, plus V3 100ms B0 tick.

use crate::app::{Command, DeviceVersion, Event};
use crate::ble::{BleNotify, DeviceSession};
use crate::protocol::{Channel, IntensityMode, Pulse, V3};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

struct V3Runtime {
    seq: u8,
    pending_mode_a: IntensityMode,
    pending_mode_b: IntensityMode,
    pending_a: u8,
    pending_b: u8,
    /// When true, skip strength change until B1 acks (simple gating).
    ack_wait: bool,
    pulses_a: [Pulse; 4],
    pulses_b: [Pulse; 4],
    limit_a: u8,
    limit_b: u8,
    active: bool,
}

impl Default for V3Runtime {
    fn default() -> Self {
        let mut pulses_b = [Pulse::IDLE; 4];
        pulses_b[3].intensity = 101;
        Self {
            seq: 0,
            pending_mode_a: IntensityMode::DoNotChange,
            pending_mode_b: IntensityMode::DoNotChange,
            pending_a: 0,
            pending_b: 0,
            ack_wait: false,
            pulses_a: [Pulse {
                frequency_hz: 20,
                intensity: 0,
            }; 4],
            pulses_b,
            limit_a: 70,
            limit_b: 70,
            active: false,
        }
    }
}

impl V3Runtime {
    fn build_b0(&mut self) -> [u8; 20] {
        let (mode_a, mode_b, sa, sb, seq) = if self.ack_wait {
            (
                IntensityMode::DoNotChange,
                IntensityMode::DoNotChange,
                0,
                0,
                0,
            )
        } else if self.pending_mode_a != IntensityMode::DoNotChange
            || self.pending_mode_b != IntensityMode::DoNotChange
        {
            self.seq = self.seq.wrapping_add(1) & 0x0F;
            if self.seq == 0 {
                self.seq = 1;
            }
            let seq = self.seq;
            let mode_a = self.pending_mode_a;
            let mode_b = self.pending_mode_b;
            let sa = self.pending_a;
            let sb = self.pending_b;
            self.pending_mode_a = IntensityMode::DoNotChange;
            self.pending_mode_b = IntensityMode::DoNotChange;
            self.pending_a = 0;
            self.pending_b = 0;
            if mode_a != IntensityMode::DoNotChange || mode_b != IntensityMode::DoNotChange {
                self.ack_wait = true;
            }
            (mode_a, mode_b, sa, sb, seq)
        } else {
            (
                IntensityMode::DoNotChange,
                IntensityMode::DoNotChange,
                0,
                0,
                0,
            )
        };

        V3::encode_b0(
            seq,
            mode_a,
            mode_b,
            sa,
            sb,
            self.pulses_a,
            self.pulses_b,
        )
    }
}

type V3Shared = Arc<Mutex<V3Runtime>>;

pub async fn run_core<S: DeviceSession + 'static>(
    mut session: S,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<()> {
    let _ = event_tx.send(Event::Msg(
        "DGLAB蓝牙核心V0.1.0已启动！Rust/btleplug（仅学习用）".into(),
    ));
    let _ = event_tx.send(Event::Start);

    let v3: V3Shared = Arc::new(Mutex::new(V3Runtime::default()));
    let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let mut version: Option<DeviceVersion> = None;
    let mut tick_handle: Option<tokio::task::JoinHandle<()>> = None;
    let mut notify_handle: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            biased;

            Some(bytes) = write_rx.recv() => {
                if let Err(e) = session.write_v3(&bytes).await {
                    let _ = event_tx.send(Event::Msg(format!("V3 写入失败：{e}")));
                }
            }

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break; };
                match cmd {
                    Command::Stop => {
                        abort(&mut tick_handle, &mut notify_handle);
                        { v3.lock().await.active = false; }
                        let _ = session.disconnect().await;
                        let _ = event_tx.send(Event::Stop);
                        break;
                    }
                    Command::Scan => {
                        let _ = event_tx.send(Event::Msg("正在搜索设备......".into()));
                        let _ = event_tx.send(Event::ScanStart);
                        match session.scan(Duration::from_secs(4)).await {
                            Ok(devices) => {
                                for d in devices {
                                    let _ = event_tx.send(Event::Msg(format!(
                                        "发现设备：{}/{}",
                                        d.address, d.name
                                    )));
                                    let _ = event_tx.send(Event::DeviceFound {
                                        address: d.address,
                                        name: d.name,
                                    });
                                }
                                let _ = event_tx.send(Event::Msg("设备搜索完成！".into()));
                            }
                            Err(e) => {
                                let _ = event_tx.send(Event::Msg(format!("扫描失败：{e}")));
                            }
                        }
                        let _ = event_tx.send(Event::ScanComplete);
                    }
                    Command::Connect(addr) => {
                        abort(&mut tick_handle, &mut notify_handle);
                        { v3.lock().await.active = false; }
                        let _ = event_tx.send(Event::Msg("正在连接设备......".into()));
                        let _ = event_tx.send(Event::ConnectStart);
                        match session.connect(&addr).await {
                            Ok(ver) => {
                                version = Some(ver);
                                let _ = event_tx.send(Event::DeviceVersion(ver));
                                let _ = event_tx.send(Event::ConnectSucceed);

                                if let Some(rx) = session.take_notify_rx().await {
                                    let vtx = event_tx.clone();
                                    let v3c = v3.clone();
                                    notify_handle = Some(tokio::spawn(async move {
                                        forward_notify(rx, vtx, v3c).await;
                                    }));
                                }

                                if ver == DeviceVersion::V3 {
                                    {
                                        let mut g = v3.lock().await;
                                        *g = V3Runtime::default();
                                        g.active = true;
                                    }
                                    let bf = {
                                        let g = v3.lock().await;
                                        V3::encode_bf(g.limit_a, g.limit_b, 160, 160, 0, 0)
                                    };
                                    if let Err(e) = session.write_v3(&bf).await {
                                        let _ = event_tx.send(Event::Msg(format!("BF 写入失败：{e}")));
                                    }
                                    tick_handle = Some(spawn_tick(v3.clone(), write_tx.clone()));
                                }

                                if let Ok(b) = session.read_battery().await {
                                    let _ = event_tx.send(Event::UpdateBattery(b));
                                }
                            }
                            Err(e) => {
                                version = None;
                                let _ = event_tx.send(Event::Msg(format!("连接失败：{e}")));
                                let _ = event_tx.send(Event::ConnectFailed);
                            }
                        }
                    }
                    Command::Disconnect => {
                        abort(&mut tick_handle, &mut notify_handle);
                        { v3.lock().await.active = false; }
                        let _ = session.disconnect().await;
                        version = None;
                        let _ = event_tx.send(Event::Disconnected);
                    }
                    Command::GetBattery => match session.read_battery().await {
                        Ok(b) => { let _ = event_tx.send(Event::UpdateBattery(b)); }
                        Err(e) => { let _ = event_tx.send(Event::Msg(format!("读电量失败：{e}"))); }
                    },
                    Command::SetStrength { a, b } => match version {
                        Some(DeviceVersion::V2) => {
                            if let Err(e) = session.write_strength_v2(a, b).await {
                                let _ = event_tx.send(Event::Msg(format!("设强度失败：{e}")));
                            } else {
                                let _ = event_tx.send(Event::UpdateStrength { a, b });
                            }
                        }
                        Some(DeviceVersion::V3) => {
                            let mut g = v3.lock().await;
                            g.pending_mode_a = IntensityMode::Absolute;
                            g.pending_mode_b = IntensityMode::Absolute;
                            g.pending_a = a.min(200) as u8;
                            g.pending_b = b.min(200) as u8;
                            g.ack_wait = false;
                            let _ = event_tx.send(Event::UpdateStrength { a, b });
                        }
                        None => { let _ = event_tx.send(Event::Msg("未连接".into())); }
                    },
                    Command::GetStrength => match version {
                        Some(DeviceVersion::V2) => match session.read_strength_v2().await {
                            Ok((a, b)) => { let _ = event_tx.send(Event::UpdateStrength { a, b }); }
                            Err(e) => { let _ = event_tx.send(Event::Msg(format!("读强度失败：{e}"))); }
                        },
                        Some(DeviceVersion::V3) => {
                            let _ = event_tx.send(Event::Msg(
                                "V3 强度由 B1 通知更新".into(),
                            ));
                        }
                        None => { let _ = event_tx.send(Event::Msg("未连接".into())); }
                    },
                    Command::SendWave { channel, x, y, z } => match version {
                        Some(DeviceVersion::V2) => {
                            if let Err(e) = session.write_wave_v2(channel, x, y, z).await {
                                let _ = event_tx.send(Event::Msg(format!("发波形失败：{e}")));
                            } else {
                                let _ = event_tx.send(Event::UpdateWave { x, y, z });
                            }
                        }
                        Some(DeviceVersion::V3) => {
                            let (pa, pb) = V3::pattern_from_xyz(x, y, z);
                            let mut g = v3.lock().await;
                            g.pulses_a = pa;
                            g.pulses_b = pb;
                            let _ = event_tx.send(Event::UpdateWave { x, y, z });
                        }
                        None => { let _ = event_tx.send(Event::Msg("未连接".into())); }
                    },
                    Command::EmergencyZero => match version {
                        Some(DeviceVersion::V2) => {
                            let _ = session.write_strength_v2(0, 0).await;
                            let _ = session.write_wave_v2(Channel::A, 0, 0, 0).await;
                            let _ = session.write_wave_v2(Channel::B, 0, 0, 0).await;
                            let _ = event_tx.send(Event::UpdateStrength { a: 0, b: 0 });
                            let _ = event_tx.send(Event::UpdateWave { x: 0, y: 0, z: 0 });
                            let _ = event_tx.send(Event::Msg("紧急停止".into()));
                        }
                        Some(DeviceVersion::V3) => {
                            {
                                let mut g = v3.lock().await;
                                g.pending_mode_a = IntensityMode::Absolute;
                                g.pending_mode_b = IntensityMode::Absolute;
                                g.pending_a = 0;
                                g.pending_b = 0;
                                g.pulses_a = [Pulse { frequency_hz: 10, intensity: 0 }; 4];
                                let mut b = [Pulse::IDLE; 4];
                                b[3].intensity = 101;
                                g.pulses_b = b;
                                g.ack_wait = false;
                            }
                            let pkt = V3::encode_emergency_zero(1);
                            let _ = session.write_v3(&pkt).await;
                            let _ = event_tx.send(Event::UpdateStrength { a: 0, b: 0 });
                            let _ = event_tx.send(Event::Msg("紧急停止".into()));
                        }
                        None => { let _ = event_tx.send(Event::Msg("未连接".into())); }
                    },
                    Command::SetLimits { a, b } => {
                        {
                            let mut g = v3.lock().await;
                            g.limit_a = a.min(200);
                            g.limit_b = b.min(200);
                        }
                        if version == Some(DeviceVersion::V3) {
                            let bf = V3::encode_bf(a.min(200), b.min(200), 160, 160, 0, 0);
                            if let Err(e) = session.write_v3(&bf).await {
                                let _ = event_tx.send(Event::Msg(format!("setLimits 失败：{e}")));
                            } else {
                                let _ = event_tx.send(Event::Msg(format!("limits {a} {b}")));
                            }
                        }
                    }
                    Command::SetPulsePattern { freq_hz, intensity } => {
                        let mut g = v3.lock().await;
                        g.pulses_a = [Pulse {
                            frequency_hz: freq_hz.clamp(1, 100),
                            intensity: intensity.min(100),
                        }; 4];
                        let _ = event_tx.send(Event::Msg(format!(
                            "pulse freq={freq_hz} intensity={intensity}"
                        )));
                    }
                }
            }
        }
    }

    Ok(())
}

fn abort(
    tick: &mut Option<tokio::task::JoinHandle<()>>,
    notify: &mut Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(h) = tick.take() {
        h.abort();
    }
    if let Some(h) = notify.take() {
        h.abort();
    }
}

fn spawn_tick(
    v3: V3Shared,
    write_tx: mpsc::UnboundedSender<Vec<u8>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let pkt = {
                let mut g = v3.lock().await;
                if !g.active {
                    break;
                }
                g.build_b0()
            };
            if write_tx.send(pkt.to_vec()).is_err() {
                break;
            }
        }
    })
}

async fn forward_notify(
    mut rx: mpsc::UnboundedReceiver<BleNotify>,
    event_tx: mpsc::UnboundedSender<Event>,
    v3: V3Shared,
) {
    while let Some(n) = rx.recv().await {
        match n {
            BleNotify::Battery(b) => {
                let _ = event_tx.send(Event::UpdateBattery(b));
            }
            BleNotify::Strength { seq, a, b } => {
                {
                    let mut g = v3.lock().await;
                    if seq == g.seq || seq == 0 {
                        g.ack_wait = false;
                    }
                }
                let _ = event_tx.send(Event::UpdateStrength {
                    a: u16::from(a),
                    b: u16::from(b),
                });
            }
            BleNotify::Raw { .. } => {}
        }
    }
}
