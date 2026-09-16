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
use std::env;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct LangInfo {
    pub icon: &'static str,
    pub name: &'static str,
    pub cmd: &'static str,
    pub version_arg: &'static str,
    /// A well-known "home" environment variable this toolchain sets (e.g. JAVA_HOME),
    /// shown as an extra detection signal when present.
    pub home_env: Option<&'static str>,
}

const LANGUAGES: &[LangInfo] = &[
    LangInfo { icon: "🟨", name: "Node.js / npm", cmd: "node", version_arg: "--version", home_env: Some("NVM_DIR") },
    LangInfo { icon: "🦕", name: "Deno", cmd: "deno", version_arg: "--version", home_env: Some("DENO_INSTALL") },
    LangInfo { icon: "🥟", name: "Bun", cmd: "bun", version_arg: "--version", home_env: Some("BUN_INSTALL") },
    LangInfo { icon: "🐍", name: "Python", cmd: "python", version_arg: "--version", home_env: Some("PYENV_ROOT") },
    LangInfo { icon: "🦀", name: "Rust / Cargo", cmd: "cargo", version_arg: "--version", home_env: Some("CARGO_HOME") },
    LangInfo { icon: "🔷", name: ".NET / C#", cmd: "dotnet", version_arg: "--version", home_env: Some("DOTNET_ROOT") },
    LangInfo { icon: "☕", name: "Java (JDK)", cmd: "java", version_arg: "-version", home_env: Some("JAVA_HOME") },
    LangInfo { icon: "🟣", name: "Kotlin", cmd: "kotlin", version_arg: "-version", home_env: None },
    LangInfo { icon: "⚙ ", name: "Gradle", cmd: "gradle", version_arg: "--version", home_env: Some("GRADLE_USER_HOME") },
    LangInfo { icon: "🅼 ", name: "Maven", cmd: "mvn", version_arg: "--version", home_env: Some("M2_HOME") },
    LangInfo { icon: "🌀", name: "Scala", cmd: "scala", version_arg: "-version", home_env: None },
    LangInfo { icon: "🐹", name: "Go", cmd: "go", version_arg: "version", home_env: Some("GOROOT") },
    LangInfo { icon: "💎", name: "Ruby", cmd: "ruby", version_arg: "--version", home_env: Some("RBENV_ROOT") },
    LangInfo { icon: "🐘", name: "PHP", cmd: "php", version_arg: "--version", home_env: None },
    LangInfo { icon: "🎼", name: "Composer", cmd: "composer", version_arg: "--version", home_env: Some("COMPOSER_HOME") },
    LangInfo { icon: "🎯", name: "Flutter / Dart", cmd: "flutter", version_arg: "--version", home_env: Some("FLUTTER_ROOT") },
    LangInfo { icon: "🍎", name: "Swift", cmd: "swift", version_arg: "--version", home_env: None },
    LangInfo { icon: "🅲 ", name: "C / GCC", cmd: "gcc", version_arg: "--version", home_env: None },
    LangInfo { icon: "🅲+", name: "C++ / Clang", cmd: "clang", version_arg: "--version", home_env: None },
    LangInfo { icon: "🅑 ", name: "Ballerina", cmd: "bal", version_arg: "version", home_env: Some("BALLERINA_HOME") },
    LangInfo { icon: "💧", name: "Elixir", cmd: "elixir", version_arg: "--version", home_env: None },
    LangInfo { icon: "⚗ ", name: "Erlang", cmd: "erl", version_arg: "-version", home_env: None },
    LangInfo { icon: "🐪", name: "Perl", cmd: "perl", version_arg: "--version", home_env: None },
    LangInfo { icon: "🌙", name: "Lua", cmd: "lua", version_arg: "-v", home_env: None },
    LangInfo { icon: "📊", name: "R", cmd: "Rscript", version_arg: "--version", home_env: Some("R_HOME") },
    LangInfo { icon: "λ ", name: "Haskell / GHC", cmd: "ghc", version_arg: "--version", home_env: Some("STACK_ROOT") },
    LangInfo { icon: "🔬", name: "Julia", cmd: "julia", version_arg: "--version", home_env: Some("JULIA_DEPOT_PATH") },
    LangInfo { icon: "⚡", name: "Zig", cmd: "zig", version_arg: "version", home_env: None },
    LangInfo { icon: "🔷", name: "PowerShell", cmd: "pwsh", version_arg: "--version", home_env: None },
    LangInfo { icon: "🔧", name: "Git", cmd: "git", version_arg: "--version", home_env: None },
    LangInfo { icon: "🐳", name: "Docker", cmd: "docker", version_arg: "--version", home_env: Some("DOCKER_HOST") },
];

/// Executable name variants to look for on PATH, honoring the OS's own convention
/// (Windows resolves bare commands through PATHEXT; Unix has no extension).
fn exe_candidates(cmd: &str) -> Vec<String> {
    if cfg!(windows) {
        let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
        let mut names: Vec<String> = pathext
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|ext| format!("{cmd}{}", ext.to_lowercase()))
            .collect();
        names.push(cmd.to_string());
        names
    } else {
        vec![cmd.to_string()]
    }
}

/// Walks the PATH environment variable the same way the OS shell would, split with
/// the platform-correct separator (`;` on Windows, `:` elsewhere) via `env::split_paths`.
pub fn find_on_path(cmd: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH").or_else(|| env::var_os("Path"))?;
    let candidates = exe_candidates(cmd);
    for dir in env::split_paths(&path_var) {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

/// Best-effort OS summary read straight from the environment rather than hard-coded,
/// so the report reflects whatever OS/architecture this process is actually running on.
pub fn os_summary() -> String {
    if cfg!(windows) {
        let os = env::var("OS").unwrap_or_else(|_| std::env::consts::OS.to_string());
        let arch = env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| std::env::consts::ARCH.to_string());
        format!("{} / {}", os, arch)
    } else {
        let os = env::var("OSTYPE").unwrap_or_else(|_| std::env::consts::OS.to_string());
        format!("{} / {}", os, std::env::consts::ARCH)
    }
}

pub struct Detected {
    pub info: &'static LangInfo,
    pub installed: bool,
    pub version: String,
    /// The home env var name+value when the toolchain's env var is set, e.g. ("JAVA_HOME", "C:\...").
    pub env_hint: Option<(&'static str, String)>,
}

fn probe(info: &'static LangInfo) -> Detected {
    // Primary signal: walk PATH (OS-aware separator/extensions) rather than only
    // relying on spawning the process, since PATH is the source of truth the OS
    // itself uses to resolve a bare command.
    let on_path = find_on_path(info.cmd).is_some();

    // The home env var is auxiliary diagnostic info (e.g. "JAVA_HOME is set but java
    // isn't resolvable on PATH"), it doesn't by itself count as "installed" — PATH
    // resolution is what the OS actually uses to run a bare command.
    let env_hint = info
        .home_env
        .and_then(|var| env::var(var).ok().map(|val| (var, val)));

    let installed = on_path;

    let version = if installed {
        Command::new(info.cmd)
            .arg(info.version_arg)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .ok()
            .map(|out| {
                let raw = if !out.stdout.is_empty() { out.stdout } else { out.stderr };
                String::from_utf8_lossy(&raw)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    Detected { info, installed, version, env_hint }
}

/// Scans the local machine for every language/runtime this tool knows how to scaffold with.
pub fn scan_languages() -> Vec<Detected> {
    LANGUAGES.iter().map(probe).collect()
}

/// Shows a read-only report of every language detected on this PC.
/// Returns true to continue to framework selection, false to go back home.
pub fn show_language_report(detected: &[Detected]) -> Option<bool> {
    enable_raw_mode().ok()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).ok()?;

    let installed_count = detected.iter().filter(|d| d.installed).count();

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let result: Option<bool>;

    loop {
        terminal
            .draw(|f| {
                let size = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ])
                    .split(size);

                let title = Paragraph::new(Line::from(vec![
                    Span::styled(
                        " 🖥  LANGUAGES ON THIS PC ",
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("[{}]", os_summary()),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{} / {} detected", installed_count, detected.len()),
                        Style::default().fg(Color::Magenta),
                    ),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                );
                f.render_widget(title, chunks[0]);

                let items: Vec<ListItem> = detected
                    .iter()
                    .map(|d| {
                        let status = if d.installed {
                            Span::styled(
                                "  ✔ installed  ",
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled(
                                "  ✘ not found  ",
                                Style::default().fg(Color::DarkGray),
                            )
                        };
                        let name_style = if d.installed {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        let env_span = match &d.env_hint {
                            Some((var, val)) => Span::styled(
                                format!("  [{}={}]", var, val),
                                Style::default().fg(Color::DarkGray),
                            ),
                            None => Span::raw(""),
                        };
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::raw(d.info.icon),
                            Span::raw(" "),
                            Span::styled(format!("{:<18}", d.info.name), name_style),
                            status,
                            Span::styled(d.version.clone(), Style::default().fg(Color::Cyan)),
                            env_span,
                        ]))
                    })
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Detected Runtimes & Tools ")
                            .border_style(Style::default().fg(Color::Cyan)),
                    )
                    .highlight_style(
                        Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");
                f.render_stateful_widget(list, chunks[1], &mut list_state);

                let hint = Paragraph::new(
                    " ↑/↓=scroll   Enter=continue to framework picker   Esc=back to home",
                )
                .style(Style::default().fg(Color::White).bg(Color::Blue));
                f.render_widget(hint, chunks[2]);
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
                        list_state.select(Some(if i == 0 { detected.len() - 1 } else { i - 1 }));
                    }
                    KeyCode::Down => {
                        let i = list_state.selected().unwrap_or(0);
                        list_state.select(Some((i + 1) % detected.len()));
                    }
                    KeyCode::Enter => {
                        result = Some(true);
                        break;
                    }
                    KeyCode::Esc => {
                        result = Some(false);
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        result = Some(false);
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
