use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, time::Duration};

mod app;
mod ui;
mod config;
mod api;
mod storage;
mod types;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if app.is_processing {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            app.stop_generation();
                        }
                        _ => {}
                    }
                } else if app.is_input_mode() {
                    match key.code {
                        KeyCode::Enter => app.submit_message().await,
                        KeyCode::Esc => app.set_normal_mode(),
                        KeyCode::Char(c) => app.enter_char(c),
                        KeyCode::Backspace => app.delete_char(),
                        _ => {}
                    }
                } else if app.show_backend_selection {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('b') => app.toggle_backend_selection(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_backend(),
                        KeyCode::Down | KeyCode::Char('j') => app.next_backend(),
                        KeyCode::Enter => app.select_backend(),
                        _ => {}
                    }
                } else if app.show_url_edit {
                    match key.code {
                        KeyCode::Esc => app.cancel_url_edit(),
                        KeyCode::Enter => app.confirm_backend_switch(),
                        KeyCode::Char(c) => app.enter_url_char(c),
                        KeyCode::Backspace => app.delete_url_char(),
                        _ => {}
                    }
                } else if app.show_shortcuts {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char(' ') => app.toggle_shortcuts(),
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('i') => app.set_input_mode(),
                        KeyCode::Char('s') => {
                            app.toggle_shortcuts();
                            app.toggle_stats();
                        }
                        KeyCode::Char('c') => {
                            app.clear_history();
                            app.toggle_shortcuts();
                        }
                        KeyCode::Char('b') => {
                            app.toggle_shortcuts();
                            app.toggle_backend_selection();
                        }
                        _ => {}
                    }
                } else if app.show_stats {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('s') => app.toggle_stats(),
                        _ => {}
                    }
                } else if app.show_history {
                    match key.code {
                        KeyCode::Esc => app.toggle_history(),
                        KeyCode::Char(' ') if key.modifiers.is_empty() => {
                            if app.pending_leader_key {
                                app.pending_leader_key = false;
                            } else {
                                app.pending_leader_key = true;
                            }
                        }
                        KeyCode::Char('e') if app.pending_leader_key => {
                            app.pending_leader_key = false;
                            app.toggle_history();
                        }
                        KeyCode::Char('j') | KeyCode::Down => app.history_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.history_up(),
                        KeyCode::Enter => app.load_selected_chat(),
                        KeyCode::Char('n') => app.new_chat(),
                        KeyCode::Char('d') => app.delete_selected_chat(),
                        KeyCode::Char('r') => app.reload_history(),
                        _ => {
                            app.pending_leader_key = false;
                        }
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('i') => app.set_input_mode(),
                        KeyCode::Char('j') => app.scroll_down(),
                        KeyCode::Char('k') => app.scroll_up(),
                        KeyCode::Char('n') => app.new_chat(),
                        KeyCode::Char(' ') if key.modifiers.is_empty() => {
                            if app.pending_leader_key {
                                app.pending_leader_key = false;
                                app.toggle_shortcuts();
                            } else {
                                app.pending_leader_key = true;
                            }
                        }
                        KeyCode::Char('e') if app.pending_leader_key => {
                            app.pending_leader_key = false;
                            app.toggle_history();
                        }
                        KeyCode::Char('s') => app.toggle_stats(),
                        KeyCode::Char('c') => app.clear_history(),
                        KeyCode::Char('b') => app.toggle_backend_selection(),
                        _ => {
                            if app.pending_leader_key {
                                app.pending_leader_key = false;
                            }
                        }
                    }
                }
            }
        }
        
        // Check for async updates (e.g. streaming response)
        app.tick().await;

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }
}
