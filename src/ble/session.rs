//! Real BLE via btleplug (Linux BlueZ / Cross-platform).

use super::{guess_version, is_coyote_name, BleNotify, DeviceSession};
use crate::app::{DeviceVersion, ScannedDevice};
use crate::protocol::{Channel, V2, V3};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use btleplug::api::{
    bleuuid::uuid_from_u16, Central, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct BtleSession {
    adapter: Option<Adapter>,
    peripheral: Option<Peripheral>,
    version: Option<DeviceVersion>,
    chars: HashMap<Uuid, Characteristic>,
    notify_rx: Option<mpsc::UnboundedReceiver<BleNotify>>,
}

impl BtleSession {
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await.context("btleplug Manager")?;
        let adapters = manager.adapters().await.context("list adapters")?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no Bluetooth adapter found"))?;
        Ok(Self {
            adapter: Some(adapter),
            peripheral: None,
            version: None,
            chars: HashMap::new(),
            notify_rx: None,
        })
    }

    fn adapter(&self) -> Result<&Adapter> {
        self.adapter
            .as_ref()
            .ok_or_else(|| anyhow!("no adapter"))
    }

    fn peripheral(&self) -> Result<&Peripheral> {
        self.peripheral
            .as_ref()
            .ok_or_else(|| anyhow!("not connected"))
    }

    fn char(&self, uuid: Uuid) -> Result<&Characteristic> {
        self.chars
            .get(&uuid)
            .ok_or_else(|| anyhow!("missing characteristic {uuid}"))
    }

    async fn find_peripheral(&self, address: &str) -> Result<Peripheral> {
        let adapter = self.adapter()?;
        let target = address.to_lowercase();
        for p in adapter.peripherals().await? {
            let addr = p.address().to_string().to_lowercase();
            if addr == target {
                return Ok(p);
            }
            // Also match id string forms
            if p.id().to_string().to_lowercase() == target {
                return Ok(p);
            }
        }
        Err(anyhow!("device {address} not found — scan first"))
    }

    async fn start_notify_task(&mut self, peripheral: Peripheral) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut stream = peripheral.notifications().await?;
        tokio::spawn(async move {
            while let Some(n) = stream.next().await {
                let event = if n.uuid == V3::BATTERY || n.uuid == V2::BATTERY {
                    n.value
                        .first()
                        .copied()
                        .map(BleNotify::Battery)
                } else if n.uuid == V3::NOTIFY {
                    V3::decode_b1(&n.value).map(|(seq, a, b)| BleNotify::Strength { seq, a, b })
                } else {
                    Some(BleNotify::Raw {
                        uuid: n.uuid.to_string(),
                        data: n.value,
                    })
                };
                if let Some(e) = event {
                    if tx.send(e).is_err() {
                        break;
                    }
                }
            }
        });
        self.notify_rx = Some(rx);
        Ok(())
    }
}

#[async_trait]
impl DeviceSession for BtleSession {
    async fn scan(&mut self, timeout: Duration) -> Result<Vec<ScannedDevice>> {
        let adapter = self.adapter()?.clone();
        adapter.start_scan(ScanFilter::default()).await?;
        tokio::time::sleep(timeout).await;
        adapter.stop_scan().await?;

        let mut out = Vec::new();
        let mut seen = HashMap::new();
        for p in adapter.peripherals().await? {
            let props = p.properties().await?.unwrap_or_default();
            let name = props.local_name.unwrap_or_default();
            if name.is_empty() || !is_coyote_name(&name) {
                continue;
            }
            let address = p.address().to_string();
            if seen.insert(address.clone(), ()).is_some() {
                continue;
            }
            out.push(ScannedDevice {
                address,
                name: name.clone(),
                guessed_version: guess_version(&name),
            });
        }
        Ok(out)
    }

    async fn connect(&mut self, address: &str) -> Result<DeviceVersion> {
        if self.peripheral.is_some() {
            let _ = self.disconnect().await;
        }
        let peripheral = self.find_peripheral(address).await?;
        peripheral.connect().await.context("connect")?;
        peripheral.discover_services().await.context("discover")?;

        let mut version = None;
        self.chars.clear();
        for service in peripheral.services() {
            let su = service.uuid;
            if su == V2::SERVICE {
                version = Some(DeviceVersion::V2);
            }
            if su == V3::SERVICE || su == uuid_from_u16(0x180C) {
                version = Some(DeviceVersion::V3);
            }
            for c in service.characteristics {
                self.chars.insert(c.uuid, c);
            }
        }

        // Also index by short form matches on known UUIDs
        for c in peripheral.characteristics() {
            self.chars.insert(c.uuid, c);
        }

        let version = version.ok_or_else(|| anyhow!("not a Coyote V2/V3 service set"))?;
        self.version = Some(version);

        // Subscribe notifies
        if version == DeviceVersion::V3 {
            if let Ok(c) = self.char(V3::NOTIFY).cloned() {
                let _ = peripheral.subscribe(&c).await;
            }
            if let Ok(c) = self.char(V3::BATTERY).cloned() {
                let _ = peripheral.subscribe(&c).await;
            }
        }

        self.start_notify_task(peripheral.clone()).await?;
        self.peripheral = Some(peripheral);
        Ok(version)
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(p) = self.peripheral.take() {
            let _ = p.disconnect().await;
        }
        self.version = None;
        self.chars.clear();
        self.notify_rx = None;
        Ok(())
    }

    async fn read_battery(&mut self) -> Result<u8> {
        let p = self.peripheral()?;
        let uuid = match self.version {
            Some(DeviceVersion::V2) => V2::BATTERY,
            Some(DeviceVersion::V3) => V3::BATTERY,
            None => return Err(anyhow!("not connected")),
        };
        let c = self.char(uuid)?;
        let data = p.read(c).await?;
        data.first()
            .copied()
            .ok_or_else(|| anyhow!("empty battery read"))
    }

    async fn read_strength_v2(&mut self) -> Result<(u16, u16)> {
        let p = self.peripheral()?;
        let c = self.char(V2::STRENGTH)?;
        let data = p.read(c).await?;
        V2::decode_strength(&data).ok_or_else(|| anyhow!("bad strength payload"))
    }

    async fn write_strength_v2(&mut self, a: u16, b: u16) -> Result<()> {
        let p = self.peripheral()?;
        let c = self.char(V2::STRENGTH)?;
        let data = V2::encode_strength(a, b);
        p.write(c, &data, WriteType::WithoutResponse).await?;
        Ok(())
    }

    async fn write_wave_v2(
        &mut self,
        channel: Channel,
        x: u8,
        y: u16,
        z: u8,
    ) -> Result<()> {
        let p = self.peripheral()?;
        let c = self.char(V2::wave_uuid(channel))?;
        let data = V2::encode_wave(x, y, z);
        p.write(c, &data, WriteType::WithoutResponse).await?;
        Ok(())
    }

    async fn write_v3(&mut self, data: &[u8]) -> Result<()> {
        let p = self.peripheral()?;
        let c = self.char(V3::WRITE)?;
        p.write(c, data, WriteType::WithoutResponse).await?;
        Ok(())
    }

    async fn take_notify_rx(&mut self) -> Option<mpsc::UnboundedReceiver<BleNotify>> {
        self.notify_rx.take()
    }
}
