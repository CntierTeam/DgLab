//! In-memory device for --mock (no BLE hardware).

use super::{guess_version, BleNotify, DeviceSession};
use crate::app::{DeviceVersion, ScannedDevice};
use crate::protocol::{Channel, V2};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct MockDevice {
    connected: bool,
    version: Option<DeviceVersion>,
    battery: u8,
    strength_a: u16,
    strength_b: u16,
    notify_tx: Option<mpsc::UnboundedSender<BleNotify>>,
    notify_rx: Option<mpsc::UnboundedReceiver<BleNotify>>,
    last_v3: Vec<u8>,
}

impl MockDevice {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            connected: false,
            version: None,
            battery: 88,
            strength_a: 0,
            strength_b: 0,
            notify_tx: Some(tx),
            notify_rx: Some(rx),
            last_v3: Vec::new(),
        }
    }
}

#[async_trait]
impl DeviceSession for MockDevice {
    async fn scan(&mut self, _timeout: Duration) -> Result<Vec<ScannedDevice>> {
        Ok(vec![
            ScannedDevice {
                address: "AA:BB:CC:DD:EE:01".into(),
                name: "D-LAB ESTIM01".into(),
                guessed_version: guess_version("D-LAB ESTIM01"),
            },
            ScannedDevice {
                address: "AA:BB:CC:DD:EE:03".into(),
                name: "47L121000".into(),
                guessed_version: guess_version("47L121000"),
            },
        ])
    }

    async fn connect(&mut self, address: &str) -> Result<DeviceVersion> {
        let version = if address.ends_with("01") {
            DeviceVersion::V2
        } else if address.ends_with("03") {
            DeviceVersion::V3
        } else {
            return Err(anyhow!("mock: unknown address {address}"));
        };
        self.connected = true;
        self.version = Some(version);
        if version == DeviceVersion::V3 {
            // Default soft limits after connect (BF would be written by core).
            if let Some(tx) = &self.notify_tx {
                let _ = tx.send(BleNotify::Strength {
                    seq: 0,
                    a: 0,
                    b: 0,
                });
            }
        }
        Ok(version)
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        self.version = None;
        Ok(())
    }

    async fn read_battery(&mut self) -> Result<u8> {
        if !self.connected {
            return Err(anyhow!("not connected"));
        }
        Ok(self.battery)
    }

    async fn read_strength_v2(&mut self) -> Result<(u16, u16)> {
        if self.version != Some(DeviceVersion::V2) {
            return Err(anyhow!("not V2"));
        }
        Ok((self.strength_a, self.strength_b))
    }

    async fn write_strength_v2(&mut self, a: u16, b: u16) -> Result<()> {
        if self.version != Some(DeviceVersion::V2) {
            return Err(anyhow!("not V2"));
        }
        let packed = V2::encode_strength(a, b);
        let (da, db) = V2::decode_strength(&packed).unwrap();
        self.strength_a = da;
        self.strength_b = db;
        Ok(())
    }

    async fn write_wave_v2(
        &mut self,
        _channel: Channel,
        _x: u8,
        _y: u16,
        _z: u8,
    ) -> Result<()> {
        if self.version != Some(DeviceVersion::V2) {
            return Err(anyhow!("not V2"));
        }
        Ok(())
    }

    async fn write_v3(&mut self, data: &[u8]) -> Result<()> {
        if self.version != Some(DeviceVersion::V3) {
            return Err(anyhow!("not V3"));
        }
        self.last_v3 = data.to_vec();
        if data.first() == Some(&0xB0) && data.len() >= 4 {
            let mode = data[1] & 0x0F;
            let seq = data[1] >> 4;
            let mut a = self.strength_a as u8;
            let mut b = self.strength_b as u8;
            let mode_a = mode >> 2;
            let mode_b = mode & 0x03;
            a = apply_mode(a, mode_a, data[2]);
            b = apply_mode(b, mode_b, data[3]);
            self.strength_a = u16::from(a);
            self.strength_b = u16::from(b);
            if seq > 0 || mode_a != 0 || mode_b != 0 {
                if let Some(tx) = &self.notify_tx {
                    let _ = tx.send(BleNotify::Strength { seq, a, b });
                }
            }
        }
        Ok(())
    }

    async fn take_notify_rx(&mut self) -> Option<mpsc::UnboundedReceiver<BleNotify>> {
        self.notify_rx.take()
    }
}

fn apply_mode(cur: u8, mode: u8, val: u8) -> u8 {
    let v = if val > 200 { 0 } else { val };
    match mode {
        0b01 => cur.saturating_add(v).min(200),
        0b10 => cur.saturating_sub(v),
        0b11 => v,
        _ => cur,
    }
}
