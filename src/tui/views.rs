use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear as RatatuiClear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{App, NameItem, Tab};
use crate::graph::ControlFlowGraph;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    // Render tabs with proper ratatui styling
    let tab_titles: Vec<Line> = [
        "Instructions",
        "Functions",
        "Names",
        "Xrefs",
        "Control Flow",
        "Graph View",
        "Hex Dump",
    ]
    .iter()
    .cloned()
    .map(Line::from)
    .collect();

    let selected_tab = match app.current_tab {
        Tab::Instructions => 0,
        Tab::Functions => 1,
        Tab::Names => 2,
        Tab::Xrefs => 3,
        Tab::ControlFlow => 4,
        Tab::GraphView => 5,
        Tab::HexDump => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sharingan Disassembler"),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray)
                .fg(Color::White),
        );

    f.render_widget(tabs, chunks[0]);

    // Render main content based on selected tab
    match app.current_tab {
        Tab::Instructions => render_instructions(f, app, chunks[1]),
        Tab::Functions => render_functions(f, app, chunks[1]),
        Tab::Names => render_names(f, app, chunks[1]),
        Tab::Xrefs => render_xrefs(f, app, chunks[1]),
        Tab::ControlFlow => render_control_flow(f, app, chunks[1]),
        Tab::GraphView => render_graph_view(f, app, chunks[1]),
        Tab::HexDump => render_hex_dump(f, app, chunks[1]),
    }

    // Render status bar
    render_status_bar(f, app, chunks[2]);

    // Render help overlay if needed
    if app.show_help {
        render_help(f);
    }
}

fn render_instructions(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // VIEWPORT OPTIMIZATION: Only render visible instructions
    let visible_start = app
        .instruction_list_state
        .selected()
        .unwrap_or(0)
        .saturating_sub(50);
    let visible_end = (visible_start + 100).min(app.filtered_instructions.len());

    let instructions: Vec<ListItem> = app.filtered_instructions[visible_start..visible_end]
        .iter()
        .enumerate()
        .map(|(list_idx, &instr_idx)| {
            let actual_idx = visible_start + list_idx;
            let is_selected = app.instruction_list_state.selected() == Some(actual_idx);

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // USE CACHED STRING - NO FORMATTING!
            let content = &app.instruction_display_cache[instr_idx];
            ListItem::new(Line::from(Span::styled(content.clone(), style)))
        })
        .collect();

    let instructions_list = List::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Instructions"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(
        instructions_list,
        chunks[0],
        &mut app.instruction_list_state,
    );
    render_instruction_details(f, app, chunks[1]);
}

fn render_instruction_details(f: &mut Frame, app: &App, area: Rect) {
    let selected_instr = app
        .selected_instruction
        .and_then(|i| app.instructions.get(i));

    let content = if let Some(instr) = selected_instr {
        let bytes_str = instr
            .bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let mut details_text = vec![
            Line::from(vec![
                Span::styled("Address: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:#08x}", instr.address)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Instruction: ", Style::default().fg(Color::Cyan)),
                Span::raw(&instr.text),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Bytes: ", Style::default().fg(Color::Cyan)),
                Span::raw(bytes_str),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Length: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} bytes", instr.bytes.len())),
            ]),
        ];

        // Add basic graph context if available (simplified)
        if let Some(cfg) = &app.cfg {
            let addr = crate::graph::Address(instr.address);

            // Find containing block
            for (block_addr, block) in &cfg.blocks {
                if block.instructions.iter().any(|i| i.address == addr) {
                    details_text.push(Line::from(""));
                    details_text.push(Line::from(vec![Span::styled(
                        "Block Context:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    details_text.push(Line::from(vec![
                        Span::styled("Block: ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{}", block_addr)),
                    ]));
                    details_text.push(Line::from(vec![
                        Span::styled("Block Size: ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{} instructions", block.instructions.len())),
                    ]));

                    if !block.successors.is_empty() {
                        details_text.push(Line::from(vec![
                            Span::styled("Successors: ", Style::default().fg(Color::Cyan)),
                            Span::raw(format!("{:?}", block.successors)),
                        ]));
                    }
                    break;
                }
            }
        }

        details_text
    } else {
        vec![Line::from("No instruction selected")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_functions(f: &mut Frame, app: &mut App, area: Rect) {
    if app.functions.is_empty() {
        let paragraph =
            Paragraph::new("No functions inferred\n\nOpen a binary with --tui to build CFG-backed function summaries")
                .block(Block::default().borders(Borders::ALL).title("Functions"))
                .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    let function_items: Vec<ListItem> = app
        .functions
        .iter()
        .enumerate()
        .map(|(idx, function)| {
            let is_selected = app.selected_function == Some(idx);
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{:#014x}", function.entry.0),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!("  callers: {}", function.caller_count),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(Span::styled(
                    format!(
                        "  blocks: {}  instructions: {}  edges: {}",
                        function.block_count, function.instruction_count, function.edge_count
                    ),
                    style,
                )),
            ])
        })
        .collect();

    let functions_list = List::new(function_items)
        .block(Block::default().borders(Borders::ALL).title("Functions"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(functions_list, chunks[0], &mut app.function_list_state);

    render_function_details(f, app, chunks[1]);
}

fn render_function_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(function_idx) = app.selected_function else {
        let paragraph = Paragraph::new("No function selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Function Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(function) = app.functions.get(function_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Entry: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#014x}", function.entry.0)),
        ]),
        Line::from(vec![
            Span::styled("Blocks: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.block_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Instructions: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.instruction_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Internal Edges: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.edge_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Callers: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.caller_count.to_string()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Enter to jump to entry",
            Style::default().fg(Color::Green),
        )]),
    ];

    if let Some(cfg) = &app.cfg {
        let callers: Vec<_> = cfg
            .edges
            .iter()
            .filter(|edge| {
                edge.edge_type == crate::graph::EdgeType::Call && edge.to == function.entry
            })
            .map(|edge| edge.from)
            .take(8)
            .collect();

        if !callers.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Call Sites:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));

            for caller in callers {
                lines.push(Line::from(format!("  {}", caller)));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Function Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_names(f: &mut Frame, app: &mut App, area: Rect) {
    if app.names.is_empty() {
        let paragraph = Paragraph::new("No imports, symbols, or strings found")
            .block(Block::default().borders(Borders::ALL).title("Names"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let name_items: Vec<ListItem> = app
        .names
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let style = if app.selected_name == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let address = item
                .address()
                .map(|address| format!("{address:#014x}"))
                .unwrap_or_else(|| "unknown".to_string());

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", address), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" {:<8} ", item.kind()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(item.label(), style),
            ]))
        })
        .collect();

    let names_list = List::new(name_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Names: {} imports, {} symbols, {} strings",
            app.analysis.imports.len(),
            app.analysis.symbols.len(),
            app.analysis.strings.len()
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(names_list, chunks[0], &mut app.name_list_state);
    render_name_details(f, app, chunks[1]);
}

fn render_name_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(name_idx) = app.selected_name else {
        let paragraph = Paragraph::new("No name selected")
            .block(Block::default().borders(Borders::ALL).title("Name Details"));
        f.render_widget(paragraph, area);
        return;
    };

    let Some(item) = app.names.get(name_idx) else {
        return;
    };

    let address = item
        .address()
        .map(|address| format!("{address:#x}"))
        .unwrap_or_else(|| "unknown".to_string());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(item.kind()),
        ]),
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(address),
        ]),
        Line::from(""),
    ];

    match item {
        NameItem::Import(import) => {
            lines.push(Line::from(vec![
                Span::styled("Library: ", Style::default().fg(Color::Cyan)),
                Span::raw(import.library.as_deref().unwrap_or("unknown")),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(import.name.clone()),
            ]));
        }
        NameItem::Symbol(symbol) => {
            lines.push(Line::from(vec![
                Span::styled("Symbol: ", Style::default().fg(Color::Cyan)),
                Span::raw(symbol.name.clone()),
            ]));
        }
        NameItem::String(string) => {
            lines.push(Line::from(vec![
                Span::styled("Value: ", Style::default().fg(Color::Cyan)),
                Span::raw(string.value.clone()),
            ]));
        }
    }

    if item.address().is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Press Enter to jump near address",
            Style::default().fg(Color::Green),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Name Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_xrefs(f: &mut Frame, app: &mut App, area: Rect) {
    if app.xrefs.is_empty() {
        let paragraph = Paragraph::new(
            "No cross-references available\n\nOpen a binary with --tui to build CFG-backed xrefs",
        )
        .block(Block::default().borders(Borders::ALL).title("Xrefs"))
        .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let xref_items: Vec<ListItem> = app
        .xrefs
        .iter()
        .enumerate()
        .map(|(idx, xref)| {
            let style = if app.selected_xref == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<8}", xref.kind()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(format!(" {} ", xref.from), Style::default().fg(Color::Cyan)),
                Span::raw("-> "),
                Span::styled(format!("{}", xref.to), style),
            ]))
        })
        .collect();

    let xrefs_list = List::new(xref_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Cross References: {}", app.xrefs.len())),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(xrefs_list, chunks[0], &mut app.xref_list_state);
    render_xref_details(f, app, chunks[1]);
}

fn render_xref_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(xref_idx) = app.selected_xref else {
        let paragraph = Paragraph::new("No xref selected")
            .block(Block::default().borders(Borders::ALL).title("Xref Details"));
        f.render_widget(paragraph, area);
        return;
    };

    let Some(xref) = app.xrefs.get(xref_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(xref.kind()),
        ]),
        Line::from(vec![
            Span::styled("From: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", xref.from)),
        ]),
        Line::from(vec![
            Span::styled("To: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", xref.to)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Enter to jump to source",
            Style::default().fg(Color::Green),
        )]),
    ];

    if let Some(cfg) = &app.cfg {
        if let Some(block) = cfg.blocks.get(&xref.from) {
            if let Some(instruction) = block.instructions.last() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Source Instruction:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(format!("  {}", instruction)));
            }
        }

        if let Some(block) = cfg.blocks.get(&xref.to) {
            if let Some(instruction) = block.instructions.first() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Target Block:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(format!("  {}", instruction)));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Xref Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_control_flow(f: &mut Frame, app: &App, area: Rect) {
    if let Some(cfg) = &app.cfg {
        // Split into metrics and blocks sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);

        // Render metrics section
        render_cfg_metrics(f, cfg, chunks[0]);

        // Render blocks section
        render_cfg_blocks(f, cfg, chunks[1]);
    } else {
        let paragraph =
            Paragraph::new("No control flow graph available\n\nUse --cfg flag to generate CFG")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Control Flow Graph"),
                )
                .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }
}

fn render_cfg_metrics(f: &mut Frame, cfg: &ControlFlowGraph, area: Rect) {
    let total_blocks = cfg.blocks.len();
    let total_edges = cfg.edges.len();

    // Fixed: Use saturating_sub to prevent underflow
    let cyclomatic_complexity = if total_blocks > 0 {
        total_edges.saturating_sub(total_blocks).saturating_add(2)
    } else {
        0
    };

    let metrics_text = vec![
        Line::from(vec![
            Span::styled("Blocks: ", Style::default().fg(Color::Cyan)),
            Span::raw(total_blocks.to_string()),
            Span::styled(" | Edges: ", Style::default().fg(Color::Cyan)),
            Span::raw(total_edges.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Complexity: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                cyclomatic_complexity.to_string(),
                if cyclomatic_complexity > 10 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Graph Type: ", Style::default().fg(Color::Cyan)),
            Span::raw("Control Flow Graph"),
        ]),
    ];

    let metrics_paragraph = Paragraph::new(metrics_text)
        .block(Block::default().borders(Borders::ALL).title("CFG Metrics"))
        .wrap(Wrap { trim: true });

    f.render_widget(metrics_paragraph, area);
}

fn render_cfg_blocks(f: &mut Frame, cfg: &ControlFlowGraph, area: Rect) {
    let mut sorted_blocks: Vec<_> = cfg.blocks.iter().collect();
    sorted_blocks.sort_by_key(|(addr, _)| addr.0);

    let block_items: Vec<ListItem> = sorted_blocks
        .iter()
        .take(20)
        .map(|(addr, block)| {
            // Remove block_type since it doesn't exist
            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{}", addr), Style::default().fg(Color::Cyan)),
                    Span::styled(" (Basic Block)", Style::default().fg(Color::White)), // Generic label
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Instructions: ", Style::default().fg(Color::Gray)),
                    Span::raw(block.instructions.len().to_string()),
                    Span::styled(" | Successors: ", Style::default().fg(Color::Gray)),
                    Span::raw(block.successors.len().to_string()),
                ]),
            ];

            ListItem::new(content)
        })
        .collect();

    let blocks_list = List::new(block_items)
        .block(Block::default().borders(Borders::ALL).title("Basic Blocks"))
        .style(Style::default().fg(Color::White));

    f.render_widget(blocks_list, area);
}

fn render_hex_dump(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let content = if let Some(selected_idx) = app.selected_instruction {
        if selected_idx < app.instructions.len() && !app.instructions.is_empty() {
            // REDUCED context size for performance
            let context_size = 4; // Reduced from 8
            let start_idx = selected_idx.saturating_sub(context_size);
            let end_idx = selected_idx
                .saturating_add(context_size)
                .saturating_add(1)
                .min(app.instructions.len());

            if start_idx < app.instructions.len()
                && end_idx <= app.instructions.len()
                && start_idx < end_idx
            {
                // PRE-ALLOCATE with capacity
                let mut hex_lines = Vec::with_capacity(end_idx - start_idx);
                let mut ascii_lines = Vec::with_capacity(end_idx - start_idx);

                for (offset, inst) in app.instructions[start_idx..end_idx].iter().enumerate() {
                    let actual_idx = start_idx + offset;
                    let is_selected = actual_idx == selected_idx;

                    // LIMIT bytes to prevent excessive formatting
                    let bytes_to_show = inst.bytes.iter().take(8); // Reduced from 16

                    // PRE-ALLOCATE string capacity
                    let mut bytes_str = String::with_capacity(24);
                    for (i, b) in bytes_to_show.enumerate() {
                        if i > 0 {
                            bytes_str.push(' ');
                        }
                        bytes_str.push_str(&format!("{:02x}", b));
                    }

                    // ASCII with capacity
                    let mut ascii_str = String::with_capacity(8);
                    for &b in inst.bytes.iter().take(8) {
                        ascii_str.push(if b.is_ascii_graphic() || b == b' ' {
                            b as char
                        } else {
                            '.'
                        });
                    }

                    let style = if is_selected {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        Style::default()
                    };

                    hex_lines.push(ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:#08x}: ", inst.address),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(format!("{:<24}", bytes_str), style),
                    ])));

                    ascii_lines.push(ListItem::new(Line::from(Span::styled(ascii_str, style))));
                }

                (hex_lines, ascii_lines)
            } else {
                (vec![ListItem::new("Invalid range")], vec![])
            }
        } else {
            (vec![ListItem::new("No valid instruction selected")], vec![])
        }
    } else {
        (vec![ListItem::new("No instruction selected")], vec![])
    };

    let hex_list = List::new(content.0)
        .block(Block::default().borders(Borders::ALL).title("Hex View"))
        .style(Style::default().fg(Color::White));

    f.render_widget(hex_list, chunks[0]);

    let ascii_list = List::new(content.1)
        .block(Block::default().borders(Borders::ALL).title("ASCII"))
        .style(Style::default().fg(Color::White));

    f.render_widget(ascii_list, chunks[1]);
}

// Update render_status_bar to show search mode
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.address_jump_mode {
        format!(
            "GOTO: '{}' | Enter jumps, ESC cancels | Back: {} Forward: {}",
            app.address_jump_query,
            app.back_stack.len(),
            app.forward_stack.len()
        )
    } else if app.search_mode {
        format!(
            "SEARCH: '{}' | Instructions: {} | Selected: {} | Tab: {:?} | ESC to exit search",
            app.search_query,
            app.filtered_instructions.len(),
            app.selected_instruction
                .map_or("None".to_string(), |i| (i + 1).to_string()),
            app.current_tab
        )
    } else {
        let message = app
            .status_message
            .as_ref()
            .map(|message| format!(" | {message}"))
            .unwrap_or_default();
        format!(
            "Instructions: {} | Functions: {} | Names: {} | Xrefs: {} | Selected: {} | Tab: {:?} | g goto, u/r back/forward, '/' search, h help, q quit{}",
            app.instructions.len(),
            app.functions.len(),
            app.names.len(),
            app.xrefs.len(),
            app.selected_instruction
                .map_or("None".to_string(), |i| (i + 1).to_string()),
            app.current_tab,
            message
        )
    };

    let paragraph = Paragraph::new(status).block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ↑/k        ", Style::default().fg(Color::Green)),
            Span::raw("- Previous instruction"),
        ]),
        Line::from(vec![
            Span::styled("  ↓/j        ", Style::default().fg(Color::Green)),
            Span::raw("- Next instruction"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn  ", Style::default().fg(Color::Green)),
            Span::raw("- Jump 10 instructions"),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", Style::default().fg(Color::Green)),
            Span::raw("- Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab  ", Style::default().fg(Color::Green)),
            Span::raw("- Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  1-7        ", Style::default().fg(Color::Green)),
            Span::raw("- Select specific tab"),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", Style::default().fg(Color::Green)),
            Span::raw("- Jump from selected function/name/xref"),
        ]),
        Line::from(vec![
            Span::styled("  g          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to address"),
        ]),
        Line::from(vec![
            Span::styled("  u/r        ", Style::default().fg(Color::Green)),
            Span::raw("- Navigation back/forward"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tabs:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  1 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Instructions"),
        ]),
        Line::from(vec![
            Span::styled("  2 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Functions"),
        ]),
        Line::from(vec![
            Span::styled("  3 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Names"),
        ]),
        Line::from(vec![
            Span::styled("  4 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Xrefs"),
        ]),
        Line::from(vec![
            Span::styled("  5 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Control Flow"),
        ]),
        Line::from(vec![
            Span::styled("  6 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Graph Analysis"),
        ]),
        Line::from(vec![
            Span::styled("  7 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Hex Dump"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  h/F1       ", Style::default().fg(Color::Green)),
            Span::raw("- Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q          ", Style::default().fg(Color::Green)),
            Span::raw("- Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press 'h' again to close this help",
            Style::default().fg(Color::Magenta),
        )]),
    ];

    // Clear the area first
    f.render_widget(RatatuiClear, area);

    let block = Block::default()
        .title(vec![Span::styled(
            "Help",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )])
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_graph_view(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref cfg) = app.cfg {
        // Only build graph view if not already computed
        if !app.graph_view.layout_computed {
            app.graph_view.build_from_cfg(cfg);
        }

        // Split area between graph and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Render the main graph
        app.graph_renderer
            .render_graph_view(f, chunks[0], &mut app.graph_view, cfg);

        // Render block details
        app.graph_renderer
            .render_block_details(f, chunks[1], &app.graph_view, cfg);
    } else {
        let paragraph =
            Paragraph::new("No control flow graph available\n\nUse --cfg flag to generate CFG")
                .block(Block::default().borders(Borders::ALL).title("Graph View"))
                .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }
}
