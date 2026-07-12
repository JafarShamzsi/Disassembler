use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear as RatatuiClear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{truncate_for_display, App, NameItem, Tab};
use crate::graph::{Address, ControlFlowGraph};

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
        "Overview",
        "Instructions",
        "Functions",
        "Call Graph",
        "Imports",
        "Exports",
        "Symbols",
        "Names",
        "Strings",
        "Data",
        "Relocations",
        "Sections",
        "Xrefs",
        "Bookmarks",
        "Control Flow",
        "Graph View",
        "Hex Dump",
    ]
    .iter()
    .cloned()
    .map(Line::from)
    .collect();

    let selected_tab = match app.current_tab {
        Tab::Overview => 0,
        Tab::Instructions => 1,
        Tab::Functions => 2,
        Tab::CallGraph => 3,
        Tab::Imports => 4,
        Tab::Exports => 5,
        Tab::Symbols => 6,
        Tab::Names => 7,
        Tab::Strings => 8,
        Tab::Data => 9,
        Tab::Relocations => 10,
        Tab::Sections => 11,
        Tab::Xrefs => 12,
        Tab::Bookmarks => 13,
        Tab::ControlFlow => 14,
        Tab::GraphView => 15,
        Tab::HexDump => 16,
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
        Tab::Overview => render_overview(f, app, chunks[1]),
        Tab::Instructions => render_instructions(f, app, chunks[1]),
        Tab::Functions => render_functions(f, app, chunks[1]),
        Tab::CallGraph => render_call_graph(f, app, chunks[1]),
        Tab::Imports => render_imports(f, app, chunks[1]),
        Tab::Exports => render_exports(f, app, chunks[1]),
        Tab::Symbols => render_symbols(f, app, chunks[1]),
        Tab::Names => render_names(f, app, chunks[1]),
        Tab::Strings => render_strings(f, app, chunks[1]),
        Tab::Data => render_data_objects(f, app, chunks[1]),
        Tab::Relocations => render_relocations(f, app, chunks[1]),
        Tab::Sections => render_sections(f, app, chunks[1]),
        Tab::Xrefs => render_xrefs(f, app, chunks[1]),
        Tab::Bookmarks => render_bookmarks(f, app, chunks[1]),
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

fn render_overview(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let metadata_lines = if let Some(metadata) = &app.metadata {
        vec![
            Line::from(vec![
                Span::styled("Format: ", Style::default().fg(Color::Cyan)),
                Span::raw(metadata.format.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Architecture: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "{} / {}-bit",
                    metadata.architecture, metadata.bitness
                )),
            ]),
            Line::from(vec![
                Span::styled("Endianness: ", Style::default().fg(Color::Cyan)),
                Span::raw(metadata.endianness.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Entry Point: ", Style::default().fg(Color::Cyan)),
                Span::raw(
                    metadata
                        .entry_point
                        .map(|entry| format!("{entry:#x}"))
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
            ]),
        ]
    } else {
        vec![Line::from("Metadata unavailable for this session")]
    };

    let project_state = if app.project_dirty { "dirty" } else { "clean" };
    let mut left_lines = vec![
        Line::from(vec![Span::styled(
            "Binary",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Cyan)),
            Span::raw(truncate_for_display(&app.binary_path_display(), 96)),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} bytes", app.binary_size())),
        ]),
        Line::from(vec![
            Span::styled("Fingerprint: ", Style::default().fg(Color::Cyan)),
            Span::raw(app.binary_fingerprint().unwrap_or("unknown")),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Format",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
    ];
    left_lines.extend(metadata_lines);
    left_lines.extend([
        Line::from(""),
        Line::from(vec![Span::styled(
            "Analysis",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!(
            "Instructions: {}   Functions: {}   Call edges: {}   Noreturn imports: {}",
            app.instructions.len(),
            app.call_graph
                .as_ref()
                .map_or(app.functions.len(), |graph| graph.functions.len()),
            app.call_graph
                .as_ref()
                .map_or(0, |graph| graph.total_edge_count()),
            app.cfg
                .as_ref()
                .map_or(0, ControlFlowGraph::noreturn_call_target_count)
        )),
        Line::from(format!(
            "Imports: {}   Exports: {}   Symbols: {}   Strings: {}   Data pointers: {}   Relocations: {}   Unwind funcs: {}",
            app.analysis.imports.len(),
            app.analysis.exports.len(),
            app.analysis.symbols.len(),
            app.analysis.strings.len(),
            app.analysis.data_objects.len(),
            app.analysis.relocations.len(),
            app.analysis.function_ranges.len()
        )),
        Line::from(format!(
            "Sections: {}   Xrefs: {}   Bookmarks: {}",
            app.sections.len(),
            app.xrefs.len(),
            app.bookmark_count()
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Project",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!(
            "State: {project_state}   Names: {}   Comments: {}   Bookmarks: {}",
            app.project
                .as_ref()
                .map_or(0, |project| project.user_names.len()),
            app.project
                .as_ref()
                .map_or(0, |project| project.comments.len()),
            app.bookmark_count()
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter jumps to the binary entry point",
            Style::default().fg(Color::Green),
        )]),
    ]);

    let left = Paragraph::new(left_lines)
        .block(Block::default().borders(Borders::ALL).title("Overview"))
        .wrap(Wrap { trim: true });
    f.render_widget(left, chunks[0]);

    let mut top_sections = app.sections.clone();
    top_sections
        .sort_by_key(|section| std::cmp::Reverse(section.file_size.max(section.virtual_size)));

    let mut right_lines = vec![
        Line::from(vec![Span::styled(
            "Section Mix",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!(
            "Executable: {}   Data-like: {}   Writable: {}",
            app.executable_section_count(),
            app.data_section_count(),
            app.writable_section_count()
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Largest Sections",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    if top_sections.is_empty() {
        right_lines.push(Line::from("No sections found"));
    } else {
        for section in top_sections.iter().take(10) {
            right_lines.push(Line::from(format!(
                "{:<14} {}  VA {:#x}  file {:#x}",
                truncate_for_display(&section.name, 14),
                section.permissions(),
                section.address,
                section.file_size
            )));
        }
    }

    right_lines.extend([
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("Tab cycles views. O returns here. S opens Strings. D opens Data."),
        Line::from(
            "Use / to search across overview, code, names, strings, data, sections, and xrefs.",
        ),
    ]);

    let right = Paragraph::new(right_lines)
        .block(Block::default().borders(Borders::ALL).title("Orientation"))
        .wrap(Wrap { trim: true });
    f.render_widget(right, chunks[1]);
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

            let content = app.instruction_display_text(instr_idx);
            ListItem::new(Line::from(Span::styled(content, style)))
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

        if let Some(name) = app.project_user_name_at(instr.address) {
            details_text.push(Line::from(""));
            details_text.push(Line::from(vec![
                Span::styled("User Name: ", Style::default().fg(Color::Green)),
                Span::raw(name.to_string()),
            ]));
        }

        if app.project_bookmark_at(instr.address).is_some() {
            details_text.push(Line::from(vec![
                Span::styled("Bookmark: ", Style::default().fg(Color::Green)),
                Span::raw("yes"),
            ]));
        }

        if let Some(comment) = app.project_comment_at(instr.address) {
            details_text.push(Line::from(vec![
                Span::styled("Comment: ", Style::default().fg(Color::Green)),
                Span::raw(comment.to_string()),
            ]));
        }

        // Add basic graph context if available (simplified)

        if let Some(cfg) = &app.cfg {
            let addr = crate::graph::Address(instr.address);
            let context = app.address_context(addr);

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

            details_text.push(Line::from(""));
            details_text.push(Line::from(vec![Span::styled(
                "Address Context:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));

            if let Some(function) = context.containing_function {
                details_text.push(Line::from(vec![
                    Span::styled("Function: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:#014x}", function.entry.0)),
                ]));
            }

            if let Some(section) = context.containing_section {
                details_text.push(Line::from(vec![
                    Span::styled("Section: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{} {}", section.name, section.permissions())),
                ]));
            }

            if let Some(name) = context.nearest_name {
                details_text.push(Line::from(vec![
                    Span::styled("Nearest Name: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{} {}", name.kind(), name.label())),
                ]));
            }

            if !context.incoming_xrefs.is_empty() {
                details_text.push(Line::from(vec![
                    Span::styled("Incoming Xrefs: ", Style::default().fg(Color::Cyan)),
                    Span::raw(
                        context
                            .incoming_xrefs
                            .iter()
                            .take(4)
                            .map(|xref| format!("{} {}", xref.kind(), xref.from))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                ]));
            }

            if !context.outgoing_xrefs.is_empty() {
                details_text.push(Line::from(vec![
                    Span::styled("Outgoing Xrefs: ", Style::default().fg(Color::Cyan)),
                    Span::raw(
                        context
                            .outgoing_xrefs
                            .iter()
                            .take(4)
                            .map(|xref| format!("{} {}", xref.kind(), xref.to))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                ]));
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

            let user_name = app.project_user_name_at(function.entry.0);
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{:#014x}", function.entry.0),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        user_name
                            .map(|name| format!("  {}", name))
                            .unwrap_or_default(),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!(
                            "  {} / {}  callers: {}",
                            function.kind.as_str(),
                            function.confidence.as_str(),
                            function.caller_count
                        ),
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
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.kind.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Confidence: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.confidence.as_str()),
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

    if let Some(name) = app.project_user_name_at(function.entry.0) {
        lines.insert(
            1,
            Line::from(vec![
                Span::styled("User Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(name.to_string()),
            ]),
        );
    }

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

fn render_call_graph(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(call_graph) = &app.call_graph else {
        let paragraph = Paragraph::new("No call graph available\n\nOpen a binary with --tui to build CFG-backed function calls")
            .block(Block::default().borders(Borders::ALL).title("Call Graph"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    };

    if call_graph.functions.is_empty() {
        let paragraph = Paragraph::new("No function-level calls inferred")
            .block(Block::default().borders(Borders::ALL).title("Call Graph"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    let call_items: Vec<ListItem> = call_graph
        .functions
        .iter()
        .enumerate()
        .map(|(idx, function)| {
            let is_selected = app.selected_call_graph == Some(idx);
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let entry = function.summary.entry.0;
            let user_name = app.project_user_name_at(entry);
            let import_thunk = function.import_thunk.as_ref();

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{:#014x}", entry), Style::default().fg(Color::Cyan)),
                    Span::styled(
                        user_name
                            .map(|name| format!("  {}", name))
                            .unwrap_or_default(),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        import_thunk
                            .map(|target| format!("  => {}", target.label))
                            .unwrap_or_default(),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
                Line::from(Span::styled(
                    format!(
                        "  {} / {}  callers: {}  calls: {}  internal: {}  imports: {}",
                        function.summary.kind.as_str(),
                        function.summary.confidence.as_str(),
                        function.incoming_call_count,
                        function.outgoing_call_count,
                        call_graph
                            .edges
                            .iter()
                            .filter(|edge| edge.caller == function.summary.entry)
                            .count(),
                        call_graph
                            .external_edges
                            .iter()
                            .filter(|edge| edge.caller == function.summary.entry)
                            .count()
                    ),
                    style,
                )),
            ])
        })
        .collect();

    let call_list = List::new(call_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Call Graph: {} functions, {} internal, {} imports, {} thunks",
            call_graph.functions.len(),
            call_graph.edges.len(),
            call_graph.external_edges.len(),
            call_graph
                .functions
                .iter()
                .filter(|function| function.import_thunk.is_some())
                .count()
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(call_list, chunks[0], &mut app.call_graph_list_state);
    render_call_graph_details(f, app, chunks[1]);
}

fn render_call_graph_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(call_graph) = &app.call_graph else {
        return;
    };
    let Some(function_idx) = app.selected_call_graph else {
        let paragraph = Paragraph::new("No function selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Call Graph Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };
    let Some(function) = call_graph.functions.get(function_idx) else {
        return;
    };

    let entry = function.summary.entry;
    let callers: Vec<_> = call_graph
        .edges
        .iter()
        .filter(|edge| edge.callee == entry)
        .take(8)
        .collect();
    let callees: Vec<_> = call_graph
        .edges
        .iter()
        .filter(|edge| edge.caller == entry)
        .take(8)
        .collect();
    let external_callees: Vec<_> = call_graph
        .external_edges
        .iter()
        .filter(|edge| edge.caller == entry)
        .take(8)
        .collect();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Function: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#014x}", entry.0)),
        ]),
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.summary.kind.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Confidence: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.summary.confidence.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Incoming Calls: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.incoming_call_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Outgoing Calls: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.outgoing_call_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Blocks: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.summary.block_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Instructions: ", Style::default().fg(Color::Cyan)),
            Span::raw(function.summary.instruction_count.to_string()),
        ]),
    ];

    let has_user_name = app.project_user_name_at(entry.0).is_some();
    if let Some(name) = app.project_user_name_at(entry.0) {
        lines.insert(
            1,
            Line::from(vec![
                Span::styled("User Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(name.to_string()),
            ]),
        );
    }

    if let Some(target) = &function.import_thunk {
        lines.insert(
            if has_user_name { 3 } else { 2 },
            Line::from(vec![
                Span::styled("Import Thunk: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} ({})", target.label, target.address)),
            ]),
        );
    }

    if !callers.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Callers:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        for edge in callers {
            let sites = edge
                .call_sites
                .iter()
                .take(3)
                .map(|site| format!("{}", site))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(Line::from(format!("  {}  sites: {}", edge.caller, sites)));
        }
    }

    if !callees.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Internal Callees:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        for edge in callees {
            let sites = edge
                .call_sites
                .iter()
                .take(3)
                .map(|site| format!("{}", site))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(Line::from(format!("  {}  sites: {}", edge.callee, sites)));
        }
    }

    if !external_callees.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "External Import Callees:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        for edge in external_callees {
            let sites = edge
                .call_sites
                .iter()
                .take(3)
                .map(|site| format!("{}", site))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(Line::from(format!(
                "  {} ({})  sites: {}",
                edge.label, edge.target, sites
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter jumps to function entry",
        Style::default().fg(Color::Green),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Call Graph Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_imports(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.imports.is_empty() {
        let paragraph = Paragraph::new("No imports found")
            .block(Block::default().borders(Borders::ALL).title("Imports"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let import_items: Vec<ListItem> = app
        .analysis
        .imports
        .iter()
        .enumerate()
        .map(|(idx, import)| {
            let style = if app.selected_import == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let address = import
                .address
                .map(|address| format!("{address:#014x}"))
                .unwrap_or_else(|| "unknown".to_string());
            let library = import.library.as_deref().unwrap_or("unknown");

            ListItem::new(Line::from(vec![
                Span::styled(address, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" {:<20} ", truncate_for_display(library, 20)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncate_for_display(&import.name, 64), style),
            ]))
        })
        .collect();

    let library_count = app
        .analysis
        .imports
        .iter()
        .filter_map(|import| import.library.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let imports_list = List::new(import_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Imports: {} entries across {} libraries",
            app.analysis.imports.len(),
            library_count
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(imports_list, chunks[0], &mut app.import_list_state);
    render_import_details(f, app, chunks[1]);
}

fn render_import_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(import_idx) = app.selected_import else {
        let paragraph = Paragraph::new("No import selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Import Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(import) = app.analysis.imports.get(import_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Library: ", Style::default().fg(Color::Cyan)),
            Span::raw(import.library.as_deref().unwrap_or("unknown")),
        ]),
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(import.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("IAT Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(
                import
                    .address
                    .map(|address| format!("{address:#x}"))
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
        ]),
    ];

    if let Some(address) = import.address.map(Address) {
        if let Some(section) = app.section_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("Section: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} {}", section.name, section.permissions())),
            ]));
        }
        if let Some(file_offset) = app.file_offset_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{file_offset:#x}")),
            ]));
        }

        let incoming_xrefs: Vec<_> = app
            .xrefs
            .iter()
            .filter(|xref| xref.kind() == "import" && xref.to == address)
            .take(10)
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Incoming Xrefs:",
            Style::default().fg(Color::Cyan),
        )]));
        if incoming_xrefs.is_empty() {
            lines.push(Line::from("  none found"));
        } else {
            for xref in incoming_xrefs {
                lines.push(Line::from(format!("  {} from {}", xref.kind(), xref.from)));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Enter opens first xref or the import slot in Hex Dump",
            Style::default().fg(Color::Green),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Import Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
fn render_exports(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.exports.is_empty() {
        let paragraph = Paragraph::new("No exports found")
            .block(Block::default().borders(Borders::ALL).title("Exports"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let export_items: Vec<ListItem> = app
        .analysis
        .exports
        .iter()
        .enumerate()
        .map(|(idx, export)| {
            let style = if app.selected_export == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let address = export
                .address
                .map(|address| format!("{address:#014x}"))
                .unwrap_or_else(|| "unknown".to_string());
            let forwarder = export
                .forwarder
                .as_deref()
                .map(|target| format!(" -> {target}"))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(address, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" {:<8} ", export.kind.as_str()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncate_for_display(&export.name, 52), style),
                Span::styled(
                    truncate_for_display(&forwarder, 36),
                    Style::default().fg(Color::Magenta),
                ),
            ]))
        })
        .collect();

    let exports_list = List::new(export_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Exports: {} entries", app.analysis.exports.len())),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(exports_list, chunks[0], &mut app.export_list_state);
    render_export_details(f, app, chunks[1]);
}

fn render_export_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(export_idx) = app.selected_export else {
        let paragraph = Paragraph::new("No export selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Export Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(export) = app.analysis.exports.get(export_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(export.kind.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(export.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(
                export
                    .address
                    .map(|address| format!("{address:#x}"))
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Cyan)),
            Span::raw(export.size.to_string()),
        ]),
    ];

    if let Some(forwarder) = &export.forwarder {
        lines.push(Line::from(vec![
            Span::styled("Forwarder: ", Style::default().fg(Color::Cyan)),
            Span::raw(forwarder.clone()),
        ]));
    }

    if let Some(address) = export.address.map(Address) {
        if let Some(section) = app.section_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("Section: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} {}", section.name, section.permissions())),
            ]));
        }
        if let Some(file_offset) = app.file_offset_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{file_offset:#x}")),
            ]));
        }

        let incoming_xrefs: Vec<_> = app
            .xrefs
            .iter()
            .filter(|xref| xref.to == address)
            .take(8)
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Incoming Xrefs:",
            Style::default().fg(Color::Cyan),
        )]));
        if incoming_xrefs.is_empty() {
            lines.push(Line::from("  none found"));
        } else {
            for xref in incoming_xrefs {
                lines.push(Line::from(format!(
                    "  {} from {} {}",
                    xref.kind(),
                    xref.from,
                    xref.label.as_deref().unwrap_or("")
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Enter opens executable exports or mapped bytes",
            Style::default().fg(Color::Green),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Export Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
fn render_symbols(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.symbols.is_empty() {
        let paragraph = Paragraph::new("No symbols found")
            .block(Block::default().borders(Borders::ALL).title("Symbols"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let symbol_items: Vec<ListItem> = app
        .analysis
        .symbols
        .iter()
        .enumerate()
        .map(|(idx, symbol)| {
            let style = if app.selected_symbol == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let address = symbol
                .address
                .map(|address| format!("{address:#014x}"))
                .unwrap_or_else(|| "unknown".to_string());

            ListItem::new(Line::from(vec![
                Span::styled(address, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" {:<8} ", symbol.kind.as_str()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncate_for_display(&symbol.name, 72), style),
            ]))
        })
        .collect();

    let symbols_list = List::new(symbol_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Symbols: {} entries", app.analysis.symbols.len())),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(symbols_list, chunks[0], &mut app.symbol_list_state);
    render_symbol_details(f, app, chunks[1]);
}

fn render_symbol_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(symbol_idx) = app.selected_symbol else {
        let paragraph = Paragraph::new("No symbol selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Symbol Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(symbol) = app.analysis.symbols.get(symbol_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(symbol.kind.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(symbol.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(
                symbol
                    .address
                    .map(|address| format!("{address:#x}"))
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
        ]),
    ];

    if let Some(address) = symbol.address.map(Address) {
        if let Some(section) = app.section_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("Section: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} {}", section.name, section.permissions())),
            ]));
        }
        if let Some(file_offset) = app.file_offset_for_address(address) {
            lines.push(Line::from(vec![
                Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{file_offset:#x}")),
            ]));
        }

        let incoming_xrefs: Vec<_> = app
            .xrefs
            .iter()
            .filter(|xref| xref.to == address)
            .take(8)
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Incoming Xrefs:",
            Style::default().fg(Color::Cyan),
        )]));
        if incoming_xrefs.is_empty() {
            lines.push(Line::from("  none found"));
        } else {
            for xref in incoming_xrefs {
                lines.push(Line::from(format!(
                    "  {} from {} {}",
                    xref.kind(),
                    xref.from,
                    xref.label.as_deref().unwrap_or("")
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Enter opens executable symbols or mapped bytes",
            Style::default().fg(Color::Green),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Symbol Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
fn render_names(f: &mut Frame, app: &mut App, area: Rect) {
    if app.names.is_empty() {
        let paragraph = Paragraph::new("No imports, symbols, or user names found")
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
            "Names: {} imports, {} symbols, {} user names",
            app.analysis.imports.len(),
            app.analysis.symbols.len(),
            app.project
                .as_ref()
                .map_or(0, |project| project.user_names.len())
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
        NameItem::User(user_name) => {
            lines.push(Line::from(vec![
                Span::styled("User Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(user_name.name.clone()),
            ]));
        }
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
                Span::styled("Section: ", Style::default().fg(Color::Cyan)),
                Span::raw(string.section.as_deref().unwrap_or("unknown")),
            ]));
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

fn render_strings(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.strings.is_empty() {
        let paragraph = Paragraph::new("No printable data-section strings found")
            .block(Block::default().borders(Borders::ALL).title("Strings"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let string_items: Vec<ListItem> = app
        .analysis
        .strings
        .iter()
        .enumerate()
        .map(|(idx, string)| {
            let style = if app.selected_string == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let address = format!("{:#014x}", string.address);
            let section = string.section.as_deref().unwrap_or("unknown");

            ListItem::new(Line::from(vec![
                Span::styled(format!("{address:<14}"), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" {section:<12} "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncate_for_display(&string.value, 96), style),
            ]))
        })
        .collect();

    let strings_list = List::new(string_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Strings: {} data-section strings",
            app.analysis.strings.len()
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(strings_list, chunks[0], &mut app.string_list_state);
    render_string_details(f, app, chunks[1]);
}

fn render_string_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(string_idx) = app.selected_string else {
        let paragraph = Paragraph::new("No string selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("String Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(string) = app.analysis.strings.get(string_idx) else {
        return;
    };

    let address = Address(string.address);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", string.address)),
        ]),
        Line::from(vec![
            Span::styled("Section: ", Style::default().fg(Color::Cyan)),
            Span::raw(string.section.as_deref().unwrap_or("unknown")),
        ]),
        Line::from(vec![
            Span::styled("Length: ", Style::default().fg(Color::Cyan)),
            Span::raw(string.value.len().to_string()),
        ]),
    ];

    if let Some(file_offset) = app.file_offset_for_address(address) {
        lines.push(Line::from(vec![
            Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{file_offset:#x}")),
        ]));

        let preview_len = string.value.len().clamp(1, 32);
        if let Some(bytes) = app.bytes_at_file_offset(file_offset, preview_len) {
            let hex = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(Line::from(vec![
                Span::styled("Bytes: ", Style::default().fg(Color::Cyan)),
                Span::raw(hex),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Value: ", Style::default().fg(Color::Cyan)),
        Span::raw(string.value.clone()),
    ]));

    let incoming_xrefs: Vec<_> = app
        .xrefs
        .iter()
        .filter(|xref| xref.to == address)
        .take(8)
        .collect();

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Incoming Xrefs:",
        Style::default().fg(Color::Cyan),
    )]));
    if incoming_xrefs.is_empty() {
        lines.push(Line::from("  none found"));
    } else {
        for xref in incoming_xrefs {
            let label = xref.label.as_deref().unwrap_or("");
            lines.push(Line::from(format!(
                "  {} from {} {}",
                xref.kind(),
                xref.from,
                label
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press Enter to open mapped bytes in Hex Dump",
        Style::default().fg(Color::Green),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("String Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_data_objects(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.data_objects.is_empty() {
        let paragraph = Paragraph::new("No decoded data pointers found")
            .block(Block::default().borders(Borders::ALL).title("Data"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(area);

    let data_items: Vec<ListItem> = app
        .analysis
        .data_objects
        .iter()
        .enumerate()
        .map(|(idx, object)| {
            let style = if app.selected_data == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let section = object.section.as_deref().unwrap_or("unknown");
            let label = object
                .target_label
                .as_deref()
                .or(object.target_section.as_deref())
                .unwrap_or("unknown");

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:#014x}", object.address),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!(" {section:<12} "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{} -> {:#014x} ", object.kind.as_str(), object.target),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(truncate_for_display(label, 56), style),
            ]))
        })
        .collect();

    let data_list = List::new(data_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Data: {} pointers",
            app.analysis.data_objects.len()
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(data_list, chunks[0], &mut app.data_list_state);
    render_data_object_details(f, app, chunks[1]);
}

fn render_data_object_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(data_idx) = app.selected_data else {
        let paragraph = Paragraph::new("No data object selected")
            .block(Block::default().borders(Borders::ALL).title("Data Details"));
        f.render_widget(paragraph, area);
        return;
    };

    let Some(object) = app.analysis.data_objects.get(data_idx) else {
        return;
    };

    let object_address = Address(object.address);
    let target_address = Address(object.target);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", object.address)),
        ]),
        Line::from(vec![
            Span::styled("Section: ", Style::default().fg(Color::Cyan)),
            Span::raw(object.section.as_deref().unwrap_or("unknown")),
        ]),
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::Cyan)),
            Span::raw(object.kind.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} bytes", object.size)),
        ]),
        Line::from(vec![
            Span::styled("Value: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", object.value)),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", object.target)),
        ]),
        Line::from(vec![
            Span::styled("Target Section: ", Style::default().fg(Color::Cyan)),
            Span::raw(object.target_section.as_deref().unwrap_or("unknown")),
        ]),
    ];

    if let Some(label) = &object.target_label {
        lines.push(Line::from(vec![
            Span::styled("Target Label: ", Style::default().fg(Color::Cyan)),
            Span::raw(label.clone()),
        ]));
    }

    if let Some(file_offset) = app.file_offset_for_address(object_address) {
        lines.push(Line::from(vec![
            Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{file_offset:#x}")),
        ]));
        if let Some(bytes) = app.bytes_at_file_offset(file_offset, object.size) {
            let hex = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(Line::from(vec![
                Span::styled("Bytes: ", Style::default().fg(Color::Cyan)),
                Span::raw(hex),
            ]));
        }
    }

    let outgoing_xrefs: Vec<_> = app
        .xrefs
        .iter()
        .filter(|xref| xref.from == object_address && xref.to == target_address)
        .take(8)
        .collect();
    if !outgoing_xrefs.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Outgoing Xrefs:",
            Style::default().fg(Color::Cyan),
        )]));
        for xref in outgoing_xrefs {
            lines.push(Line::from(format!(
                "  {} -> {} {}",
                xref.kind(),
                xref.to,
                xref.label.as_deref().unwrap_or("")
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter follows executable targets or opens Hex Dump",
        Style::default().fg(Color::Green),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Data Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_relocations(f: &mut Frame, app: &mut App, area: Rect) {
    if app.analysis.relocations.is_empty() {
        let paragraph = Paragraph::new("No relocations found")
            .block(Block::default().borders(Borders::ALL).title("Relocations"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(area);

    let relocation_items: Vec<ListItem> = app
        .analysis
        .relocations
        .iter()
        .enumerate()
        .map(|(idx, relocation)| {
            let style = if app.selected_relocation == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let section = relocation.section.as_deref().unwrap_or("unknown");
            let symbol = relocation.symbol.as_deref().unwrap_or("");

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:#014x}", relocation.address),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!(" {:<12} ", truncate_for_display(section, 12)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:<12} ", truncate_for_display(&relocation.kind, 12)),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(truncate_for_display(symbol, 48), style),
            ]))
        })
        .collect();

    let relocation_list = List::new(relocation_items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Relocations: {} entries",
            app.analysis.relocations.len()
        )))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(relocation_list, chunks[0], &mut app.relocation_list_state);
    render_relocation_details(f, app, chunks[1]);
}

fn render_relocation_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(relocation_idx) = app.selected_relocation else {
        let paragraph = Paragraph::new("No relocation selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Relocation Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };

    let Some(relocation) = app.analysis.relocations.get(relocation_idx) else {
        return;
    };

    let address = Address(relocation.address);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", relocation.address)),
        ]),
        Line::from(vec![
            Span::styled("Section: ", Style::default().fg(Color::Cyan)),
            Span::raw(relocation.section.as_deref().unwrap_or("unknown")),
        ]),
        Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::Cyan)),
            Span::raw(relocation.source.clone()),
        ]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} ({})", relocation.kind, relocation.type_id)),
        ]),
    ];

    if let Some(symbol) = &relocation.symbol {
        lines.push(Line::from(vec![
            Span::styled("Symbol: ", Style::default().fg(Color::Cyan)),
            Span::raw(symbol.clone()),
        ]));
    }

    if let Some(addend) = relocation.addend {
        lines.push(Line::from(vec![
            Span::styled("Addend: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{addend:#x}")),
        ]));
    }

    if let Some(section) = app.section_for_address(address) {
        lines.push(Line::from(vec![
            Span::styled("Permissions: ", Style::default().fg(Color::Cyan)),
            Span::raw(section.permissions()),
        ]));
    }
    if let Some(file_offset) = app.file_offset_for_address(address) {
        lines.push(Line::from(vec![
            Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{file_offset:#x}")),
        ]));
        if let Some(bytes) = app.bytes_at_file_offset(file_offset, 8) {
            let hex = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(Line::from(vec![
                Span::styled("Bytes: ", Style::default().fg(Color::Cyan)),
                Span::raw(hex),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter opens relocation bytes or containing instruction",
        Style::default().fg(Color::Green),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Relocation Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
fn render_sections(f: &mut Frame, app: &mut App, area: Rect) {
    if app.sections.is_empty() {
        let paragraph = Paragraph::new("No sections found")
            .block(Block::default().borders(Borders::ALL).title("Sections"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    let section_items: Vec<ListItem> = app
        .sections
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            let style = if app.selected_section == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{:<12}", section.name), style),
                    Span::styled(
                        format!(" {} ", section.permissions()),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        format!("{:#014x}", section.address),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(Span::styled(
                    format!(
                        "  virt: {:#x}  file: {:#x} @ {:#x}",
                        section.virtual_size, section.file_size, section.file_offset
                    ),
                    Style::default().fg(Color::Gray),
                )),
            ])
        })
        .collect();

    let sections_list = List::new(section_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Sections: {}", app.sections.len())),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(sections_list, chunks[0], &mut app.section_list_state);
    render_section_details(f, app, chunks[1]);
}

fn render_section_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(section_idx) = app.selected_section else {
        let paragraph = Paragraph::new("No section selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Section Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };
    let Some(section) = app.sections.get(section_idx) else {
        return;
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(section.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("Permissions: ", Style::default().fg(Color::Cyan)),
            Span::raw(section.permissions()),
        ]),
        Line::from(vec![
            Span::styled("VA Range: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{:#x} - {:#x}",
                section.address,
                section.end_address()
            )),
        ]),
        Line::from(vec![
            Span::styled("Virtual Size: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", section.virtual_size)),
        ]),
        Line::from(vec![
            Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", section.file_offset)),
        ]),
        Line::from(vec![
            Span::styled("File Size: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", section.file_size)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Enter to jump to section start",
            Style::default().fg(Color::Green),
        )]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Section Details"),
        )
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

            let label = xref
                .label
                .as_ref()
                .map(|label| format!("  {}", label))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<8}", xref.kind()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(format!(" {} ", xref.from), Style::default().fg(Color::Cyan)),
                Span::raw("-> "),
                Span::styled(format!("{}", xref.to), style),
                Span::styled(label, Style::default().fg(Color::Green)),
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
        Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Cyan)),
            Span::raw(xref.label.as_deref().unwrap_or("unknown")),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Enter to jump to source",
            Style::default().fg(Color::Green),
        )]),
    ];

    if let Some(instruction) = app
        .instructions
        .iter()
        .find(|instruction| instruction.address == xref.from.0)
    {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Source Instruction:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(format!("  {}", instruction)));
    } else if let Some(cfg) = &app.cfg {
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
    }

    if let Some(cfg) = &app.cfg {
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

fn render_bookmarks(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(project) = &app.project else {
        let paragraph = Paragraph::new("No project state available")
            .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    };

    if project.bookmarks.is_empty() {
        let paragraph = Paragraph::new("No bookmarks yet\n\nPress b on an address to add one")
            .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    let bookmark_items: Vec<ListItem> = project
        .bookmarks
        .iter()
        .enumerate()
        .map(|(idx, bookmark)| {
            let style = if app.selected_bookmark == Some(idx) {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let name = app.project_user_name_at(bookmark.address).unwrap_or("");
            let label = bookmark.label.as_deref().unwrap_or(name);
            let comment = app.project_comment_at(bookmark.address).unwrap_or("");

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{:#014x}", bookmark.address),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        if label.is_empty() {
                            "".to_string()
                        } else {
                            format!("  {}", label)
                        },
                        style,
                    ),
                ]),
                Line::from(Span::styled(
                    if comment.is_empty() {
                        "  no comment".to_string()
                    } else {
                        format!("  {}", comment)
                    },
                    Style::default().fg(Color::Gray),
                )),
            ])
        })
        .collect();

    let bookmarks_list = List::new(bookmark_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Bookmarks: {}", project.bookmarks.len())),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(bookmarks_list, chunks[0], &mut app.bookmark_list_state);
    render_bookmark_details(f, app, chunks[1]);
}

fn render_bookmark_details(f: &mut Frame, app: &App, area: Rect) {
    let Some(project) = &app.project else {
        return;
    };
    let Some(bookmark_idx) = app.selected_bookmark else {
        let paragraph = Paragraph::new("No bookmark selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Bookmark Details"),
        );
        f.render_widget(paragraph, area);
        return;
    };
    let Some(bookmark) = project.bookmarks.get(bookmark_idx) else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#014x}", bookmark.address)),
        ]),
        Line::from(vec![
            Span::styled("Label: ", Style::default().fg(Color::Cyan)),
            Span::raw(bookmark.label.as_deref().unwrap_or("none")),
        ]),
    ];

    if let Some(name) = app.project_user_name_at(bookmark.address) {
        lines.push(Line::from(vec![
            Span::styled("User Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(name.to_string()),
        ]));
    }

    if let Some(comment) = app.project_comment_at(bookmark.address) {
        lines.push(Line::from(vec![
            Span::styled("Comment: ", Style::default().fg(Color::Cyan)),
            Span::raw(comment.to_string()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter jumps to bookmark; b removes it",
        Style::default().fg(Color::Green),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Bookmark Details"),
        )
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
        .constraints([Constraint::Percentage(74), Constraint::Percentage(26)])
        .split(area);

    let Some(address) = app.current_address() else {
        let paragraph = Paragraph::new("No address selected")
            .block(Block::default().borders(Borders::ALL).title("Hex View"));
        f.render_widget(paragraph, area);
        return;
    };

    let Some(center_offset) = app.file_offset_for_address(address) else {
        let section = app
            .section_for_address(address)
            .map(|section| section.name.as_str())
            .unwrap_or("unmapped");
        let paragraph = Paragraph::new(format!(
            "Address {:#x} is not backed by file bytes\nSection: {}",
            address.0, section
        ))
        .block(Block::default().borders(Borders::ALL).title("Hex View"))
        .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    };

    let bytes_per_row = 16usize;
    let rows_before = 8usize;
    let total_rows = 18usize;
    let start_offset =
        center_offset.saturating_sub(rows_before * bytes_per_row) / bytes_per_row * bytes_per_row;

    let mut hex_lines = Vec::with_capacity(total_rows);
    let mut detail_lines = Vec::new();

    for row in 0..total_rows {
        let file_offset = start_offset.saturating_add(row * bytes_per_row);
        let Some(bytes) = app.bytes_at_file_offset(file_offset, bytes_per_row) else {
            break;
        };

        let row_va = app
            .sections
            .iter()
            .find_map(|section| {
                let section_start = usize::try_from(section.file_offset).ok()?;
                let section_size = usize::try_from(section.file_size).ok()?;
                let section_end = section_start.checked_add(section_size)?;
                if file_offset >= section_start && file_offset < section_end {
                    Some(section.address + (file_offset - section_start) as u64)
                } else {
                    None
                }
            })
            .unwrap_or(address.0);

        let is_selected_row =
            center_offset >= file_offset && center_offset < file_offset + bytes.len();
        let style = if is_selected_row {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };

        let hex = bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = bytes
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                }
            })
            .collect::<String>();

        hex_lines.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{row_va:#014x}  "),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{file_offset:#08x}  "),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(format!("{hex:<47}  "), style),
            Span::styled(ascii, style),
        ])));
    }

    if let Some(section) = app.section_for_address(address) {
        detail_lines.push(Line::from(vec![
            Span::styled("Address: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", address.0)),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("File Offset: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:#x}", center_offset)),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Section: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} {}", section.name, section.permissions())),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("VA Range: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{:#x} - {:#x}",
                section.address,
                section.end_address()
            )),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("File Range: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{:#x} - {:#x}",
                section.file_offset,
                section.file_offset.saturating_add(section.file_size)
            )),
        ]));
    }

    let hex_list = List::new(hex_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Hex View: VA | File | Bytes | ASCII"),
    );

    f.render_widget(hex_list, chunks[0]);

    let details = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Mapped Address"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(details, chunks[1]);
}

// Update render_status_bar to show search mode
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.rename_mode {
        format!("RENAME: '{}' | Enter saves, ESC cancels", app.rename_query)
    } else if app.comment_mode {
        format!(
            "COMMENT: '{}' | Enter saves, ESC cancels",
            app.comment_query
        )
    } else if app.address_jump_mode {
        format!(
            "GOTO: '{}' | Enter jumps, ESC cancels | Back: {} Forward: {}",
            app.address_jump_query,
            app.back_stack.len(),
            app.forward_stack.len()
        )
    } else if app.search_mode {
        format!(
            "SEARCH: '{}' | Instructions: {} | Results: {} | Selected: {} | Tab: {:?} | ESC exits, n/N cycles after search",
            app.search_query,
            app.filtered_instructions.len(),
            app.search_matches.len(),
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
            "Overview | Instructions: {} | Functions: {} | Calls: {} | Imports: {} | Exports: {} | Symbols: {} | Names: {} | Strings: {} | Data: {} | Relocs: {} | Unwind: {} | Sections: {} | Xrefs: {} | Bookmarks: {} | Project: {} | Selected: {} | Tab: {:?} | O overview, I imports, E exports, Y symbols, L relocs, R rename, ; comment, b bookmark, g goto, h help, q quit{}",
            app.instructions.len(),
            app.functions.len(),
            app.call_graph.as_ref().map_or(0, |graph| graph.total_edge_count()),
            app.analysis.imports.len(),
            app.analysis.exports.len(),
            app.analysis.symbols.len(),
            app.names.len(),
            app.analysis.strings.len(),
            app.analysis.data_objects.len(),
            app.analysis.relocations.len(),
            app.analysis.function_ranges.len(),
            app.sections.len(),
            app.xrefs.len(),
            app.bookmark_count(),
            if app.project_dirty { "dirty" } else { "clean" },
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
            Span::styled("  1-9/0      ", Style::default().fg(Color::Green)),
            Span::raw("- Select numbered tab"),
        ]),
        Line::from(vec![
            Span::styled("  O          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Overview"),
        ]),
        Line::from(vec![
            Span::styled("  I          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Imports"),
        ]),
        Line::from(vec![
            Span::styled("  E          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Exports"),
        ]),
        Line::from(vec![
            Span::styled("  Y          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Symbols"),
        ]),
        Line::from(vec![
            Span::styled("  S          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Strings"),
        ]),
        Line::from(vec![
            Span::styled("  D          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Data"),
        ]),
        Line::from(vec![
            Span::styled("  L          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to Relocations"),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", Style::default().fg(Color::Green)),
            Span::raw(
                "- Jump from overview/function/call/import/export/symbol/name/string/data/relocation/section/xref/bookmark/graph block",
            ),
        ]),
        Line::from(vec![
            Span::styled("  g          ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to address"),
        ]),
        Line::from(vec![
            Span::styled("  u/r        ", Style::default().fg(Color::Green)),
            Span::raw("- Navigation back/forward"),
        ]),
        Line::from(vec![
            Span::styled("  n/N        ", Style::default().fg(Color::Green)),
            Span::raw("- Next/previous search result"),
        ]),
        Line::from(vec![
            Span::styled("  R          ", Style::default().fg(Color::Green)),
            Span::raw("- Rename selected address"),
        ]),
        Line::from(vec![
            Span::styled("  ;          ", Style::default().fg(Color::Green)),
            Span::raw("- Edit comment at selected address"),
        ]),
        Line::from(vec![
            Span::styled("  b          ", Style::default().fg(Color::Green)),
            Span::raw("- Toggle bookmark at selected address"),
        ]),
        Line::from(vec![
            Span::styled("  f          ", Style::default().fg(Color::Green)),
            Span::raw("- Toggle Graph View function scope"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tabs:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  O ", Style::default().fg(Color::Yellow)),
            Span::raw("- Overview"),
        ]),
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
            Span::raw("- Call Graph"),
        ]),
        Line::from(vec![
            Span::styled("  I ", Style::default().fg(Color::Yellow)),
            Span::raw("- Imports"),
        ]),
        Line::from(vec![
            Span::styled("  E ", Style::default().fg(Color::Yellow)),
            Span::raw("- Exports"),
        ]),
        Line::from(vec![
            Span::styled("  Y ", Style::default().fg(Color::Yellow)),
            Span::raw("- Symbols"),
        ]),
        Line::from(vec![
            Span::styled("  4 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Names"),
        ]),
        Line::from(vec![
            Span::styled("  S ", Style::default().fg(Color::Yellow)),
            Span::raw("- Strings"),
        ]),
        Line::from(vec![
            Span::styled("  D ", Style::default().fg(Color::Yellow)),
            Span::raw("- Data"),
        ]),
        Line::from(vec![
            Span::styled("  L ", Style::default().fg(Color::Yellow)),
            Span::raw("- Relocations"),
        ]),
        Line::from(vec![
            Span::styled("  5 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Sections"),
        ]),
        Line::from(vec![
            Span::styled("  6 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Xrefs"),
        ]),
        Line::from(vec![
            Span::styled("  7 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Bookmarks"),
        ]),
        Line::from(vec![
            Span::styled("  8 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Control Flow"),
        ]),
        Line::from(vec![
            Span::styled("  9 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Graph Analysis"),
        ]),
        Line::from(vec![
            Span::styled("  0 ", Style::default().fg(Color::Yellow)),
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
