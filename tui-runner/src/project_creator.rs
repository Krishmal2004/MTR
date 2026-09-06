use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub struct Framework {
    pub icon: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub language: &'static str,
    pub cmd: &'static str,
    pub args: &'static [&'static str],
}

pub const FRAMEWORKS: &[Framework] = &[
    Framework {
        icon: "⚛ ",
        name: "React  (Vite)",
        description: "Fast React SPA with Vite bundler",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "vite@latest", "{name}", "--", "--template", "react"],
    },
    Framework {
        icon: "▲ ",
        name: "Next.js",
        description: "Full-stack React framework with SSR/SSG",
        language: "JavaScript / TypeScript",
        cmd: "npx",
        args: &["create-next-app@latest", "{name}", "--yes"],
    },
    Framework {
        icon: "◈ ",
        name: "Vue 3  (Vite)",
        description: "Progressive JavaScript framework",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "vite@latest", "{name}", "--", "--template", "vue"],
    },
    Framework {
        icon: "A ",
        name: "Angular",
        description: "Enterprise-grade Angular application",
        language: "TypeScript",
        cmd: "npx",
        args: &["-p", "@angular/cli", "ng", "new", "{name}", "--defaults"],
    },
    Framework {
        icon: "S ",
        name: "SvelteKit",
        description: "Cybernetically enhanced web apps",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "svelte@latest", "{name}"],
    },
    Framework {
        icon: "* ",
        name: "Flutter",
        description: "Cross-platform mobile, web & desktop",
        language: "Dart",
        cmd: "flutter",
        args: &["create", "{name}"],
    },
    Framework {
        icon: "R ",
        name: "Rust  (Cargo)",
        description: "New Rust binary crate",
        language: "Rust",
        cmd: "cargo",
        args: &["new", "{name}"],
    },
    Framework {
        icon: ". ",
        name: ".NET Web API",
        description: "ASP.NET Core minimal or controller API",
        language: "C#",
        cmd: "dotnet",
        args: &["new", "webapi", "-n", "{name}"],
    },
    Framework {
        icon: "N ",
        name: "Node.js  (Express)",
        description: "Express REST API server",
        language: "JavaScript",
        cmd: "npx",
        args: &["express-generator", "--no-view", "{name}"],
    },
    Framework {
        icon: "P ",
        name: "Python  (FastAPI)",
        description: "Modern async Python web API",
        language: "Python",
        cmd: "python",
        args: &["-m", "venv", "{name}"],
    },
];

fn is_installed(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}
pub fn available_frameworks() -> Vec<(usize, &'static Framework)> {
    FRAMEWORKS
        .iter()
        .enumerate()
        .filter(|(_, fw)| {
            let check = if fw.cmd == "npx" { "npm" } else { fw.cmd };
            is_installed(check)
        })
        .collect()
}

pub fn select_framework() -> Option<usize> {
    enable_raw_mode().ok()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).ok()?;

    let available = available_frameworks();

    if available.is_empty() {
        disable_raw_mode().ok();
        execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
        println!();
        println!("  No supported frameworks detected on this machine.");
        println!("  Install one or more of the following tools first:");
        println!("    npm / node   → React, Next.js, Vue, Angular, SvelteKit, Node.js");
        println!("    flutter      → Flutter");
        println!("    cargo        → Rust");
        println!("    dotnet       → .NET Web API");
        println!("    python       → Python");
        println!();
        println!("  Press Enter to go back...");
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        return None;
    }

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let result: Option<usize>;

    loop {
        terminal
            .draw(|f| {
                let size = f.size();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // title
                        Constraint::Min(1),    // framework list
                        Constraint::Length(4), // detail panel
                        Constraint::Length(1), // hint bar
                    ])
                    .split(size);

                let detected_count = format!(
                    "  {} framework(s) detected on your PC",
                    available.len()
                );
                let title = Paragraph::new(Line::from(vec![
                    Span::styled(
                        " 🚀 CREATE NEW PROJECT ",
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        detected_count,
                        Style::default().fg(Color::Green),
                    ),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                );
                f.render_widget(title, chunks[0]);

                let list_items: Vec<ListItem> = available
                    .iter()
                    .map(|(_, fw)| {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(fw.icon, Style::default().fg(Color::Cyan)),
                            Span::raw(" "),
                            Span::styled(
                                fw.name,
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw("  "),
                            Span::styled(
                                fw.language,
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]))
                    })
                    .collect();

                let list = List::new(list_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Installed Frameworks ")
                            .border_style(Style::default().fg(Color::Cyan)),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Cyan)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                f.render_stateful_widget(list, chunks[1], &mut list_state);

                let detail_lines = if let Some(sel) = list_state.selected() {
                    if let Some((_, fw)) = available.get(sel) {
                        vec![
                            Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    fw.name,
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]),
                            Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    fw.description,
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                            Line::from(vec![
                                Span::raw("  cmd: "),
                                Span::styled(
                                    format!("{} {}", fw.cmd, fw.args.join(" ")),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]),
                        ]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                let detail = Paragraph::new(detail_lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Details ")
                        .border_style(Style::default().fg(Color::Yellow)),
                );
                f.render_widget(detail, chunks[2]);

                let hint = Paragraph::new(
                    " ↑/↓=select   Enter=choose   Esc=back to home",
                )
                .style(Style::default().fg(Color::White).bg(Color::Blue));
                f.render_widget(hint, chunks[3]);
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
                        list_state.select(Some(
                            if i == 0 { available.len() - 1 } else { i - 1 },
                        ));
                    }
                    KeyCode::Down => {
                        let i = list_state.selected().unwrap_or(0);
                        list_state.select(Some((i + 1) % available.len()));
                    }
                    KeyCode::Enter => {
                        result = list_state
                            .selected()
                            .and_then(|i| available.get(i))
                            .map(|(orig_idx, _)| *orig_idx);
                        break;
                    }
                    KeyCode::Esc => {
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

pub fn scaffold_project(framework_idx: usize, project_path: &PathBuf) -> anyhow::Result<()> {
    let fw = &FRAMEWORKS[framework_idx];

    let project_name = project_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".to_string());

    let parent = project_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| project_path.clone());

    let resolved_args: Vec<String> = fw
        .args
        .iter()
        .map(|a| a.replace("{name}", &project_name))
        .collect();

    println!();
    println!("  Scaffolding {} project ...", fw.name);
    println!("  Name   : {}", project_name);
    println!("  Folder : {}", parent.display());
    println!("  Cmd    : {} {}", fw.cmd, resolved_args.join(" "));
    println!();

    let status = std::process::Command::new(fw.cmd)
        .args(&resolved_args)
        .current_dir(&parent)
        .status()?;

    if status.success() {
        println!();
        println!("  Project '{}' created successfully!", project_name);
        println!("  Path: {}", project_path.display());
    } else {
        println!();
        println!("  Scaffold exited with code {:?}", status.code());
        println!("  Make sure '{}' is installed and on PATH.", fw.cmd);
    }

    println!();
    println!("  Press Enter to continue...");
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();

    Ok(())
}