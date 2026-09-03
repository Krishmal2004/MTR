use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(PartialEq)]
enum InputMode {
    Normal,
    CreatingFolder,
    ChoosingDrive,
}

struct FileBrowser {
    current_dir: PathBuf,
    entries: Vec<Entry>,
    list_state: ListState,
    mode: InputMode,
    input_buf: String,
    error_msg: Option<String>,
    drives: Vec<String>,
    drive_state: ListState,
}

struct Entry {
    name: String,
    is_dir: bool,
    is_parent: bool,
}

/// Detect available Windows drive letters by probing A..Z
fn list_drives() -> Vec<String> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).exists() {
            drives.push(drive);
        }
    }
    drives
}

impl FileBrowser {
    fn new(start: &Path) -> Self {
        let drives = list_drives();
        let mut browser = Self {
            current_dir: start.to_path_buf(),
            entries: vec![],
            list_state: ListState::default(),
            mode: InputMode::Normal,
            input_buf: String::new(),
            error_msg: None,
            drives,
            drive_state: ListState::default(),
        };
        browser.refresh();
        browser
    }

    /// Returns true only when there is a real parent directory to go up to.
    /// On Windows, C:\ (Prefix + RootDir = 2 components) is treated as root — no back.
    /// On Unix, / (RootDir = 1 component) is treated as root — no back.
    fn can_go_back(&self) -> bool {
        use std::path::Component;
        let count = self.current_dir.components().count();
        let comps: Vec<_> = self.current_dir.components().collect();
        // Windows drive root: [Prefix("C:"), RootDir]  → count == 2
        // Unix fs root:       [RootDir]                → count == 1
        let has_prefix = comps.first()
            .map(|c| matches!(c, Component::Prefix(_)))
            .unwrap_or(false);
        if has_prefix {
            count > 2  // Windows: need more than Prefix + RootDir
        } else {
            count > 1  // Unix: need more than just RootDir
        }
    }

    fn refresh(&mut self) {
        self.entries.clear();

        // Show the Back entry only when there is a real parent (not at drive root)
        if self.can_go_back() {
            self.entries.push(Entry {
                name: "🔙 Back  (go up one folder)".to_string(),
                is_dir: true,
                is_parent: true,
            });
        }

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
                    // Only go up if we actually can
                    if self.can_go_back() {
                        self.current_dir.parent().unwrap().to_path_buf()
                    } else {
                        self.current_dir.clone()
                    }
                } else {
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
            self.mode = InputMode::Normal;
            self.input_buf.clear();
            return;
        }
        let new_path = self.current_dir.join(&name);
        match fs::create_dir_all(&new_path) {
            Ok(_) => {
                // Stay in the current directory so the new folder is visible in the list
                self.input_buf.clear();
                self.mode = InputMode::Normal;
                self.error_msg = Some(format!("✅ Folder \"{}\" created!", name));
                self.refresh();
                // Auto-select the newly created folder in the list
                if let Some(idx) = self.entries.iter().position(|e| {
                    e.name.trim_end_matches('/') == name
                }) {
                    self.list_state.select(Some(idx));
                }
            }
            Err(e) => {
                self.error_msg = Some(format!("❌ Error: {}", e));
                self.input_buf.clear();
                self.mode = InputMode::Normal;
            }
        }

    }

    fn switch_to_drive(&mut self, drive: String) {
        let path = PathBuf::from(&drive);
        if path.exists() {
            self.current_dir = path;
            self.refresh();
            self.error_msg = None;
        } else {
            self.error_msg = Some(format!("Drive {} is not accessible.", drive));
        }
        self.mode = InputMode::Normal;
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

    fn drive_move_up(&mut self) {
        if self.drives.is_empty() { return; }
        let i = self.drive_state.selected().unwrap_or(0);
        let new_i = if i == 0 { self.drives.len() - 1 } else { i - 1 };
        self.drive_state.select(Some(new_i));
    }

    fn drive_move_down(&mut self) {
        if self.drives.is_empty() { return; }
        let i = self.drive_state.selected().unwrap_or(0);
        let new_i = (i + 1) % self.drives.len();
        self.drive_state.select(Some(new_i));
    }
}

pub fn browse_for_directory(start: &Path) -> Option<PathBuf> {
    enable_raw_mode().ok()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).ok()?;

    let mut browser = FileBrowser::new(start);
    // Pre-select the current drive in the drive picker
    let current_drive = start.to_string_lossy()
        .chars().take(3).collect::<String>()
        .to_uppercase();
    if let Some(idx) = browser.drives.iter().position(|d| d.to_uppercase() == current_drive) {
        browser.drive_state.select(Some(idx));
    } else {
        browser.drive_state.select(Some(0));
    }

    let result: Option<PathBuf>;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // title bar
                    Constraint::Min(1),     // file list
                    Constraint::Length(3),  // status / input
                    Constraint::Length(1),  // key hints
                ])
                .split(size);

            // ── Title bar ──
            let current_dir_str = browser.current_dir.to_string_lossy().to_string();
            let title = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 📁 SELECT WORKING DIRECTORY ",
                    Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(current_dir_str, Style::default().fg(Color::Yellow)),
            ]))
            .block(Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(title, chunks[0]);

            // ── File list ──
            let items: Vec<ListItem> = browser.entries.iter().map(|e| {
                let icon = if e.is_dir { "📁 " } else { "📄 " };
                let style = if e.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(Line::from(Span::styled(format!("{}{}", icon, e.name), style)))
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL)
                    .title(" Files & Folders ")
                    .border_style(Style::default().fg(Color::Cyan)))
                .highlight_style(
                    Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, chunks[1], &mut browser.list_state);

            // ── Status / input bar ──
            let (status_text, status_style) = if browser.mode == InputMode::CreatingFolder {
                (
                    format!(" 📝 New folder name: {}_", browser.input_buf),
                    Style::default().fg(Color::Black).bg(Color::Green),
                )
            } else if let Some(ref err) = browser.error_msg {
                (
                    format!(" ⚠  {}", err),
                    Style::default().fg(Color::White).bg(Color::Red),
                )
            } else {
                let selected_path = browser.list_state.selected()
                    .and_then(|i| browser.entries.get(i))
                    .map(|e| {
                        if e.is_parent {
                            browser.current_dir.parent()
                                .filter(|p| *p != Path::new(""))
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| browser.current_dir.to_string_lossy().to_string())
                        } else {
                            let name = e.name.trim_end_matches('/');
                            browser.current_dir.join(name).to_string_lossy().to_string()
                        }
                    })
                    .unwrap_or_else(|| browser.current_dir.to_string_lossy().to_string());
                (
                    format!(" 📌 Path: {}", selected_path),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                )
            };

            let status_par = Paragraph::new(status_text)
                .style(status_style)
                .block(Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(status_par, chunks[2]);

            // ── Key hint bar ──
            let hint = match browser.mode {
                InputMode::CreatingFolder =>
                    " Enter=create   Esc=cancel",
                InputMode::ChoosingDrive =>
                    " ↑/↓=select drive   Enter=switch   Esc=cancel",
                InputMode::Normal =>
                    " ↑/↓=move   Enter=open   Backspace=back   Space=confirm   n=new folder   d=drive   Esc=quit",
            };
            let hint_par = Paragraph::new(hint)
                .style(Style::default().fg(Color::White).bg(Color::Blue));
            f.render_widget(hint_par, chunks[3]);

            // ── Drive picker popup (overlay) ──
            if browser.mode == InputMode::ChoosingDrive && !browser.drives.is_empty() {
                let popup_width = 20u16;
                let popup_height = (browser.drives.len() as u16 + 2).min(size.height.saturating_sub(4));
                let popup_x = size.width.saturating_sub(popup_width + 2);
                let popup_y = 3u16;
                let popup_area = ratatui::layout::Rect {
                    x: popup_x,
                    y: popup_y,
                    width: popup_width,
                    height: popup_height,
                };

                f.render_widget(Clear, popup_area);

                let drive_items: Vec<ListItem> = browser.drives.iter().map(|d| {
                    ListItem::new(Line::from(Span::styled(
                        format!("  💾 {}", d),
                        Style::default().fg(Color::Yellow),
                    )))
                }).collect();

                let drive_list = List::new(drive_items)
                    .block(Block::default().borders(Borders::ALL)
                        .title(" Drives ")
                        .border_style(Style::default().fg(Color::Yellow)))
                    .highlight_style(
                        Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)
                    )
                    .highlight_symbol("▶ ");

                f.render_stateful_widget(drive_list, popup_area, &mut browser.drive_state);
            }
        }).ok();

        if let Ok(true) = event::poll(std::time::Duration::from_millis(80)) {
            if let Ok(Event::Key(key)) = event::read() {
                // ── FIX: Only handle key-press events, ignore key-release ──
                // On Windows, crossterm fires both Press and Release events,
                // which caused every character to appear twice.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match browser.mode {
                    // ── Folder creation input ──
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

                    // ── Drive chooser popup ──
                    InputMode::ChoosingDrive => match key.code {
                        KeyCode::Up => browser.drive_move_up(),
                        KeyCode::Down => browser.drive_move_down(),
                        KeyCode::Enter => {
                            if let Some(idx) = browser.drive_state.selected() {
                                if let Some(drive) = browser.drives.get(idx).cloned() {
                                    browser.switch_to_drive(drive);
                                }
                            }
                        }
                        KeyCode::Esc => {
                            browser.mode = InputMode::Normal;
                        }
                        _ => {}
                    },

                    // ── Normal navigation ──
                    InputMode::Normal => match key.code {
                        KeyCode::Up => browser.move_up(),
                        KeyCode::Down => browser.move_down(),
                        KeyCode::Enter => browser.navigate_into_selected(),
                        // Backspace = go up one folder, but NEVER past drive root
                        KeyCode::Backspace => {
                            if browser.can_go_back() {
                                browser.current_dir =
                                    browser.current_dir.parent().unwrap().to_path_buf();
                                browser.refresh();
                                browser.error_msg = None;
                            }
                        }
                        KeyCode::Char(' ') => {
                            result = Some(browser.current_dir.clone());
                            break;
                        }
                        KeyCode::Char('n') => {
                            browser.mode = InputMode::CreatingFolder;
                            browser.input_buf.clear();
                            browser.error_msg = None;
                        }
                        KeyCode::Char('d') => {
                            browser.mode = InputMode::ChoosingDrive;
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