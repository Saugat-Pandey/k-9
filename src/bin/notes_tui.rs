use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{env, io, fs, process::Command};
use kv_store::notes::NoteStore;

struct AppState {
    selected: usize,
    search: String,
    in_search: bool,
    in_new: bool,
    new_title: String,
    error: Option<String>,
    confirm_delete: bool,
    delete_id: Option<u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "notes.db".to_string());

    let os_hint = env::args().nth(2);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal, &file_path, os_hint);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    file_path: &str,
    os_hint: Option<String>,
) -> Result<(), Box<dyn std::error::Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let mut store = NoteStore::open(file_path)?;
    let mut metas = store.list_meta()?;
    
    let mut state = AppState {
        selected: 0,
        search: String::new(),
        in_search: false,
        in_new: false,
        new_title: String::new(),
        error: None,
        confirm_delete: false,
        delete_id: None,
    };

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
                .split(f.area());

            let main_block = Block::default()
                .title("K-9 Notes")
                .borders(Borders::ALL);
            let main_area = main_block.inner(chunks[0]);
            f.render_widget(main_block, chunks[0]);

            let main_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(main_area);

            let filtered = if state.search.is_empty() {
                metas.clone()
            } else {
                let search_lower = state.search.to_lowercase();
                metas
                    .iter()
                    .filter(|m| {
                        m.title.to_lowercase().contains(&search_lower)
                            || m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                    })
                    .cloned()
                    .collect()
            };

            let list_text = if !filtered.is_empty() {
                filtered
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let marker = if i == state.selected { ">" } else { " " };
                        format!("{} {}  {}", marker, m.id, m.title)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else if state.search.is_empty() {
                "No notes".to_string()
            } else {
                "No matching notes".to_string()
            };

            let preview_text = if let Some(ref err) = state.error {
                format!("Error: {}", err)
            } else if !filtered.is_empty() {
                let meta = &filtered[state.selected];
                match store.get(meta.id) {
                    Ok(Some(note)) => format!("{}\n\n{}", note.title, note.body),
                    Ok(None) => "Note not found".to_string(),
                    Err(err) => format!("Error: {}", err),
                }
            } else if !state.search.is_empty() {
                "No matching notes".to_string()
            } else {
                "No notes".to_string()
            };

            let list_widget = Paragraph::new(list_text)
                .block(Block::default().title("Notes").borders(Borders::ALL));
            f.render_widget(list_widget, main_split[0]);

            let preview_widget = Paragraph::new(preview_text)
                .block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(preview_widget, main_split[1]);

            // Render confirmation popup if needed
            if state.confirm_delete {
                if let Some(id) = state.delete_id {
                    let popup_text = format!("Delete note {}? (y/n)", id);
                    let popup_width = popup_text.len() as u16 + 4;
                    let popup_height = 3;
                    
                    let popup_area = ratatui::layout::Rect {
                        x: (chunks[0].width.saturating_sub(popup_width)) / 2,
                        y: (chunks[0].height.saturating_sub(popup_height)) / 2,
                        width: popup_width,
                        height: popup_height,
                    };
                    
                    let popup_widget = Paragraph::new(popup_text)
                        .block(Block::default().title("Confirm").borders(Borders::ALL));
                    f.render_widget(popup_widget, popup_area);
                }
            }

            let status_text = if state.in_new {
                format!("New title: {} (Enter=save, Esc=cancel)", state.new_title)
            } else if state.in_search {
                format!("Search: {}", state.search)
            } else if state.confirm_delete {
                "Confirm deletion: y=yes, n/Esc=cancel".to_string()
            } else {
                format!("File: {} | q: quit | /: search | n: new | d: delete | e: edit", file_path)
            };
            let status = Paragraph::new(status_text);
            f.render_widget(status, chunks[1]);
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events, ignore key release events
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                
                if state.confirm_delete {
                    match key.code {
                        KeyCode::Char('y') => {
                            if let Some(id) = state.delete_id {
                                match store.delete(id) {
                                    Ok(_) => {
                                        match store.save(file_path) {
                                            Ok(_) => {
                                                match store.list_meta() {
                                                    Ok(new_metas) => {
                                                        metas = new_metas;
                                                        // Clamp selected index
                                                        if !metas.is_empty() && state.selected >= metas.len() {
                                                            state.selected = metas.len() - 1;
                                                        } else if metas.is_empty() {
                                                            state.selected = 0;
                                                        }
                                                        state.error = None;
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(format!("Failed to reload: {}", e));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                state.error = Some(format!("Failed to save: {}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        state.error = Some(format!("Failed to delete: {}", e));
                                    }
                                }
                            }
                            state.confirm_delete = false;
                            state.delete_id = None;
                        }
                        KeyCode::Char('n') | KeyCode::Esc => {
                            state.confirm_delete = false;
                            state.delete_id = None;
                        }
                        _ => {}
                    }
                } else if state.in_new {
                    match key.code {
                        KeyCode::Esc => {
                            state.in_new = false;
                            state.error = None;
                        }
                        KeyCode::Backspace => {
                            state.new_title.pop();
                        }
                        KeyCode::Char(c) => {
                            state.new_title.push(c);
                        }
                        KeyCode::Enter => {
                            let title = state.new_title.trim();
                            if !title.is_empty() {
                                match store.create(title.to_string(), String::new()) {
                                    Ok(_id) => {
                                        match store.save(file_path) {
                                            Ok(_) => {
                                                match store.list_meta() {
                                                    Ok(new_metas) => {
                                                        metas = new_metas;
                                                        if !metas.is_empty() {
                                                            state.selected = metas.len() - 1;
                                                        }
                                                        state.in_new = false;
                                                        state.new_title.clear();
                                                        state.error = None;
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(format!("Failed to reload: {}", e));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                state.error = Some(format!("Failed to save: {}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        state.error = Some(format!("Failed to create: {}", e));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } else if state.in_search {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            state.in_search = false;
                            // Clamp selected to filtered length
                            let filtered_len = if state.search.is_empty() {
                                metas.len()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower)
                                            || m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .count()
                            };
                            if filtered_len > 0 && state.selected >= filtered_len {
                                state.selected = filtered_len - 1;
                            }
                        }
                        KeyCode::Backspace => {
                            state.search.pop();
                        }
                        KeyCode::Char(c) => {
                            state.search.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('n') => {
                            state.in_new = true;
                            state.new_title.clear();
                            state.error = None;
                        }
                        KeyCode::Char('/') => {
                            state.in_search = true;
                            state.search.clear();
                        }
                        KeyCode::Char('d') => {
                            // Get the filtered list to find the actual note ID
                            let filtered: Vec<_> = if state.search.is_empty() {
                                metas.clone()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower)
                                            || m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .cloned()
                                    .collect()
                            };
                            
                            if !filtered.is_empty() && state.selected < filtered.len() {
                                let note_id = filtered[state.selected].id;
                                state.confirm_delete = true;
                                state.delete_id = Some(note_id);
                                state.error = None;
                            }
                        }
                        KeyCode::Char('e') => {
                            // Get the filtered list to find the actual note ID
                            let filtered: Vec<_> = if state.search.is_empty() {
                                metas.clone()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower)
                                            || m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .cloned()
                                    .collect()
                            };
                            
                            if !filtered.is_empty() && state.selected < filtered.len() {
                                let note_id = filtered[state.selected].id;
                                
                                // Load note
                                match store.get(note_id) {
                                    Ok(Some(mut note)) => {
                                        // Disable raw mode, edit, then re-enable
                                        if let Err(e) = edit_note_in_editor(&mut note, os_hint.as_deref()) {
                                            state.error = Some(format!("Edit failed: {}", e));
                                        } else {
                                            // Update note in store
                                            match store.update(note) {
                                                Ok(_) => {
                                                    match store.save(file_path) {
                                                        Ok(_) => {
                                                            match store.list_meta() {
                                                                Ok(new_metas) => {
                                                                    metas = new_metas;
                                                                    state.error = None;
                                                                }
                                                                Err(e) => {
                                                                    state.error = Some(format!("Failed to reload: {}", e));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            state.error = Some(format!("Failed to save: {}", e));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    state.error = Some(format!("Failed to update note: {}", e));
                                                }
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        state.error = Some("Note not found".to_string());
                                    }
                                    Err(e) => {
                                        state.error = Some(format!("Failed to load note: {}", e));
                                    }
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected > 0 {
                                state.selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let filtered_len = if state.search.is_empty() {
                                metas.len()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower)
                                            || m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .count()
                            };
                            if state.selected + 1 < filtered_len {
                                state.selected += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
fn edit_note_in_editor(note: &mut kv_store::notes::Note, os_hint: Option<&str>) -> Result<(), String> {
    
    // Get editor from environment or default based on OS hint
    let editor = env::var("EDITOR").unwrap_or_else(|_| {
        let is_linux = os_hint == Some("linux") || (os_hint.is_none() && !cfg!(windows));
        if is_linux {
            "nano".to_string()
        } else {
            "notepad".to_string()
        }
    });
    
    // Create temp file path (platform-independent)
    let mut temp_dir = env::temp_dir();
    temp_dir.push(format!("k9_note_{}.md", note.id));
    let temp_file = temp_dir.to_string_lossy().to_string();
    
    // Write note content to temp file
    let content = format!("Title: {}\n\n{}", note.title, note.body);
    fs::write(&temp_file, &content)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    // Disable raw mode before spawning editor
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
    
    // Open editor
    let status = Command::new(&editor)
        .arg(&temp_file)
        .status()
        .map_err(|e| format!("Failed to start editor: {}", e))?;
    
    if !status.success() {
        enable_raw_mode().map_err(|e| format!("Failed to re-enable raw mode: {}", e))?;
        return Err("Editor exited with error".to_string());
    }
    
    // Re-enable raw mode
    enable_raw_mode().map_err(|e| format!("Failed to re-enable raw mode: {}", e))?;
    
    // Read edited content from temp file
    let edited_content = fs::read_to_string(&temp_file)
        .map_err(|e| format!("Failed to read temp file: {}", e))?;
    
    // Parse the content
    let lines: Vec<&str> = edited_content.split('\n').collect();
    
    if lines.is_empty() {
        return Err("File is empty".to_string());
    }
    
    // Extract title from first line
    let title_line = lines[0];
    if !title_line.starts_with("Title: ") {
        return Err("Invalid format: first line must start with 'Title: '".to_string());
    }
    
    let new_title = title_line[7..].trim().to_string();
    if new_title.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    
    // Extract body (skip "Title: " line and the blank line after it)
    let body_start = if lines.len() > 2 && lines[1].trim().is_empty() {
        2
    } else if lines.len() > 1 {
        1
    } else {
        1
    };
    
    let new_body = lines[body_start..].join("\n").trim_end().to_string();
    
    // Update note
    note.title = new_title;
    note.body = new_body;
    
    // Clean up temp file
    let _ = fs::remove_file(&temp_file);
    
    Ok(())
}