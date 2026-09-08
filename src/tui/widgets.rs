//! ratatui widgets for device list, bars, wave form, log.

use crate::app::{AppState, DeviceVersion};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Devices,
    Strength,
    Wave,
    Log,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::Devices => Focus::Strength,
            Focus::Strength => Focus::Wave,
            Focus::Wave => Focus::Log,
            Focus::Log => Focus::Devices,
        }
    }
}

pub fn draw(frame: &mut Frame, state: &AppState, focus: Focus) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, root[0], state);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(root[1]);

    draw_devices(frame, mid[0], state, focus == Focus::Devices);
    draw_strength(frame, mid[1], state, focus == Focus::Strength);
    draw_wave(frame, mid[2], state, focus == Focus::Wave);
    draw_log(frame, root[2], state, focus == Focus::Log);
    draw_help(frame, root[3]);
}

fn title_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let status = if state.connected {
        let ver = state
            .version
            .map(|v| format!("V{}", v.as_u8()))
            .unwrap_or_else(|| "?".into());
        let addr = state.address.as_deref().unwrap_or("-");
        format!("CONNECTED {ver} @ {addr}")
    } else if state.scanning {
        "SCANNING...".into()
    } else {
        "DISCONNECTED".into()
    };
    let batt = if state.connected {
        format!(" battery {}%", state.battery)
    } else {
        String::new()
    };
    let emergency = if state.emergency { " [E-STOP]" } else { "" };
    let text = Paragraph::new(Line::from(vec![
        Span::styled(
            " DgLab ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {status}{batt}{emergency}")),
    ]))
    .block(Block::default().borders(Borders::ALL).title("status"));
    frame.render_widget(text, area);
}

fn draw_devices(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let items: Vec<ListItem> = state
        .devices
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let ver = match d.guessed_version {
                Some(DeviceVersion::V2) => "V2",
                Some(DeviceVersion::V3) => "V3",
                None => "?",
            };
            let mark = if i == state.selected { ">" } else { " " };
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{mark} [{ver}] {} {}", d.address, d.name)).style(style)
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("devices (s scan / c connect)")
            .title_style(title_style(active)),
    );
    frame.render_widget(list, area);
}

fn draw_strength(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .margin(1)
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("strength ([ ] A / - = B / Space STOP)")
        .title_style(title_style(active));
    frame.render_widget(block, area);

    let max = match state.version {
        Some(DeviceVersion::V3) => 200.0,
        _ => 100.0,
    };
    let a_ratio = (state.strength_a as f64 / max).clamp(0.0, 1.0);
    let b_ratio = (state.strength_b as f64 / max).clamp(0.0, 1.0);
    let batt_ratio = (state.battery as f64 / 100.0).clamp(0.0, 1.0);

    frame.render_widget(
        Gauge::default()
            .block(Block::default().title(format!("A {}", state.strength_a)))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(a_ratio),
        chunks[0],
    );
    frame.render_widget(
        Gauge::default()
            .block(Block::default().title(format!("B {}", state.strength_b)))
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(b_ratio),
        chunks[1],
    );
    frame.render_widget(
        Gauge::default()
            .block(Block::default().title(format!("Battery {}%", state.battery)))
            .gauge_style(Style::default().fg(Color::Yellow))
            .ratio(batt_ratio),
        chunks[2],
    );

    let limits = format!(
        "V3 limits A={} B={}  pulse {}Hz/{}",
        state.limit_a, state.limit_b, state.pulse_freq_hz, state.pulse_intensity
    );
    frame.render_widget(Paragraph::new(limits), chunks[3]);
}

fn draw_wave(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let ch = match state.wave_channel {
        crate::protocol::Channel::A => "A",
        crate::protocol::Channel::B => "B",
    };
    let body = format!(
        "channel: {ch}  (w toggle)\n\
         x={}  y={}  z={}\n\
         (h/l x · j/k y · u/i z)\n\
         Enter = sendWave\n\
         V3: maps to 100ms pulse pattern\n\
         limits: L/K edit via setLimits cmds",
        state.wave_x, state.wave_y, state.wave_z
    );
    let p = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("wave")
                .title_style(title_style(active)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn draw_log(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let lines: Vec<Line> = state
        .log
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| Line::from(s.as_str()))
        .collect();
    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("log (msg/event)")
            .title_style(title_style(active)),
    );
    frame.render_widget(p, area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help = "q quit · Tab focus · s scan · c connect · g battery · Space/0 e-stop · [/] A · -/= B";
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
