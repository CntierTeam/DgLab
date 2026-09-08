//! DgLab Coyote V2/V3 BLE CLI/TUI.

mod app;
mod ble;
mod core;
mod headless;
mod protocol;
mod tui;

use crate::ble::{BtleSession, MockDevice};
use anyhow::{bail, Result};
use clap::Parser;
use std::io::IsTerminal;

#[derive(Parser, Debug)]
#[command(
    name = "dglab",
    about = "DG-LAB Coyote V2/V3 BLE controller (TUI + headless) — 仅学习用"
)]
struct Cli {
    /// Line-protocol mode compatible with DGLAB-BT (stdin/stdout).
    #[arg(long)]
    headless: bool,

    /// Use in-memory mock devices (no Bluetooth adapter required).
    #[arg(long)]
    mock: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.headless && !std::io::stdout().is_terminal() {
        bail!("TUI requires an interactive terminal; pass --headless for line protocol");
    }

    if cli.mock {
        let session = MockDevice::new();
        if cli.headless {
            headless::run_headless(session).await
        } else {
            tui::run_tui(session).await
        }
    } else {
        let session = BtleSession::new().await?;
        if cli.headless {
            headless::run_headless(session).await
        } else {
            tui::run_tui(session).await
        }
    }
}
