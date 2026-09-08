//! DGLAB-BT compatible headless line protocol (stdin → Command, Event → stdout).

use crate::app::{parse_command_line, Command, Event};
use crate::ble::DeviceSession;
use crate::core;
use anyhow::Result;
use std::io::{self, BufRead, Write};
use tokio::sync::mpsc;

pub async fn run_headless<S: DeviceSession + 'static>(session: S) -> Result<()> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();

    let core_handle = tokio::spawn(async move {
        if let Err(e) = core::run_core(session, cmd_rx, event_tx).await {
            eprintln!("core error: {e}");
        }
    });

    let printer = tokio::spawn(async move {
        while let Some(ev) = event_rx.recv().await {
            let line = ev.to_line();
            let mut out = io::stdout().lock();
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
            if matches!(ev, Event::Stop) {
                break;
            }
        }
    });

    let stdin_task = tokio::task::spawn_blocking(move || {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = cmd_tx.send(Command::Stop);
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match parse_command_line(trimmed) {
                        Some(cmd) => {
                            let stop = matches!(cmd, Command::Stop);
                            if cmd_tx.send(cmd).is_err() {
                                break;
                            }
                            if stop {
                                break;
                            }
                        }
                        None => {
                            let mut out = io::stdout().lock();
                            let _ = writeln!(out, "msg unknown command: {trimmed}");
                            let _ = out.flush();
                        }
                    }
                }
                Err(_) => {
                    let _ = cmd_tx.send(Command::Stop);
                    break;
                }
            }
        }
    });

    let _ = stdin_task.await;
    let _ = printer.await;
    let _ = core_handle.await;
    Ok(())
}
