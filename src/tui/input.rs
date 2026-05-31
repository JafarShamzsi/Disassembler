use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::io;

use super::app::{App, Tab};
use super::views::ui;
use crate::graph_view::NavigationDirection;

pub(crate) fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(16); // 60 FPS limit

    loop {
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        let should_draw = last_tick.elapsed() >= tick_rate;

        // Only draw if enough time has passed
        if should_draw {
            terminal.draw(|f| ui(f, &mut app))?;
            last_tick = std::time::Instant::now();
        }

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle key events...
                    if app.address_jump_mode {
                        match key.code {
                            KeyCode::Esc => app.exit_address_jump_mode(),
                            KeyCode::Enter => app.jump_to_address_query(),
                            KeyCode::Backspace => {
                                app.address_jump_query.pop();
                            }
                            KeyCode::Char(c) if c.is_ascii_hexdigit() || matches!(c, 'x' | 'X') => {
                                app.address_jump_query.push(c);
                            }
                            _ => {}
                        }
                    } else if app.search_mode {
                        match key.code {
                            KeyCode::Esc => app.exit_search_mode(),
                            KeyCode::Enter => app.exit_search_mode(),
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.apply_filter();
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.apply_filter();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Handle Ctrl+C gracefully
                                return Ok(());
                            }
                            KeyCode::Esc => return Ok(()),
                            KeyCode::Char('h') | KeyCode::F(1) => app.toggle_help(),
                            KeyCode::Char('/') => app.enter_search_mode(),
                            KeyCode::Char('g') => app.enter_address_jump_mode(),
                            KeyCode::Char('u') => app.go_back(),
                            KeyCode::Char('r') => app.go_forward(),
                            KeyCode::Char('n') => app.next_search_match(),
                            KeyCode::Char('N') => app.previous_search_match(),
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view
                                            .move_selection(cfg, NavigationDirection::Down);
                                    }
                                } else if app.current_tab == Tab::Functions {
                                    app.next_function();
                                } else if app.current_tab == Tab::Names {
                                    app.next_name();
                                } else if app.current_tab == Tab::Xrefs {
                                    app.next_xref();
                                } else {
                                    app.next_instruction();
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view.move_selection(cfg, NavigationDirection::Up);
                                    }
                                } else if app.current_tab == Tab::Functions {
                                    app.previous_function();
                                } else if app.current_tab == Tab::Names {
                                    app.previous_name();
                                } else if app.current_tab == Tab::Xrefs {
                                    app.previous_xref();
                                } else {
                                    app.previous_instruction();
                                }
                            }
                            KeyCode::Enter if app.current_tab == Tab::Functions => {
                                app.jump_to_selected_function();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Names => {
                                app.jump_to_selected_name();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Xrefs => {
                                app.jump_to_selected_xref();
                            }
                            KeyCode::Left => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view
                                            .move_selection(cfg, NavigationDirection::Left);
                                    }
                                }
                            }
                            KeyCode::Right => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view
                                            .move_selection(cfg, NavigationDirection::Right);
                                    }
                                }
                            }
                            KeyCode::Tab => app.next_tab(),
                            KeyCode::BackTab => app.previous_tab(),
                            KeyCode::Char('1') => app.current_tab = Tab::Instructions,
                            KeyCode::Char('2') => app.current_tab = Tab::Functions,
                            KeyCode::Char('3') => app.current_tab = Tab::Names,
                            KeyCode::Char('4') => app.current_tab = Tab::Xrefs,
                            KeyCode::Char('5') => app.current_tab = Tab::ControlFlow,
                            KeyCode::Char('6') => app.current_tab = Tab::GraphView,
                            KeyCode::Char('7') => app.current_tab = Tab::HexDump,
                            KeyCode::PageDown => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.zoom_out();
                                } else {
                                    for _ in 0..10 {
                                        app.next_instruction();
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.zoom_in();
                                } else {
                                    for _ in 0..10 {
                                        app.previous_instruction();
                                    }
                                }
                            }
                            KeyCode::Char('w') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.pan(0.0, -5.0);
                                }
                            }
                            KeyCode::Char('s') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.pan(0.0, 5.0);
                                }
                            }
                            KeyCode::Char('a') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.pan(-5.0, 0.0);
                                }
                            }
                            KeyCode::Char('d') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.pan(5.0, 0.0);
                                }
                            }
                            KeyCode::Char('c') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.center_on_selected();
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                if app.current_tab == Tab::GraphView {
                                    app.graph_view.zoom_in();
                                }
                            }
                            KeyCode::Char('-') if app.current_tab == Tab::GraphView => {
                                app.graph_view.zoom_out();
                            }
                            _ => {}
                        }
                    }

                    // Force immediate redraw after user input
                    terminal.draw(|f| ui(f, &mut app))?;
                    last_tick = std::time::Instant::now();
                }
            }
        }
    }
}
