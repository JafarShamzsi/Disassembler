use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::graph::{ControlFlowGraph, EdgeType};
use crate::graph_view::{BlockType, GraphView};

/// A custom widget that renders the control flow graph as text blocks
pub struct GraphWidget<'a> {
    pub graph_view: &'a GraphView,
    pub cfg: &'a ControlFlowGraph,
}

impl<'a> GraphWidget<'a> {
    pub fn new(graph_view: &'a GraphView, cfg: &'a ControlFlowGraph) -> Self {
        Self { graph_view, cfg }
    }

    fn world_to_screen(&self, world_x: f64, world_y: f64, _area: Rect) -> (i16, i16) {
        let (screen_x, screen_y) = self.graph_view.viewport.world_to_screen(world_x, world_y);

        // Adjust for the rendering area
        let adjusted_x = screen_x as i16;
        let adjusted_y = screen_y as i16;

        (adjusted_x, adjusted_y)
    }

    fn get_block_colors(&self, block_type: &BlockType, is_selected: bool) -> (Style, Style, Style) {
        let (border_color, bg_color, text_color) = match block_type {
            BlockType::Entry => (Color::Green, Color::Rgb(0, 60, 0), Color::White),
            BlockType::Exit => (Color::Red, Color::Rgb(60, 0, 0), Color::White),
            BlockType::Conditional => (Color::Yellow, Color::Rgb(60, 60, 0), Color::White),
            BlockType::Call => (Color::Cyan, Color::Rgb(0, 40, 60), Color::White),
            BlockType::Loop => (Color::Magenta, Color::Rgb(60, 0, 60), Color::White),
            BlockType::Normal => (Color::Gray, Color::Black, Color::White),
            BlockType::Data => (Color::Blue, Color::Rgb(0, 0, 60), Color::LightBlue),
        };

        if is_selected {
            (
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                Style::default().bg(Color::DarkGray),
                Style::default().fg(Color::LightYellow).bg(Color::DarkGray),
            )
        } else {
            (
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
                Style::default().bg(bg_color),
                Style::default().fg(text_color).bg(bg_color),
            )
        }
    }

    fn draw_block(
        &self,
        buf: &mut Buffer,
        area: Rect,
        block_layout: &crate::graph_view::BlockLayout,
        is_selected: bool,
    ) {
        let (screen_x, screen_y) =
            self.world_to_screen(block_layout.position.x, block_layout.position.y, area);

        // Calculate block dimensions, making sure they fit in the area
        let block_width = (block_layout.size.x as u16).min(40).min(area.width);
        let block_height = (block_layout.size.y as u16).min(15).min(area.height);

        // Skip blocks that are completely outside the visible area
        if screen_x < -(block_width as i16)
            || screen_x > area.width as i16
            || screen_y < -(block_height as i16)
            || screen_y > area.height as i16
        {
            return;
        }

        // Calculate actual position, ensuring it's within bounds
        let block_x = if screen_x < 0 {
            0
        } else if screen_x + block_width as i16 > area.width as i16 {
            area.width.saturating_sub(block_width)
        } else {
            screen_x as u16
        };

        let block_y = if screen_y < 0 {
            0
        } else if screen_y + block_height as i16 > area.height as i16 {
            area.height.saturating_sub(block_height)
        } else {
            screen_y as u16
        };

        // Create the block area
        let block_rect = Rect {
            x: area.x + block_x,
            y: area.y + block_y,
            width: block_width,
            height: block_height,
        };

        // Get the block data
        if let Some(block) = self.cfg.blocks.get(&block_layout.address) {
            // Get colors based on block type
            let (border_style, bg_style, text_style) =
                self.get_block_colors(&block_layout.block_type, is_selected);

            // Create block content
            let mut lines = vec![];

            // Add header with address and block type
            let block_type_text = match block_layout.block_type {
                BlockType::Entry => " [ENTRY]",
                BlockType::Exit => " [EXIT]",
                BlockType::Conditional => " [COND]",
                BlockType::Call => " [CALL]",
                BlockType::Loop => " [LOOP]",
                BlockType::Normal => "",
                BlockType::Data => " [DATA]",
            };
            let header_text = format!(" {}{} ", block_layout.address, block_type_text);
            let header_padding =
                "─".repeat(block_width.saturating_sub(header_text.len() as u16) as usize);
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "┌{}┐",
                    if header_padding.is_empty() {
                        header_text
                    } else {
                        format!("{}{}", header_text, header_padding)
                    }
                ),
                border_style,
            )]));

            // Add instructions (limit to available height - 2 for borders)
            let max_instructions = block_height.saturating_sub(2) as usize;
            for instr in block.instructions.iter().take(max_instructions) {
                let instruction_text =
                    format!("{}: {} {}", instr.address, instr.mnemonic, instr.operands);

                // Truncate if too long for the block width
                let content_width = block_width.saturating_sub(2) as usize;
                let truncated = if instruction_text.len() > content_width {
                    format!("{}…", &instruction_text[..content_width.saturating_sub(1)])
                } else {
                    instruction_text
                };

                let padded = format!("│{:<width$}│", truncated, width = content_width);

                lines.push(Line::from(vec![Span::styled(padded, text_style)]));
            }

            // Add bottom border
            let bottom_border = format!("└{}┘", "─".repeat(block_width.saturating_sub(2) as usize));
            lines.push(Line::from(vec![Span::styled(bottom_border, border_style)]));

            // Render the block
            let block_paragraph = Paragraph::new(lines).style(bg_style);
            block_paragraph.render(block_rect, buf);
        }
    }

    fn draw_edge_connections(&self, buf: &mut Buffer, area: Rect) {
        // Draw simple connections between blocks
        for edge in &self.graph_view.edges {
            if let (Some(from_block), Some(to_block)) = (
                self.graph_view.blocks.get(&edge.from),
                self.graph_view.blocks.get(&edge.to),
            ) {
                let (from_x, from_y) = self.world_to_screen(
                    from_block.position.x + from_block.size.x / 2.0,
                    from_block.position.y + from_block.size.y,
                    area,
                );
                let (to_x, to_y) = self.world_to_screen(
                    to_block.position.x + to_block.size.x / 2.0,
                    to_block.position.y,
                    area,
                );

                // Get edge styling based on type
                let (color, line_char, arrow_char) = match edge.edge_type {
                    EdgeType::ConditionalTrue => (Color::Green, "│", "▼"),
                    EdgeType::ConditionalFalse => (Color::Red, "┊", "▽"),
                    EdgeType::Call => (Color::Cyan, "║", "▼"),
                    EdgeType::Return => (Color::Yellow, "┃", "△"),
                    _ => (Color::Gray, "│", "▼"),
                };

                // Draw a simple vertical line if blocks are roughly aligned
                if (from_x - to_x).abs() <= 2 && from_y < to_y {
                    for y in (from_y + 1)..to_y {
                        if y >= 0
                            && y < area.height as i16
                            && from_x >= 0
                            && from_x < area.width as i16
                        {
                            let screen_pos_x = (area.x as i16 + from_x) as u16;
                            let screen_pos_y = (area.y as i16 + y) as u16;

                            if screen_pos_y < buf.area.height && screen_pos_x < buf.area.width {
                                buf[(screen_pos_x, screen_pos_y)]
                                    .set_symbol(line_char)
                                    .set_fg(color);
                            }
                        }
                    }

                    // Draw arrow at the end
                    if to_y >= 0
                        && to_y < area.height as i16
                        && to_x >= 0
                        && to_x < area.width as i16
                    {
                        let screen_pos_x = (area.x as i16 + to_x) as u16;
                        let screen_pos_y = (area.y as i16 + to_y) as u16;

                        if screen_pos_y < buf.area.height && screen_pos_x < buf.area.width {
                            buf[(screen_pos_x, screen_pos_y)]
                                .set_symbol(arrow_char)
                                .set_fg(color);
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Widget for GraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].reset();
            }
        }

        // Draw connections first (so they appear behind blocks)
        self.draw_edge_connections(buf, area);

        // Draw blocks
        for (addr, block_layout) in &self.graph_view.blocks {
            let is_selected = self.graph_view.selected_block == Some(*addr);
            self.draw_block(buf, area, block_layout, is_selected);
        }
    }
}

pub struct GraphRenderer {}

impl Default for GraphRenderer {
    fn default() -> Self {
        Self {}
    }
}

impl GraphRenderer {
    pub fn render_graph_view(
        &mut self,
        f: &mut Frame,
        area: Rect,
        graph_view: &mut GraphView,
        cfg: &ControlFlowGraph,
    ) {
        // Update viewport size to match the rendering area
        graph_view.viewport.update_size(area.width, area.height);

        let graph_widget = GraphWidget::new(graph_view, cfg);

        let block_with_title = Block::default()
            .borders(Borders::ALL)
            .title("Control Flow Graph");

        let inner_area = block_with_title.inner(area);
        f.render_widget(block_with_title, area);
        f.render_widget(graph_widget, inner_area);

        // Render controls overlay
        self.render_graph_controls(f, area, graph_view);

        // Render legend overlay
        self.render_legend(f, area);
    }

    fn render_graph_controls(&self, f: &mut Frame, area: Rect, graph_view: &GraphView) {
        // Create a small overlay at the bottom for controls
        let control_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(4),
            width: area.width,
            height: 3,
        };

        let selected_text = if let Some(addr) = graph_view.selected_block {
            let block_type_text = if let Some(block_layout) = graph_view.blocks.get(&addr) {
                match block_layout.block_type {
                    BlockType::Entry => " [ENTRY]",
                    BlockType::Exit => " [EXIT]",
                    BlockType::Conditional => " [COND]",
                    BlockType::Call => " [CALL]",
                    BlockType::Loop => " [LOOP]",
                    BlockType::Normal => " [NORMAL]",
                    BlockType::Data => " [DATA]",
                }
            } else {
                ""
            };
            format!("Selected: {}{}", addr, block_type_text)
        } else {
            "No selection".to_string()
        };

        let controls_text = vec![
            Line::from(vec![
                Span::styled(
                    "Graph Controls: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Arrow Keys", Style::default().fg(Color::Green)),
                Span::raw(" = Navigate Blocks | "),
                Span::styled("WASD", Style::default().fg(Color::Green)),
                Span::raw(" = Pan View | "),
                Span::styled("+/-", Style::default().fg(Color::Green)),
                Span::raw(" = Zoom | "),
                Span::styled("C", Style::default().fg(Color::Green)),
                Span::raw(" = Center"),
            ]),
            Line::from(vec![
                Span::styled(&selected_text, Style::default().fg(Color::Cyan)),
                Span::raw(" | "),
                Span::styled("Blocks: ", Style::default().fg(Color::Cyan)),
                Span::raw(graph_view.blocks.len().to_string()),
                Span::raw(" | "),
                Span::styled(
                    "Legend in top-right corner",
                    Style::default().fg(Color::LightBlue),
                ),
            ]),
        ];

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().bg(Color::Black));

        f.render_widget(Clear, control_area);
        f.render_widget(controls, control_area);
    }

    pub fn render_block_details(
        &self,
        f: &mut Frame,
        area: Rect,
        graph_view: &GraphView,
        cfg: &ControlFlowGraph,
    ) {
        let content = if let Some(selected_addr) = graph_view.selected_block {
            if let Some(block) = cfg.blocks.get(&selected_addr) {
                let mut lines = vec![Line::from(vec![
                    Span::styled(
                        "Block ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}", selected_addr),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])];

                // Add block type information
                if let Some(block_layout) = graph_view.blocks.get(&selected_addr) {
                    let (type_name, type_color, description) = match block_layout.block_type {
                        BlockType::Entry => ("ENTRY", Color::Green, "Program entry point"),
                        BlockType::Exit => ("EXIT", Color::Red, "Program exit or return"),
                        BlockType::Conditional => {
                            ("CONDITIONAL", Color::Yellow, "Branching control flow")
                        }
                        BlockType::Call => ("CALL", Color::Cyan, "Function call"),
                        BlockType::Loop => ("LOOP", Color::Magenta, "Loop structure"),
                        BlockType::Normal => ("NORMAL", Color::Gray, "Sequential execution"),
                        BlockType::Data => ("DATA", Color::Blue, "Data or constants"),
                    };

                    lines.push(Line::from(vec![
                        Span::styled("Type: ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            type_name,
                            Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" - {}", description),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                }

                lines.push(Line::from(""));

                // Add instructions
                for instr in &block.instructions {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}: ", instr.address),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            &instr.mnemonic,
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(&instr.operands, Style::default().fg(Color::White)),
                    ]));
                }

                lines.push(Line::from(""));

                // Add successor information
                if !block.successors.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "Successors: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    for successor in &block.successors {
                        // Find edge type
                        let edge_type = cfg
                            .edges
                            .iter()
                            .find(|e| e.from == selected_addr && e.to == *successor)
                            .map(|e| &e.edge_type);

                        let (edge_text, edge_color) = match edge_type {
                            Some(EdgeType::ConditionalTrue) => (" [TRUE]", Color::Green),
                            Some(EdgeType::ConditionalFalse) => (" [FALSE]", Color::Red),
                            Some(EdgeType::Call) => (" [CALL]", Color::Cyan),
                            Some(EdgeType::Return) => (" [RET]", Color::Yellow),
                            _ => ("", Color::Gray),
                        };

                        lines.push(Line::from(vec![
                            Span::raw("  → "),
                            Span::styled(
                                format!("{}", successor),
                                Style::default().fg(Color::Green),
                            ),
                            Span::styled(edge_text, Style::default().fg(edge_color)),
                        ]));
                    }
                }

                // Add predecessor information
                if !block.predecessors.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Predecessors: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    for predecessor in &block.predecessors {
                        lines.push(Line::from(vec![
                            Span::raw("  ← "),
                            Span::styled(
                                format!("{}", predecessor),
                                Style::default().fg(Color::Blue),
                            ),
                        ]));
                    }
                }

                lines
            } else {
                vec![Line::from("Block not found")]
            }
        } else {
            vec![
                Line::from("No block selected"),
                Line::from(""),
                Line::from("Use arrow keys to navigate between blocks"),
            ]
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Block Details"),
            )
            .style(Style::default().fg(Color::White))
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    fn render_legend(&self, f: &mut Frame, area: Rect) {
        // Create a legend area at the top right
        let legend_width = 18;
        let legend_height = 10;
        let legend_area = Rect {
            x: area.x + area.width.saturating_sub(legend_width + 1),
            y: area.y + 1,
            width: legend_width,
            height: legend_height,
        };

        let legend_items = vec![
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Entry Block"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Exit Block"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Conditional"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Call Block"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Loop Block"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Normal Block"),
            ]),
            Line::from(vec![
                Span::styled(
                    "■",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Data Block"),
            ]),
        ];

        let legend = Paragraph::new(legend_items)
            .block(Block::default().borders(Borders::ALL).title("Legend"))
            .style(Style::default().bg(Color::Black).fg(Color::White));

        f.render_widget(Clear, legend_area);
        f.render_widget(legend, legend_area);
    }
}
