use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::io;

use super::app::{App, Tab};
use super::views::ui;
use crate::graph_view::NavigationDirection;

pub(crate) fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<App> {
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
                    if app.rename_mode {
                        match key.code {
                            KeyCode::Esc => app.exit_rename_mode(),
                            KeyCode::Enter => app.apply_rename_query(),
                            KeyCode::Backspace => {
                                app.rename_query.pop();
                            }
                            KeyCode::Char(c) if !c.is_control() => {
                                app.rename_query.push(c);
                            }
                            _ => {}
                        }
                    } else if app.comment_mode {
                        match key.code {
                            KeyCode::Esc => app.exit_comment_mode(),
                            KeyCode::Enter => app.apply_comment_query(),
                            KeyCode::Backspace => {
                                app.comment_query.pop();
                            }
                            KeyCode::Char(c) if !c.is_control() => {
                                app.comment_query.push(c);
                            }
                            _ => {}
                        }
                    } else if app.address_jump_mode {
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
                            KeyCode::Char('q') => return Ok(app),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Handle Ctrl+C gracefully
                                return Ok(app);
                            }
                            KeyCode::Esc => return Ok(app),
                            KeyCode::Char('h') | KeyCode::F(1) => app.toggle_help(),
                            KeyCode::Char('/') => app.enter_search_mode(),
                            KeyCode::Char('g') => app.enter_address_jump_mode(),
                            KeyCode::Char('u') => app.go_back(),
                            KeyCode::Char('r') => app.go_forward(),
                            KeyCode::Char('n') => app.next_search_match(),
                            KeyCode::Char('N') => app.previous_search_match(),
                            KeyCode::Char('R') => app.enter_rename_mode(),
                            KeyCode::Char(';') => app.enter_comment_mode(),
                            KeyCode::Char('b') => app.toggle_bookmark_at_selection(),
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view
                                            .move_selection(cfg, NavigationDirection::Down);
                                    }
                                } else if app.current_tab == Tab::Functions {
                                    app.next_function();
                                } else if app.current_tab == Tab::CallGraph {
                                    app.next_call_graph_function();
                                } else if app.current_tab == Tab::Imports {
                                    app.next_import();
                                } else if app.current_tab == Tab::Exports {
                                    app.next_export();
                                } else if app.current_tab == Tab::Symbols {
                                    app.next_symbol();
                                } else if app.current_tab == Tab::Names {
                                    app.next_name();
                                } else if app.current_tab == Tab::Strings {
                                    app.next_string();
                                } else if app.current_tab == Tab::Data {
                                    app.next_data_object();
                                } else if app.current_tab == Tab::Relocations {
                                    app.next_relocation();
                                } else if app.current_tab == Tab::Sections {
                                    app.next_section();
                                } else if app.current_tab == Tab::Xrefs {
                                    app.next_xref();
                                } else if app.current_tab == Tab::Bookmarks {
                                    app.next_bookmark();
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
                                } else if app.current_tab == Tab::CallGraph {
                                    app.previous_call_graph_function();
                                } else if app.current_tab == Tab::Imports {
                                    app.previous_import();
                                } else if app.current_tab == Tab::Exports {
                                    app.previous_export();
                                } else if app.current_tab == Tab::Symbols {
                                    app.previous_symbol();
                                } else if app.current_tab == Tab::Names {
                                    app.previous_name();
                                } else if app.current_tab == Tab::Strings {
                                    app.previous_string();
                                } else if app.current_tab == Tab::Data {
                                    app.previous_data_object();
                                } else if app.current_tab == Tab::Relocations {
                                    app.previous_relocation();
                                } else if app.current_tab == Tab::Sections {
                                    app.previous_section();
                                } else if app.current_tab == Tab::Xrefs {
                                    app.previous_xref();
                                } else if app.current_tab == Tab::Bookmarks {
                                    app.previous_bookmark();
                                } else {
                                    app.previous_instruction();
                                }
                            }
                            KeyCode::Enter if app.current_tab == Tab::Overview => {
                                app.jump_to_entry_point();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Functions => {
                                app.jump_to_selected_function();
                            }
                            KeyCode::Enter if app.current_tab == Tab::CallGraph => {
                                app.jump_to_selected_call_graph_function();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Imports => {
                                app.jump_to_selected_import();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Exports => {
                                app.jump_to_selected_export();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Symbols => {
                                app.jump_to_selected_symbol();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Names => {
                                app.jump_to_selected_name();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Strings => {
                                app.jump_to_selected_string();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Data => {
                                app.jump_to_selected_data_object();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Relocations => {
                                app.jump_to_selected_relocation();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Sections => {
                                app.jump_to_selected_section();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Xrefs => {
                                app.jump_to_selected_xref();
                            }
                            KeyCode::Enter if app.current_tab == Tab::Bookmarks => {
                                app.jump_to_selected_bookmark();
                            }
                            KeyCode::Enter if app.current_tab == Tab::GraphView => {
                                app.jump_to_selected_graph_block();
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
                            KeyCode::Char('O') => app.set_current_tab(Tab::Overview),
                            KeyCode::Char('1') => app.set_current_tab(Tab::Instructions),
                            KeyCode::Char('2') => app.set_current_tab(Tab::Functions),
                            KeyCode::Char('3') => app.set_current_tab(Tab::CallGraph),
                            KeyCode::Char('4') => app.set_current_tab(Tab::Names),
                            KeyCode::Char('I') => app.set_current_tab(Tab::Imports),
                            KeyCode::Char('E') => app.set_current_tab(Tab::Exports),
                            KeyCode::Char('Y') => app.set_current_tab(Tab::Symbols),
                            KeyCode::Char('5') => app.set_current_tab(Tab::Sections),
                            KeyCode::Char('6') => app.set_current_tab(Tab::Xrefs),
                            KeyCode::Char('7') => app.set_current_tab(Tab::Bookmarks),
                            KeyCode::Char('8') => app.set_current_tab(Tab::ControlFlow),
                            KeyCode::Char('9') => app.set_current_tab(Tab::GraphView),
                            KeyCode::Char('0') => app.set_current_tab(Tab::HexDump),
                            KeyCode::Char('S') => app.set_current_tab(Tab::Strings),
                            KeyCode::Char('D') => app.set_current_tab(Tab::Data),
                            KeyCode::Char('L') => app.set_current_tab(Tab::Relocations),
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
                            KeyCode::Char('f') => {
                                if app.current_tab == Tab::GraphView {
                                    app.toggle_graph_scope();
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
