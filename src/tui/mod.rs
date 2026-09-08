//! ratatui main loop.

mod widgets;

use crate::app::{AppState, Command, Event};
use crate::ble::DeviceSession;
use crate::core;
use crate::protocol::Channel;
use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self,Stdout};
use std::time::Duration;
use tokio::sync::mpsc;
use widgets::{draw, Focus};

pub async fn run_tui<S: DeviceSession + 'static>(session: S) -> Result<()> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();

    let core_handle = tokio::spawn(async move {
        if let Err(e) = core::run_core(session, cmd_rx, event_tx).await {
            eprintln!("core error: {e}");
        }
    });

    let mut terminal = setup_terminal()?;
    let mut state = AppState::default();
    let mut focus = Focus::Devices;
    let mut quitting = false;

    tokio::time::sleep(Duration::from_millis(20)).await;

    let result = loop {
        while let Ok(ev) = event_rx.try_recv() {
            if let Event::ConnectSucceed = &ev {
                if let Some(d) = state.devices.get(state.selected) {
                    state.address = Some(d.address.clone());
                }
            }
            let stop = matches!(ev, Event::Stop);
            state.apply_event(&ev);
            if stop {
                quitting = true;
            }
        }
        if quitting {
            break Ok(());
        }

        terminal.draw(|f| draw(f, &state, focus))?;

        if event::poll(Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => {
                        let _ = cmd_tx.send(Command::Stop);
                    }
                    KeyCode::Tab => focus = focus.next(),
                    KeyCode::Char('s') => {
                        let _ = cmd_tx.send(Command::Scan);
                    }
                    KeyCode::Char('c') => {
                        if let Some(d) = state.devices.get(state.selected) {
                            state.address = Some(d.address.clone());
                            let _ = cmd_tx.send(Command::Connect(d.address.clone()));
                        }
                    }
                    KeyCode::Char('g') => {
                        let _ = cmd_tx.send(Command::GetBattery);
                    }
                    KeyCode::Char(' ') | KeyCode::Char('0') => {
                        state.emergency = true;
                        state.strength_a = 0;
                        state.strength_b = 0;
                        let _ = cmd_tx.send(Command::EmergencyZero);
                    }
                    KeyCode::Up | KeyCode::Char('p') => {
                        if state.selected > 0 {
                            state.selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('n') => {
                        if !state.devices.is_empty() {
                            state.selected = (state.selected + 1).min(state.devices.len() - 1);
                        }
                    }
                    KeyCode::Char('[') => {
                        state.strength_a = state.strength_a.saturating_sub(1);
                        let _ = cmd_tx.send(Command::SetStrength {
                            a: state.strength_a,
                            b: state.strength_b,
                        });
                    }
                    KeyCode::Char(']') => {
                        let max = if state.version == Some(crate::app::DeviceVersion::V3) {
                            200
                        } else {
                            100
                        };
                        state.strength_a = (state.strength_a + 1).min(max);
                        let _ = cmd_tx.send(Command::SetStrength {
                            a: state.strength_a,
                            b: state.strength_b,
                        });
                    }
                    KeyCode::Char('-') => {
                        state.strength_b = state.strength_b.saturating_sub(1);
                        let _ = cmd_tx.send(Command::SetStrength {
                            a: state.strength_a,
                            b: state.strength_b,
                        });
                    }
                    KeyCode::Char('=') => {
                        let max = if state.version == Some(crate::app::DeviceVersion::V3) {
                            200
                        } else {
                            100
                        };
                        state.strength_b = (state.strength_b + 1).min(max);
                        let _ = cmd_tx.send(Command::SetStrength {
                            a: state.strength_a,
                            b: state.strength_b,
                        });
                    }
                    KeyCode::Char('w') => {
                        state.wave_channel = match state.wave_channel {
                            Channel::A => Channel::B,
                            Channel::B => Channel::A,
                        };
                    }
                    KeyCode::Char('h') => {
                        state.wave_x = state.wave_x.saturating_sub(1);
                    }
                    KeyCode::Char('l') => {
                        state.wave_x = state.wave_x.saturating_add(1).min(31);
                    }
                    KeyCode::Char('j') => {
                        state.wave_y = state.wave_y.saturating_sub(1);
                    }
                    KeyCode::Char('k') => {
                        state.wave_y = (state.wave_y + 1).min(1023);
                    }
                    KeyCode::Char('u') => {
                        state.wave_z = state.wave_z.saturating_sub(1);
                    }
                    KeyCode::Char('i') => {
                        state.wave_z = state.wave_z.saturating_add(1).min(31);
                    }
                    KeyCode::Enter => {
                        let _ = cmd_tx.send(Command::SendWave {
                            channel: state.wave_channel,
                            x: state.wave_x,
                            y: state.wave_y,
                            z: state.wave_z,
                        });
                        if state.version == Some(crate::app::DeviceVersion::V3) {
                            let _ = cmd_tx.send(Command::SetPulsePattern {
                                freq_hz: state.wave_x.max(1),
                                intensity: (state.wave_y.min(100)) as u8,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    restore_terminal(&mut terminal)?;
    let _ = core_handle.await;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
