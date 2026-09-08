//! AppState + Command/Event — data is the design.
//!
//! Event names aligned with DGLAB-BT stdin/stdout protocol.

use crate::protocol::Channel;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceVersion {
    V2,
    V3,
}

impl DeviceVersion {
    pub fn as_u8(self) -> u8 {
        match self {
            DeviceVersion::V2 => 2,
            DeviceVersion::V3 => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScannedDevice {
    pub address: String,
    pub name: String,
    pub guessed_version: Option<DeviceVersion>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub connected: bool,
    pub address: Option<String>,
    pub version: Option<DeviceVersion>,
    pub battery: u8,
    pub strength_a: u16,
    pub strength_b: u16,
    pub wave_channel: Channel,
    pub wave_x: u8,
    pub wave_y: u16,
    pub wave_z: u8,
    pub devices: Vec<ScannedDevice>,
    pub selected: usize,
    pub log: VecDeque<String>,
    pub emergency: bool,
    pub limit_a: u8,
    pub limit_b: u8,
    pub pulse_freq_hz: u8,
    pub pulse_intensity: u8,
    pub scanning: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            address: None,
            version: None,
            battery: 0,
            strength_a: 0,
            strength_b: 0,
            wave_channel: Channel::A,
            wave_x: 10,
            wave_y: 50,
            wave_z: 10,
            devices: Vec::new(),
            selected: 0,
            log: VecDeque::with_capacity(200),
            emergency: false,
            limit_a: 70,
            limit_b: 70,
            pulse_freq_hz: 20,
            pulse_intensity: 30,
            scanning: false,
        }
    }
}

impl AppState {
    pub fn push_log(&mut self, line: String) {
        if self.log.len() >= 200 {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    pub fn apply_event(&mut self, ev: &Event) {
        match ev {
            Event::Msg(s) => self.push_log(format!("msg {s}")),
            Event::Start => self.push_log("event start".to_string()),
            Event::Stop => self.push_log("event stop".to_string()),
            Event::ScanStart => {
                self.scanning = true;
                self.devices.clear();
                self.selected = 0;
                self.push_log("event scanStart".to_string());
            }
            Event::ScanComplete => {
                self.scanning = false;
                self.push_log("event scanComplete".to_string());
            }
            Event::DeviceFound { address, name } => {
                self.devices.push(ScannedDevice {
                    address: address.clone(),
                    name: name.clone(),
                    guessed_version: crate::ble::guess_version(name),
                });
                self.push_log(format!("event deviceFound {address}"));
            }
            Event::DeviceVersion(v) => {
                self.version = Some(*v);
                self.push_log(format!("event deviceVersion {}", v.as_u8()));
            }
            Event::ConnectStart => self.push_log("event connectStart".to_string()),
            Event::ConnectSucceed => {
                self.connected = true;
                self.emergency = false;
                self.push_log("event connectSucceed".to_string());
            }
            Event::ConnectFailed => {
                self.connected = false;
                self.version = None;
                self.push_log("event connectFailed".to_string());
            }
            Event::UpdateBattery(b) => {
                self.battery = *b;
                self.push_log(format!("event updateBattery {b}"));
            }
            Event::UpdateStrength { a, b } => {
                self.strength_a = *a;
                self.strength_b = *b;
                self.push_log(format!("event updateStrength {a} {b}"));
            }
            Event::UpdateWave { x, y, z } => {
                self.wave_x = *x;
                self.wave_y = *y;
                self.wave_z = *z;
                self.push_log(format!("event updateWave {x} {y} {z}"));
            }
            Event::Disconnected => {
                self.connected = false;
                self.version = None;
                self.address = None;
                self.push_log("msg disconnected".to_string());
            }
        }
    }
}

/// Commands from TUI / headless → core worker.
#[derive(Clone, Debug)]
pub enum Command {
    Scan,
    Connect(String),
    Disconnect,
    GetBattery,
    SetStrength { a: u16, b: u16 },
    GetStrength,
    SendWave {
        channel: Channel,
        x: u8,
        y: u16,
        z: u8,
    },
    /// Exit process (DGLAB-BT `stop`).
    Stop,
    EmergencyZero,
    SetLimits { a: u8, b: u8 },
    SetPulsePattern { freq_hz: u8, intensity: u8 },
}

/// Events from core → UI / stdout (DGLAB-BT names).
#[derive(Clone, Debug)]
pub enum Event {
    Msg(String),
    Start,
    Stop,
    ScanStart,
    ScanComplete,
    DeviceFound { address: String, name: String },
    DeviceVersion(DeviceVersion),
    ConnectStart,
    ConnectSucceed,
    ConnectFailed,
    UpdateBattery(u8),
    UpdateStrength { a: u16, b: u16 },
    UpdateWave { x: u8, y: u16, z: u8 },
    Disconnected,
}

impl Event {
    /// Format as DGLAB-BT line protocol.
    pub fn to_line(&self) -> String {
        match self {
            Event::Msg(s) => format!("msg {s}"),
            Event::Start => "event start".into(),
            Event::Stop => "event stop".into(),
            Event::ScanStart => "event scanStart".into(),
            Event::ScanComplete => "event scanComplete".into(),
            Event::DeviceFound { address, .. } => format!("event deviceFound {address}"),
            Event::DeviceVersion(v) => format!("event deviceVersion {}", v.as_u8()),
            Event::ConnectStart => "event connectStart".into(),
            Event::ConnectSucceed => "event connectSucceed".into(),
            Event::ConnectFailed => "event connectFailed".into(),
            Event::UpdateBattery(b) => format!("event updateBattery {b}"),
            Event::UpdateStrength { a, b } => format!("event updateStrength {a} {b}"),
            Event::UpdateWave { x, y, z } => format!("event updateWave {x} {y} {z}"),
            Event::Disconnected => "msg disconnected".into(),
        }
    }
}

/// Parse one DGLAB-BT input line into a Command.
pub fn parse_command_line(line: &str) -> Option<Command> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "stop" => Some(Command::Stop),
        "scan" => Some(Command::Scan),
        "connect" if parts.len() >= 2 => Some(Command::Connect(parts[1].to_string())),
        "getBattery" => Some(Command::GetBattery),
        "setStrength" if parts.len() >= 3 => {
            let a = parts[1].parse().ok()?;
            let b = parts[2].parse().ok()?;
            Some(Command::SetStrength { a, b })
        }
        "getStrength" => Some(Command::GetStrength),
        "sendWave" if parts.len() >= 5 => {
            let channel = Channel::parse(parts[1])?;
            let x = parts[2].parse().ok()?;
            let y = parts[3].parse().ok()?;
            let z = parts[4].parse().ok()?;
            Some(Command::SendWave { channel, x, y, z })
        }
        "emergency" | "zero" => Some(Command::EmergencyZero),
        "setLimits" if parts.len() >= 3 => {
            let a = parts[1].parse().ok()?;
            let b = parts[2].parse().ok()?;
            Some(Command::SetLimits { a, b })
        }
        "setPulse" if parts.len() >= 3 => {
            let freq_hz = parts[1].parse().ok()?;
            let intensity = parts[2].parse().ok()?;
            Some(Command::SetPulsePattern { freq_hz, intensity })
        }
        "disconnect" => Some(Command::Disconnect),
        _ => None,
    }
}
