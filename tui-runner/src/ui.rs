use crate::config::ProcessConfig;
use crate::engine::{LogEvent, LogKind, ProcHandle};
use crate::header::header_lines;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_LINES: usize = 2000;

fn color_from_name(name: &str) -> Color {
    match name {
        "green" => Color::Green,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "red" => Color::Red,
        "white" => Color::White,
        _ => Color::White,
    }
}

struct Pane {
    name: String,
    color: Color,
    lines: VecDeque<Line<'static>>,
    scroll_from_bottom: u16,
}

pub async fn run_ui(
    title: String,
    processes: &[ProcessConfig],
    mut rx: mpsc::UnboundedReceiver<LogEvent>,
    handles: Vec<Arc<ProcHandle>>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let palette = ["green", "cyan", "magenta", "yellow", "blue", "red", "white"];
    let mut panes: Vec<Pane> = processes
        .iter()
        .enumerate()
        .map(|(i, p)| Pane {
            name: p.name.clone(),
            color: color_from_name(p.color.as_deref().unwrap_or(palette[i % palette.len()])),
            lines: VecDeque::new(),
            scroll_from_bottom: 0,
        })
        .collect();

    let mut focus: usize = 0;
    let header = header_lines(&title);
    let header_height = header.len() as u16 + 2;

    let result = loop {
        // Drain any pending log events without blocking the redraw loop.
        while let Ok(ev) = rx.try_recv() {
            if let Some(pane) = panes.get_mut(ev.pane) {
                let style = match ev.kind {
                    LogKind::Stderr => Style::default().fg(Color::Red),
                    LogKind::System => Style::default().add_modifier(Modifier::BOLD),
                    LogKind::Stdout => Style::default(),
                };
                pane.lines.push_back(Line::from(Span::styled(ev.line, style)));
                if pane.lines.len() > MAX_LINES {
                    pane.lines.pop_front();
                }
            }
        }

        terminal.draw(|f| {
            let size = f.size();
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            let header_text: Vec<Line> = header.iter().map(|l| Line::from(l.clone())).collect();
            let header_par = Paragraph::new(header_text)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(header_par, outer[0]);

            let n = panes.len().max(1);
            let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Percentage((100 / n) as u16)).collect();
            let cols = Layout::default().direction(Direction::Horizontal).constraints(constraints).split(outer[1]);

            for (i, pane) in panes.iter().enumerate() {
                let area = cols[i];
                let visible_height = area.height.saturating_sub(2) as usize;
                let total = pane.lines.len();
                let scroll_from_bottom = pane.scroll_from_bottom as usize;
                let end = total.saturating_sub(scroll_from_bottom);
                let start = end.saturating_sub(visible_height);
                let visible: Vec<Line> = pane.lines.iter().skip(start).take(end - start).cloned().collect();

                let border_style = if i == focus {
                    Style::default().fg(pane.color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(pane.color)
                };
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", pane.name))
                    .border_style(border_style);
                let par = Paragraph::new(visible).block(block);
                f.render_widget(par, area);
            }

            let status = Paragraph::new(Line::from(
                " Tab=switch pane  \u{2191}/\u{2193}=scroll  q / Ctrl+C=quit all",
            ))
            .style(Style::default().fg(Color::White).bg(Color::Blue));
            f.render_widget(status, outer[2]);
        })?;

        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break Ok(()),
                    KeyCode::Tab => {
                        if !panes.is_empty() {
                            focus = (focus + 1) % panes.len();
                        }
                    }
                    KeyCode::Up => {
                        if let Some(p) = panes.get_mut(focus) {
                            p.scroll_from_bottom = p.scroll_from_bottom.saturating_add(1);
                        }
                    }
                    KeyCode::Down => {
                        if let Some(p) = panes.get_mut(focus) {
                            p.scroll_from_bottom = p.scroll_from_bottom.saturating_sub(1);
                        }
                    }
                    KeyCode::PageUp => {
                        if let Some(p) = panes.get_mut(focus) {
                            p.scroll_from_bottom = p.scroll_from_bottom.saturating_add(10);
                        }
                    }
                    KeyCode::PageDown => {
                        if let Some(p) = panes.get_mut(focus) {
                            p.scroll_from_bottom = p.scroll_from_bottom.saturating_sub(10);
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("Shutting down all processes...");
    for handle in &handles {
        handle.kill().await;
    }
    // Give processes a brief moment to exit cleanly.
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("Done. All processes stopped.");

    result
}
