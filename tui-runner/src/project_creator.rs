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

    pub create_own_dir: bool,

    pub install_cmd: Option<(&'static str, &'static [&'static str])>,
}

pub const FRAMEWORKS: &[Framework] = &[
    Framework {
        icon: "⚛ ",
        name: "React  (Vite)",
        description: "Fast React SPA with Vite, initialized in the folder you pick",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "vite@latest", ".", "--", "--template", "react"],
        create_own_dir: false,
        install_cmd: Some(("npm", &["install"])),
    },
    Framework {
        icon: "▲ ",
        name: "Next.js",
        description: "Full-stack React framework, initialized in the folder you pick",
        language: "JavaScript / TypeScript",
        cmd: "npx",
        args: &["create-next-app@latest", ".", "--yes"],
        create_own_dir: false,
        install_cmd: None, // create-next-app installs deps itself
    },
    Framework {
        icon: "◈ ",
        name: "Vue 3  (Vite)",
        description: "Progressive JS framework, initialized in the folder you pick",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "vite@latest", ".", "--", "--template", "vue"],
        create_own_dir: false,
        install_cmd: Some(("npm", &["install"])),
    },
    Framework {
        icon: "A ",
        name: "Angular",
        description: "Enterprise-grade Angular app, initialized in the folder you pick",
        language: "TypeScript",
        cmd: "npx",
        args: &["-p", "@angular/cli", "ng", "new", "{name}", "--directory=.", "--defaults"],
        create_own_dir: false,
        install_cmd: None, // ng new installs deps itself
    },
    Framework {
        icon: "S ",
        name: "SvelteKit",
        description: "Cybernetically enhanced web app, initialized in the folder you pick",
        language: "JavaScript / TypeScript",
        cmd: "npm",
        args: &["create", "svelte@latest", "."],
        create_own_dir: false,
        install_cmd: Some(("npm", &["install"])),
    },
    Framework {
        icon: "🦕",
        name: "Deno",
        description: "Secure TS/JS runtime project, initialized in the folder you pick",
        language: "TypeScript / JavaScript",
        cmd: "deno",
        args: &["init", "."],
        create_own_dir: false,
        install_cmd: None, // Deno resolves remote imports on first run, no node_modules step
    },
    Framework {
        icon: "🥟",
        name: "Bun",
        description: "All-in-one JS/TS runtime project, initialized in the folder you pick",
        language: "TypeScript / JavaScript",
        cmd: "bun",
        args: &["init", "-y"],
        create_own_dir: false,
        install_cmd: Some(("bun", &["install"])),
    },
    Framework {
        icon: "N ",
        name: "Node.js  (Express)",
        description: "Express REST API server, initialized in the folder you pick",
        language: "JavaScript",
        cmd: "npx",
        args: &["express-generator", "--no-view", "."],
        create_own_dir: false,
        install_cmd: Some(("npm", &["install"])),
    },
    Framework {
        icon: "* ",
        name: "Flutter",
        description: "Cross-platform mobile/web/desktop app, initialized in the folder you pick",
        language: "Dart",
        cmd: "flutter",
        args: &["create", "."],
        create_own_dir: false,
        install_cmd: None, // flutter create runs `pub get` itself
    },
    Framework {
        icon: "R ",
        name: "Rust  (Cargo)",
        description: "Rust binary crate, initialized in the folder you pick",
        language: "Rust",
        cmd: "cargo",
        args: &["init"],
        create_own_dir: false,
        install_cmd: Some(("cargo", &["fetch"])),
    },
    Framework {
        icon: ". ",
        name: ".NET Web API",
        description: "ASP.NET Core web API, initialized in the folder you pick",
        language: "C#",
        cmd: "dotnet",
        args: &["new", "webapi", "-n", "{name}"],
        create_own_dir: false,
        install_cmd: Some(("dotnet", &["restore"])),
    },
    Framework {
        icon: "🐹",
        name: "Go  (module)",
        description: "Go module, go.mod initialized in the folder you pick",
        language: "Go",
        cmd: "go",
        args: &["mod", "init", "{name}"],
        create_own_dir: false,
        install_cmd: Some(("go", &["mod", "tidy"])),
    },
    Framework {
        icon: "🅑 ",
        name: "Ballerina",
        description: "Ballerina package, initialized in the folder you pick",
        language: "Ballerina",
        cmd: "bal",
        args: &["new", "."],
        create_own_dir: false,
        install_cmd: None,
    },
    Framework {
        icon: "💧",
        name: "Elixir  (Mix)",
        description: "Elixir project, initialized in the folder you pick",
        language: "Elixir",
        cmd: "mix",
        args: &["new", "."],
        create_own_dir: false,
        install_cmd: Some(("mix", &["deps.get"])),
    },
    Framework {
        icon: "🍎",
        name: "Swift Package",
        description: "Executable Swift package, initialized in the folder you pick",
        language: "Swift",
        cmd: "swift",
        args: &["package", "init", "--type", "executable"],
        create_own_dir: false,
        install_cmd: Some(("swift", &["package", "resolve"])),
    },
    Framework {
        icon: "☕",
        name: "Java  (Maven)",
        description: "Maven quickstart archetype (creates a nested artifactId subfolder)",
        language: "Java",
        cmd: "mvn",
        args: &[
            "archetype:generate",
            "-DgroupId=com.example",
            "-DartifactId={name}",
            "-DarchetypeArtifactId=maven-archetype-quickstart",
            "-DarchetypeVersion=1.4",
            "-DinteractiveMode=false",
        ],
        create_own_dir: true,
        install_cmd: None, // archetype:generate already resolves the archetype's own deps
    },
    Framework {
        icon: "P ",
        name: "Python  (venv)",
        description: "Virtual environment created inside the folder you pick",
        language: "Python",
        cmd: "python",
        args: &["-m", "venv", "venv"],
        create_own_dir: false,
        install_cmd: None, // fresh venv has no requirements.txt yet
    },
    Framework {
        icon: "🐍",
        name: "Python  (Poetry)",
        description: "Poetry-managed package, initialized in the folder you pick",
        language: "Python",
        cmd: "poetry",
        args: &["init", "--no-interaction"],
        create_own_dir: false,
        install_cmd: Some(("poetry", &["install", "--no-interaction", "--no-root"])),
    },
    Framework {
        icon: "🐘",
        name: "PHP  (Laravel)",
        description: "Laravel web application, installed into the folder you pick",
        language: "PHP",
        cmd: "composer",
        args: &["create-project", "laravel/laravel", "."],
        create_own_dir: false,
        install_cmd: None, // composer create-project already installs deps itself
    },
];

fn is_installed(cmd: &str) -> bool {
    crate::langscan::find_on_path(cmd).is_some()
}

/// Resolves a bare command name to its full path (with extension) found on PATH.
/// This matters on Windows: tools like npm/npx/flutter are `.cmd`/`.bat` shims, and
/// `Command::new("npm")` fails with "program not found" because Windows only resolves
/// bare names to `.exe` automatically, not `.cmd`/`.bat`. Falls back to the bare name
/// (e.g. on Unix, or if it isn't found — the spawn error will surface either way).
fn resolve_cmd(cmd: &'static str) -> std::ffi::OsString {
    crate::langscan::find_on_path(cmd)
        .map(|p| p.into_os_string())
        .unwrap_or_else(|| cmd.into())
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
        println!("    npm / node   → React, Next.js, Vue, Angular, SvelteKit, Express");
        println!("    deno / bun   → Deno, Bun");
        println!("    flutter      → Flutter");
        println!("    cargo        → Rust");
        println!("    dotnet       → .NET Web API");
        println!("    go           → Go module");
        println!("    bal          → Ballerina");
        println!("    mix          → Elixir");
        println!("    swift        → Swift Package");
        println!("    mvn          → Java (Maven)");
        println!("    python       → Python (venv)");
        println!("    poetry       → Python (Poetry)");
        println!("    composer     → PHP (Laravel)");
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

    let run_dir = if fw.create_own_dir {
        parent.clone()
    } else {
        std::fs::create_dir_all(project_path)?;
        project_path.clone()
    };

    println!();
    println!("  Scaffolding {} project ...", fw.name);
    println!("  Name   : {}", project_name);
    println!("  Folder : {}", run_dir.display());
    println!("  Cmd    : {} {}", fw.cmd, resolved_args.join(" "));
    println!();

    let status = match std::process::Command::new(resolve_cmd(fw.cmd))
        .args(&resolved_args)
        .current_dir(&run_dir)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            println!();
            println!("  Could not run '{}': {}", fw.cmd, e);
            println!("  Make sure '{}' is installed and on PATH.", fw.cmd);
            println!();
            println!("  Press Enter to continue...");
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).ok();
            return Ok(());
        }
    };

    if status.success() {
        if let Some((icmd, iargs)) = fw.install_cmd {
            println!();
            println!("  Installing dependencies ...");
            println!("  Cmd    : {} {}", icmd, iargs.join(" "));
            println!();

            let install_status = std::process::Command::new(resolve_cmd(icmd))
                .args(iargs)
                .current_dir(project_path)
                .status();

            match install_status {
                Ok(s) if s.success() => {
                    println!("  Dependencies installed.");
                }
                Ok(s) => {
                    println!(
                        "  Dependency install exited with code {:?} (project files are still in place).",
                        s.code()
                    );
                }
                Err(e) => {
                    println!("  Could not run '{}': {}", icmd, e);
                }
            }
        }

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