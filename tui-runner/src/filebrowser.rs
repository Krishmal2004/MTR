use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(PartialEq)]
enum InputMode {
    Normal,
    CreatingFolder,
}

struct FileBrowser {
    current_dir: PathBuf,
    entries: Vec<Entry>,
    list_state: ListState,
    mode: InputMode,
    input_buf: String,
    error_msg: Option<String>,
}

struct Entry {
    name: String,
    is_dir: bool,
    is_parent: bool,
}

impl FileBrowser {
    fn new(start: &Path) -> Self {
        let mut browser = Self {
            current_dir: start.to_path_buf(),
            entries: vec![],
            list_state: ListState::default(),
            mode: InputMode::Normal,
            input_buf: String::new(),
            error_msg: None,
        };
        browser.refresh();
        browser
    }

    fn refresh(&mut self) {
        self.entries.clear();

        // Add parent ".." entry if we are not at root
        if self.current_dir.parent().is_some() {
            self.entries.push(Entry {
                name: "../".to_string(),
                is_dir: true,
                is_parent: true,
            });
        }

        // Read directory contents, show dirs first then files
        let mut dirs: Vec<Entry> = vec![];
        let mut files: Vec<Entry> = vec![];

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut all: Vec<_> = read_dir.flatten().collect();
            all.sort_by_key(|e| e.file_name());
            for entry in all {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    dirs.push(Entry { name: format!("{}/", name), is_dir: true, is_parent: false });
                } else {
                    files.push(Entry { name, is_dir: false, is_parent: false });
                }
            }
        }

        self.entries.extend(dirs);
        self.entries.extend(files);

        // Reset selection to first item
        if self.entries.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn selected_entry(&self) -> Option<&Entry> {
        self.list_state.selected().and_then(|i| self.entries.get(i))
    }

    fn navigate_into_selected(&mut self) {
        if let Some(entry) = self.selected_entry() {
            if entry.is_dir {
                let new_path = if entry.is_parent {
                    self.current_dir.parent().unwrap().to_path_buf()
                } else {
                    // Strip trailing slash added for display
                    let name = entry.name.trim_end_matches('/');
                    self.current_dir.join(name)
                };
                self.current_dir = new_path;
                self.refresh();
                self.error_msg = None;
            }
        }
    }

    fn create_folder(&mut self) {
        let name = self.input_buf.trim().to_string();
        if name.is_empty() {
            self.error_msg = Some("Folder name cannot be empty.".to_string());
            return;
        }
        let new_path = self.current_dir.join(&name);
        match fs::create_dir_all(&new_path) {
            Ok(_) => {
                self.current_dir = new_path;
                self.input_buf.clear();
                self.mode = InputMode::Normal;
                self.error_msg = None;
                self.refresh();
            }
            Err(e) => {
                self.error_msg = Some(format!("Error: {}", e));
                self.input_buf.clear();
                self.mode = InputMode::Normal;
            }
        }
    }

    fn move_up(&mut self) {
        if self.entries.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        let new_i = if i == 0 { self.entries.len() - 1 } else { i - 1 };
        self.list_state.select(Some(new_i));
    }

    fn move_down(&mut self) {
        if self.entries.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        let new_i = (i + 1) % self.entries.len();
        self.list_state.select(Some(new_i));
    }
}

/// Launch the interactive folder browser. Returns `Some(path)` when the user
/// confirms a directory, or `None` if they cancel (Esc / Ctrl+C).
pub fn browse_for_directory(start: &Path) -> Option<PathBuf> {
    enable_raw_mode().ok()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).ok()?;

    let mut browser = FileBrowser::new(start);
    let result: Option<PathBuf>;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // title
                    Constraint::Min(1),     // file list
                    Constraint::Length(3),  // input / status bar
                    Constraint::Length(1),  // key hint bar
                ])
                .split(size);

            // ── Title ──
            let current_dir_str = browser.current_dir.to_string_lossy().to_string();
            let title = Paragraph::new(Line::from(vec![
                Span::styled(" 📁 SELECT WORKING DIRECTORY ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(
                    current_dir_str,
                    Style::default().fg(Color::Yellow),
                ),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(title, chunks[0]);


            // ── File list ──
            let items: Vec<ListItem> = browser.entries.iter().map(|e| {
                let icon = if e.is_dir { "📁 " } else { "📄 " };
                let style = if e.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", icon, e.name),
                    style,
                )))
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Files & Folders ").border_style(Style::default().fg(Color::Cyan)))
                .highlight_style(
                    Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, chunks[1], &mut browser.list_state);

            // ── Input / status bar ──
            let status_text = if browser.mode == InputMode::CreatingFolder {
                format!(" New folder name: {}█", browser.input_buf)
            } else if let Some(ref err) = browser.error_msg {
                format!(" ⚠  {}", err)
            } else {
                let selected_path = browser.list_state.selected()
                    .and_then(|i| browser.entries.get(i))
                    .map(|e| {
                        if e.is_parent {
                            browser.current_dir.parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default()
                        } else {
                            let name = e.name.trim_end_matches('/');
                            browser.current_dir.join(name).to_string_lossy().to_string()
                        }
                    })
                    .unwrap_or_else(|| browser.current_dir.to_string_lossy().to_string());
                format!(" Selected: {}", selected_path)
            };

            let status_style = if browser.mode == InputMode::CreatingFolder {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else if browser.error_msg.is_some() {
                Style::default().fg(Color::Black).bg(Color::Red)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };

            let status_par = Paragraph::new(status_text)
                .style(status_style)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(status_par, chunks[2]);

            // ── Key hint bar ──
            let hint = if browser.mode == InputMode::CreatingFolder {
                " Enter=create folder   Esc=cancel"
            } else {
                " ↑/↓=navigate   Enter=open folder   Space=use this dir   n=new folder   Esc=cancel"
            };
            let hint_par = Paragraph::new(hint)
                .style(Style::default().fg(Color::White).bg(Color::Blue));
            f.render_widget(hint_par, chunks[3]);
        }).ok();

        if let Ok(true) = event::poll(std::time::Duration::from_millis(80)) {
            if let Ok(Event::Key(key)) = event::read() {
                match browser.mode {
                    InputMode::CreatingFolder => match key.code {
                        KeyCode::Enter => browser.create_folder(),
                        KeyCode::Esc => {
                            browser.mode = InputMode::Normal;
                            browser.input_buf.clear();
                            browser.error_msg = None;
                        }
                        KeyCode::Backspace => { browser.input_buf.pop(); }
                        KeyCode::Char(c) => browser.input_buf.push(c),
                        _ => {}
                    },
                    InputMode::Normal => match key.code {
                        KeyCode::Up => browser.move_up(),
                        KeyCode::Down => browser.move_down(),
                        KeyCode::Enter => browser.navigate_into_selected(),
                        KeyCode::Char(' ') => {
                            // Confirm current directory
                            result = Some(browser.current_dir.clone());
                            break;
                        }
                        KeyCode::Char('n') => {
                            browser.mode = InputMode::CreatingFolder;
                            browser.input_buf.clear();
                            browser.error_msg = None;
                        }
                        KeyCode::Esc => {
                            result = None;
                            break;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            result = None;
                            break;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}