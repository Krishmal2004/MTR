use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::io;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum HomeChoice {
    CreateProject,
    BrowsePC,
}

struct MenuItem {
    icon: &'static str,
    label: &'static str,
    description: &'static str,
    choice: HomeChoice,
}

pub fn show_home() -> Option<HomeChoice> {
    enable_raw_mode().ok()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).ok()?;

    let items = vec![
        MenuItem {
            icon: "🚀",
            label: "Create New Project",
            description: "Pick a framework and scaffold a brand-new project",
            choice: HomeChoice::CreateProject,
        },
        MenuItem {
            icon: "📂",
            label: "Browse PC / File Browser",
            description: "Navigate your filesystem and pick a working directory",
            choice: HomeChoice::BrowsePC,
        },
    ];

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let result: Option<HomeChoice>;

    let logo: Vec<&str> = vec![
        r" _____ _   _ ___   ____  _   _ _   _ _   _ _____ ____  ",
        r"|_   _| | | |_ _| |  _ \| | | | \ | | \ | | ____|  _ \ ",
        r"  | | | | | || |  | |_) | | | |  \| |  \| |  _| | |_) |",
        r"  | | | |_| || |  |  _ <| |_| | |\  | |\  | |___|  _ < ",
        r"  |_|  \___/|___| |_| \_\\___/|_| \_|_| \_|_____|_| \_\",
    ];

    loop {
        terminal
            .draw(|f| {
                let size = f.size();

                let outer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),  
                        Constraint::Length(7), 
                        Constraint::Length(2),  
                        Constraint::Min(8),    
                        Constraint::Length(1),  
                    ])
                    .split(size);

                let logo_lines: Vec<Line> = logo
                    .iter()
                    .map(|l| {
                        Line::from(Span::styled(
                            *l,
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                    })
                    .collect();
                let logo_par = Paragraph::new(logo_lines).alignment(Alignment::Center);
                f.render_widget(logo_par, outer[1]);

                let sub = Paragraph::new(Line::from(vec![
                    Span::styled(
                        "  Your project runner & developer toolkit  ",
                        Style::default().fg(Color::Yellow),
                    ),
                ]))
                .alignment(Alignment::Center);
                f.render_widget(sub, outer[2]);

                let menu_width = size.width.min(62);
                let menu_x = size.width.saturating_sub(menu_width) / 2;
                let menu_area = ratatui::layout::Rect {
                    x: menu_x,
                    y: outer[3].y,
                    width: menu_width,
                    height: outer[3].height,
                };

                let menu_items: Vec<ListItem> = items
                    .iter()
                    .map(|item| {
                        ListItem::new(vec![
                            Line::from(vec![
                                Span::raw("  "),
                                Span::raw(item.icon),
                                Span::raw("  "),
                                Span::styled(
                                    item.label,
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]),
                            Line::from(vec![
                                Span::raw("        "),
                                Span::styled(
                                    item.description,
                                    Style::default().fg(Color::Gray),
                                ),
                            ]),
                            Line::from(""),
                        ])
                    })
                    .collect();

                let menu = List::new(menu_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" What would you like to do? ")
                            .border_style(Style::default().fg(Color::Yellow)),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                f.render_stateful_widget(menu, menu_area, &mut list_state);

                let hint = Paragraph::new(
                    " ↑/↓=select   Enter=open   q=quit",
                )
                .style(Style::default().fg(Color::White).bg(Color::Blue));
                f.render_widget(hint, outer[4]);
            })
            .ok();

        if event::poll(Duration::from_millis(80)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Up => {
                        let i = list_state.selected().unwrap_or(0);
                        list_state.select(Some(if i == 0 { items.len() - 1 } else { i - 1 }));
                    }
                    KeyCode::Down => {
                        let i = list_state.selected().unwrap_or(0);
                        list_state.select(Some((i + 1) % items.len()));
                    }
                    KeyCode::Enter => {
                        result = list_state
                            .selected()
                            .and_then(|i| items.get(i))
                            .map(|m| m.choice.clone());
                        break;
                    }
                    KeyCode::Char('q') => {
                        result = None;
                        break;
                    }
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        result = None;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}