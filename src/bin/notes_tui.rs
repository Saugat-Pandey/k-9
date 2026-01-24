use crossterm::{
    event::{ self, Event, KeyCode, KeyEventKind },
    execute,
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{ Constraint, Direction, Layout },
    widgets::{ Block, Borders, Paragraph },
    Terminal,
};
use std::{ env, io, fs, process::Command };
use kv_store::notes::NoteStore;

#[derive(Copy, Clone)]
enum SortMode {
    Id,
    Title,
    Updated,
}

struct AppState {
    selected: usize,
    search: String,
    in_search: bool,
    in_new: bool,
    new_title: String,
    error: Option<String>,
    confirm_delete: bool,
    delete_id: Option<u64>,
    sort_mode: SortMode,
    sort_desc: bool,
    show_favorites_only: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = env
        ::args()
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
    os_hint: Option<String>
) -> Result<(), Box<dyn std::error::Error>>
    where <B as ratatui::backend::Backend>::Error: 'static
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

        sort_mode: SortMode::Updated,
        sort_desc: true,

        show_favorites_only: false,
    };

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
                .split(f.area());

            let main_block = Block::default().title("K-9 Notes").borders(Borders::ALL);
            let main_area = main_block.inner(chunks[0]);
            f.render_widget(main_block, chunks[0]);

            let main_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(main_area);

            let mut filtered = if state.search.is_empty() {
                metas.clone()
            } else {
                let search_lower = state.search.to_lowercase();
                metas
                    .iter()
                    .filter(|m| {
                        m.title.to_lowercase().contains(&search_lower) ||
                            m.tags.iter().any(|t| t.to_lowercase().contains(&search_lower))
                    })
                    .cloned()
                    .collect()
            };

            if state.show_favorites_only {
                filtered.retain(|m| m.favorite);
            }

            match state.sort_mode {
                SortMode::Id => filtered.sort_by_key(|m| m.id),
                SortMode::Title => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
                SortMode::Updated => filtered.sort_by_key(|m| m.updated_at),
            }

            if state.sort_desc {
                filtered.reverse();
            }

            if state.selected >= filtered.len() && !filtered.is_empty() {
                state.selected = filtered.len() - 1;
            }

            let list_text = if !filtered.is_empty() {
                filtered
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let marker = if i == state.selected { ">" } else { " " };
                        let star = if m.favorite { "★" } else { " " };
                        format!("{}{} {}  {}", marker, star, m.id, m.title)
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
                    Ok(Some(note)) => {
                        let mut text = format!("{}\n\n{}", note.title, note.body);

                        if note.image.is_some() {
                            text.push_str("\n\n[📷 Image attached]");
                        }

                        text
                    }
                    Ok(None) => "Note not found".to_string(),
                    Err(err) => format!("Error: {}", err),
                }
            } else if !state.search.is_empty() {
                "No matching notes".to_string()
            } else {
                "No notes".to_string()
            };

            let list_widget = Paragraph::new(list_text).block(
                Block::default().title("Notes").borders(Borders::ALL)
            );
            f.render_widget(list_widget, main_split[0]);

            let preview_widget = Paragraph::new(preview_text).block(
                Block::default().title("Preview").borders(Borders::ALL)
            );
            f.render_widget(preview_widget, main_split[1]);

            // Render confirmation popup if needed
            if state.confirm_delete {
                if let Some(id) = state.delete_id {
                    let popup_text = format!("Delete note {}? (y/n)", id);
                    let popup_width = (popup_text.len() as u16) + 4;
                    let popup_height = 3;

                    let popup_area = ratatui::layout::Rect {
                        x: chunks[0].width.saturating_sub(popup_width) / 2,
                        y: chunks[0].height.saturating_sub(popup_height) / 2,
                        width: popup_width,
                        height: popup_height,
                    };

                    let popup_widget = Paragraph::new(popup_text).block(
                        Block::default().title("Confirm").borders(Borders::ALL)
                    );
                    f.render_widget(popup_widget, popup_area);
                }
            }

            let sort_label = match state.sort_mode {
                SortMode::Id => "ID",
                SortMode::Title => "Title",
                SortMode::Updated => "Updated",
            };

            let fav_flag = if state.show_favorites_only { "ON" } else { "OFF" };
            let sort_dir = if state.sort_desc { "↓" } else { "↑" };

            let status_text = if state.in_new {
                format!("New title: {} (Enter=save, Esc=cancel)", state.new_title)
            } else if state.in_search {
                format!("Search: {}", state.search)
            } else if state.confirm_delete {
                "Confirm deletion: y=yes, n/Esc=cancel".to_string()
            } else {
                format!(
                    "File: {} | q: quit | /: search | n: new | d: delete | e: edit | a: attach image (picker) | i: open image | f: favorite | F: fav-only({}) | s: sort | Sort: {} {}",
                    file_path,
                    fav_flag,
                    sort_label,
                    sort_dir
                )
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
                                                        if
                                                            !metas.is_empty() &&
                                                            state.selected >= metas.len()
                                                        {
                                                            state.selected = metas.len() - 1;
                                                        } else if metas.is_empty() {
                                                            state.selected = 0;
                                                        }
                                                        state.error = None;
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(
                                                            format!("Failed to reload: {}", e)
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                state.error = Some(
                                                    format!("Failed to save: {}", e)
                                                );
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
                                                        state.error = Some(
                                                            format!("Failed to reload: {}", e)
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                state.error = Some(
                                                    format!("Failed to save: {}", e)
                                                );
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
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
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
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
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
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
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
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
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
                                        if
                                            let Err(e) = edit_note_in_editor(
                                                &mut note,
                                                os_hint.as_deref()
                                            )
                                        {
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
                                                                    state.error = Some(
                                                                        format!("Failed to reload: {}", e)
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            state.error = Some(
                                                                format!("Failed to save: {}", e)
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    state.error = Some(
                                                        format!("Failed to update note: {}", e)
                                                    );
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

                            state.error = None;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let filtered_len = if state.search.is_empty() {
                                metas.len()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .count()
                            };
                            if state.selected + 1 < filtered_len {
                                state.selected += 1;
                                state.error = None;
                            }
                        }
                        KeyCode::Char('i') => {
                            // Get filtered notes (same logic as delete/edit)
                            let filtered: Vec<_> = if state.search.is_empty() {
                                metas.clone()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .cloned()
                                    .collect()
                            };

                            if !filtered.is_empty() && state.selected < filtered.len() {
                                let note_id = filtered[state.selected].id;

                                match store.get(note_id) {
                                    Ok(Some(note)) => {
                                        if note.image.is_none() {
                                            state.error = Some(
                                                "No image attached to this note".to_string()
                                            );
                                        } else if let Err(e) = open_note_image(&note) {
                                            state.error = Some(e);
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
                        KeyCode::Char('a') => {
                            // build filtered list in the same way as the draw logic
                            let mut filtered: Vec<_> = if state.search.is_empty() {
                                metas.clone()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .cloned()
                                    .collect()
                            };

                            if state.show_favorites_only {
                                filtered.retain(|m| m.favorite);
                            }

                            if filtered.is_empty() {
                                state.error = Some("No note selected".to_string());
                                continue;
                            }

                            if state.selected >= filtered.len() {
                                state.selected = filtered.len() - 1;
                            }

                            let note_id = filtered[state.selected].id;

                            // Temporarily leave raw mode while OS UI is active
                            if let Err(e) = disable_raw_mode() {
                                state.error = Some(format!("Failed to disable raw mode: {}", e));
                                continue;
                            }

                            let picked = pick_image_file();

                            if let Err(e) = enable_raw_mode() {
                                state.error = Some(format!("Failed to re-enable raw mode: {}", e));
                                continue;
                            }

                            match picked {
                                Ok(path) => {
                                    match store.attach_image(note_id, &path) {
                                        Ok(_) => {
                                            if let Err(e) = store.save(file_path) {
                                                state.error = Some(
                                                    format!("Failed to save: {}", e)
                                                );
                                            } else {
                                                match store.list_meta() {
                                                    Ok(new_metas) => {
                                                        metas = new_metas;
                                                        state.error = None;
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(
                                                            format!("Failed to reload: {}", e)
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            state.error = Some(
                                                format!("Failed to attach image: {}", e)
                                            );
                                        }
                                    }
                                }
                                Err(msg) => {
                                    // cancelled / tool missing / unsupported
                                    state.error = Some(msg);
                                }
                            }
                        }
                        KeyCode::Char('s') => {
                            state.sort_mode = match state.sort_mode {
                                SortMode::Id => SortMode::Title,
                                SortMode::Title => SortMode::Updated,
                                SortMode::Updated => SortMode::Id,
                            };
                        }
                        KeyCode::Char('S') => {
                            state.sort_desc = !state.sort_desc;
                        }
                        KeyCode::Char('f') => {
                            let filtered: Vec<_> = if state.search.is_empty() {
                                metas.clone()
                            } else {
                                let search_lower = state.search.to_lowercase();
                                metas
                                    .iter()
                                    .filter(|m| {
                                        m.title.to_lowercase().contains(&search_lower) ||
                                            m.tags
                                                .iter()
                                                .any(|t| t.to_lowercase().contains(&search_lower))
                                    })
                                    .cloned()
                                    .collect()
                            };

                            // Apply favorites-only view too (must match draw logic)
                            let mut filtered = filtered;
                            if state.show_favorites_only {
                                filtered.retain(|m| m.favorite);
                            }

                            if !filtered.is_empty() && state.selected < filtered.len() {
                                let note_id = filtered[state.selected].id;

                                if let Err(e) = store.toggle_favorite(note_id) {
                                    state.error = Some(format!("Failed to toggle favorite: {}", e));
                                } else if let Err(e) = store.save(file_path) {
                                    state.error = Some(format!("Failed to save: {}", e));
                                } else {
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
                            }
                        }
                        KeyCode::Char('F') => {
                            state.show_favorites_only = !state.show_favorites_only;
                            state.selected = 0;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
fn edit_note_in_editor(
    note: &mut kv_store::notes::Note,
    os_hint: Option<&str>
) -> Result<(), String> {
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
    fs::write(&temp_file, &content).map_err(|e| format!("Failed to write temp file: {}", e))?;

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
    let edited_content = fs
        ::read_to_string(&temp_file)
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

fn open_note_image(note: &kv_store::notes::Note) -> Result<(), String> {
    let bytes = match &note.image {
        Some(b) => b,
        None => {
            return Err("No image attached to this note".to_string());
        }
    };

    let mut path = std::env::temp_dir();
    path.push(format!("k9_note_image_{}.png", note.id));

    std::fs::write(&path, bytes).map_err(|e| format!("Failed to write temp image: {}", e))?;

    std::process::Command
        ::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open image: {}", e))?;

    Ok(())
}

fn pick_image_file() -> Result<String, String> {
    let output = (
        if cfg!(target_os = "macos") {
            Command::new("osascript")
                .args(["-e", r#"POSIX path of (choose file with prompt "Select an image")"#])
                .output()
        } else if cfg!(target_os = "linux") {
            // Uses zenity (common on desktop Linux)
            Command::new("zenity").args(["--file-selection", "--title=Select an image"]).output()
        } else if cfg!(target_os = "windows") {
            // Uses PowerShell + OpenFileDialog
            Command::new("powershell")
                .args([
                    "-Command",
                    "Add-Type -AssemblyName System.Windows.Forms; \
                 $f = New-Object System.Windows.Forms.OpenFileDialog; \
                 $f.Filter = 'Images|*.png;*.jpg;*.jpeg;*.gif;*.bmp;*.webp|All files|*.*'; \
                 if ($f.ShowDialog() -eq 'OK') { $f.FileName }",
                ])
                .output()
        } else {
            return Err("Unsupported OS".to_string());
        }
    ).map_err(|e| format!("Failed to open file picker: {}", e))?;

    if !output.status.success() {
        return Err("File picker cancelled".to_string());
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        Err("No file selected".to_string())
    } else {
        Ok(path)
    }
}
