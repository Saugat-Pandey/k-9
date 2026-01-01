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
use std::{env, io};
use kv_store::notes::NoteStore;

struct AppState {
    selected: usize,
    search: String,
    in_search: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "notes.db".to_string());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal, &file_path);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let store_result = NoteStore::open(file_path);
    let metas_result = match &store_result {
        Ok(store) => store.list_meta(),
        Err(e) => Err(kv_store::KvError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("{}", e),
        ))),
    };
    
    let mut state = AppState {
        selected: 0,
        search: String::new(),
        in_search: false,
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

            let filtered = match &metas_result {
                Ok(metas) => {
                    if state.search.is_empty() {
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
                    }
                }
                Err(_) => vec![],
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

            let preview_text = match (&store_result, &metas_result) {
                (Err(err), _) => format!("Error: {}", err),
                (_, Err(err)) => format!("Error: {}", err),
                (Ok(store), Ok(_)) if !filtered.is_empty() => {
                    let meta = &filtered[state.selected];
                    match store.get(meta.id) {
                        Ok(Some(note)) => format!("{}\n\n{}", note.title, note.body),
                        Ok(None) => "Note not found".to_string(),
                        Err(err) => format!("Error: {}", err),
                    }
                }
                _ if !state.search.is_empty() => "No matching notes".to_string(),
                _ => "No notes".to_string(),
            };

            let list_widget = Paragraph::new(list_text)
                .block(Block::default().title("Notes").borders(Borders::ALL));
            f.render_widget(list_widget, main_split[0]);

            let preview_widget = Paragraph::new(preview_text)
                .block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(preview_widget, main_split[1]);

            let status_text = if state.in_search {
                format!("Search: {}", state.search)
            } else {
                format!("File: {} | q: quit | /: search", file_path)
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
                
                if state.in_search {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            state.in_search = false;
                            // Clamp selected to filtered length
                            if let Ok(metas) = &metas_result {
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
                        KeyCode::Char('/') => {
                            state.in_search = true;
                            state.search.clear();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected > 0 {
                                state.selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let Ok(metas) = &metas_result {
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
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
