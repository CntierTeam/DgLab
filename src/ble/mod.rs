//! BLE transport: trait + real btleplug session + mock.

mod mock;
mod session;

pub use mock::MockDevice;
pub use session::BtleSession;

use crate::app::{DeviceVersion, ScannedDevice};
use crate::protocol::Channel;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::mpsc;

/// Notification from device (V3 B1 / battery).
#[derive(Clone, Debug)]
pub enum BleNotify {
    Battery(u8),
    Strength { seq: u8, a: u8, b: u8 },
    Raw {
        #[allow(dead_code)]
        uuid: String,
        #[allow(dead_code)]
        data: Vec<u8>,
    },
}

/// Minimal device I/O surface. UI never touches GATT directly.
#[async_trait]
pub trait DeviceSession: Send {
    async fn scan(&mut self, timeout: Duration) -> Result<Vec<ScannedDevice>>;
    async fn connect(&mut self, address: &str) -> Result<DeviceVersion>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn read_battery(&mut self) -> Result<u8>;
    async fn read_strength_v2(&mut self) -> Result<(u16, u16)>;
    async fn write_strength_v2(&mut self, a: u16, b: u16) -> Result<()>;
    async fn write_wave_v2(&mut self, channel: Channel, x: u8, y: u16, z: u8) -> Result<()>;
    async fn write_v3(&mut self, data: &[u8]) -> Result<()>;
    async fn take_notify_rx(&mut self) -> Option<mpsc::UnboundedReceiver<BleNotify>>;
}

/// Guess version from advertised name.
pub fn guess_version(name: &str) -> Option<DeviceVersion> {
    let upper = name.to_uppercase();
    if upper.contains("D-LAB ESTIM") || upper.contains("DG-LAB") {
        Some(DeviceVersion::V2)
    } else if upper.contains("47L121") {
        Some(DeviceVersion::V3)
    } else {
        None
    }
}

pub fn is_coyote_name(name: &str) -> bool {
    guess_version(name).is_some() || {
        let u = name.to_uppercase();
        u.contains("47L12") // show other 47L12x in scan list
    }
}
