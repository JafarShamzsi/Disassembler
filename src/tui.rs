use crossterm::{
    cursor::MoveTo,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    style::ResetColor,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear as RatatuiClear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::arch::x86::Instruction;
use crate::graph::{Address, ControlFlowGraph, FunctionSummary};
use crate::graph_renderer::GraphRenderer;
use crate::graph_view::{GraphView, NavigationDirection};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Instructions,
    Functions,
    ControlFlow,
    GraphView,
    HexDump,
}

pub struct App {
    pub instructions: Vec<Instruction>,
    pub cfg: Option<ControlFlowGraph>,
    pub current_tab: Tab,
    pub instruction_list_state: ListState,
    pub function_list_state: ListState,
    pub selected_instruction: Option<usize>,
    pub selected_function: Option<usize>,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub filtered_instructions: Vec<usize>,
    pub instruction_display_cache: Vec<String>,
    pub last_search_query: String,
    pub functions: Vec<FunctionSummary>,
    pub graph_view: GraphView,
    pub graph_renderer: GraphRenderer,
}

impl App {
    pub fn new(instructions: Vec<Instruction>, cfg: Option<ControlFlowGraph>) -> Self {
        let instruction_display_cache: Vec<String> = instructions
            .iter()
            .map(|instr| format!("{:#08x}: {}", instr.address, instr.text))
            .collect();

        let mut graph_view = GraphView::new();
        let functions = cfg
            .as_ref()
            .map(ControlFlowGraph::function_summaries)
            .unwrap_or_default();

        // Initialize graph view with CFG if available
        if let Some(ref cfg) = cfg {
            graph_view.build_from_cfg(cfg);
        }

        let mut app = Self {
            instructions,
            cfg,
            current_tab: Tab::Instructions,
            instruction_list_state: ListState::default(),
            function_list_state: ListState::default(),
            selected_instruction: None,
            selected_function: None,
            scroll_offset: 0,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            filtered_instructions: Vec::new(),
            instruction_display_cache,
            last_search_query: String::new(),
            functions,
            graph_view,
            graph_renderer: GraphRenderer::default(),
        };

        if !app.instructions.is_empty() {
            app.instruction_list_state.select(Some(0));
            app.selected_instruction = Some(0);
            app.filtered_instructions = (0..app.instructions.len()).collect();
        }

        if !app.functions.is_empty() {
            app.function_list_state.select(Some(0));
            app.selected_function = Some(0);
        }

        app
    }

    pub fn next_instruction(&mut self) {
        if self.filtered_instructions.is_empty() {
            return;
        }

        if let Some(current_pos) = self.instruction_list_state.selected() {
            let next_pos =
                (current_pos + 1).min(self.filtered_instructions.len().saturating_sub(1));
            self.instruction_list_state.select(Some(next_pos));

            if let Some(&instruction_idx) = self.filtered_instructions.get(next_pos) {
                if instruction_idx < self.instructions.len() {
                    self.selected_instruction = Some(instruction_idx);
                }
            }
        }
    }

    pub fn previous_instruction(&mut self) {
        if self.filtered_instructions.is_empty() {
            return;
        }

        if let Some(current_pos) = self.instruction_list_state.selected() {
            let prev_pos = current_pos.saturating_sub(1);
            self.instruction_list_state.select(Some(prev_pos));

            if let Some(&instruction_idx) = self.filtered_instructions.get(prev_pos) {
                if instruction_idx < self.instructions.len() {
                    self.selected_instruction = Some(instruction_idx);
                }
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::Functions,
            Tab::Functions => Tab::ControlFlow,
            Tab::ControlFlow => Tab::GraphView,
            Tab::GraphView => Tab::HexDump,
            Tab::HexDump => Tab::Instructions,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::HexDump,
            Tab::Functions => Tab::Instructions,
            Tab::ControlFlow => Tab::Functions,
            Tab::GraphView => Tab::ControlFlow,
            Tab::HexDump => Tab::GraphView,
        };
    }

    pub fn next_function(&mut self) {
        if self.functions.is_empty() {
            return;
        }

        if let Some(current) = self.function_list_state.selected() {
            let next = (current + 1).min(self.functions.len().saturating_sub(1));
            self.function_list_state.select(Some(next));
            self.selected_function = Some(next);
        }
    }

    pub fn previous_function(&mut self) {
        if self.functions.is_empty() {
            return;
        }

        if let Some(current) = self.function_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.function_list_state.select(Some(previous));
            self.selected_function = Some(previous);
        }
    }

    pub fn jump_to_selected_function(&mut self) {
        let Some(function_idx) = self.selected_function else {
            return;
        };
        let Some(function) = self.functions.get(function_idx) else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_by_address(function.entry) else {
            return;
        };

        self.selected_instruction = Some(instruction_idx);
        if let Some(filtered_idx) = self
            .filtered_instructions
            .iter()
            .position(|idx| *idx == instruction_idx)
        {
            self.instruction_list_state.select(Some(filtered_idx));
        }
        self.current_tab = Tab::Instructions;
    }

    fn find_instruction_by_address(&self, address: Address) -> Option<usize> {
        self.instructions
            .binary_search_by_key(&address.0, |instruction| instruction.address)
            .ok()
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        // Reset to show all instructions
        self.filtered_instructions = (0..self.instructions.len()).collect();
    }

    pub fn update_search(&mut self, query: String) {
        self.search_query = query;
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        // Only rebuild if search query actually changed
        if self.search_query == self.last_search_query {
            return;
        }

        self.last_search_query = self.search_query.clone();

        if self.search_query.is_empty() {
            self.filtered_instructions = (0..self.instructions.len()).collect();
        } else {
            // Pre-lowercase search query once
            let search_lower = self.search_query.to_lowercase();

            self.filtered_instructions = (0..self.instructions.len())
                .filter(|&i| {
                    // Use cached display strings instead of formatting
                    self.instruction_display_cache[i]
                        .to_lowercase()
                        .contains(&search_lower)
                })
                .collect();
        }

        // Update selection safely
        if let Some(selected) = self.selected_instruction {
            if !self.filtered_instructions.contains(&selected) {
                if let Some(&first) = self.filtered_instructions.first() {
                    self.selected_instruction = Some(first);
                    self.instruction_list_state.select(Some(0));
                } else {
                    self.selected_instruction = None;
                    self.instruction_list_state.select(None);
                }
            }
        }
    }
}

pub fn run_tui(
    instructions: Vec<Instruction>,
    cfg: Option<ControlFlowGraph>,
) -> Result<(), Box<dyn std::error::Error>> {
    // AGGRESSIVE: Reset terminal state before we even start
    emergency_terminal_reset();

    // Setup signal handler for proper cleanup on Ctrl+C
    setup_panic_hook();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(instructions, cfg);
    let _res = run_app(&mut terminal, app);

    // Comprehensive terminal cleanup - ensure we always restore terminal state
    cleanup_terminal(&mut terminal)?;

    // AGGRESSIVE: Force complete terminal reset after TUI
    emergency_terminal_reset();

    // Force exit to prevent any background threads
    std::process::exit(0);
}

fn emergency_terminal_reset() {
    // Nuclear option: completely reset terminal state
    use std::io::Write;

    // Force disable raw mode
    let _ = disable_raw_mode();

    // Send every possible terminal reset sequence
    let _ = io::stdout().write_all(b"\x1b[?1049l"); // Exit alternate screen
    let _ = io::stdout().write_all(b"\x1b[?25h"); // Show cursor
    let _ = io::stdout().write_all(b"\x1b[0m"); // Reset all attributes
    let _ = io::stdout().write_all(b"\x1bc"); // Full terminal reset
    let _ = io::stdout().write_all(b"\x1b[!p"); // Soft terminal reset
    let _ = io::stdout().write_all(b"\x1b[?1000l"); // Disable mouse tracking
    let _ = io::stdout().write_all(b"\x1b[?1002l"); // Disable button event mouse tracking
    let _ = io::stdout().write_all(b"\x1b[?1003l"); // Disable any motion mouse tracking
    let _ = io::stdout().write_all(b"\x1b[?1006l"); // Disable extended mouse tracking
    let _ = io::stdout().write_all(b"\x1b[2J"); // Clear entire screen
    let _ = io::stdout().write_all(b"\x1b[H"); // Move cursor to home

    // Force flush everything
    let _ = io::stdout().flush();

    // Also try crossterm reset
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        MoveTo(0, 0),
        ResetColor
    );

    // Additional separation
    println!("\n\n");
}

fn setup_panic_hook() {
    // Ensure terminal is reset even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        // Force reset terminal state
        let _ = execute!(
            io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        );
        use std::io::Write;
        let _ = io::stdout().flush();
        original_hook(panic_info);
    }));
}

fn cleanup_terminal<B: Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Restore terminal state step by step with error handling
    if let Err(e) = disable_raw_mode() {
        eprintln!("Warning: Failed to disable raw mode: {}", e);
    }

    // Use stdout directly for terminal commands
    if let Err(e) = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture) {
        eprintln!("Warning: Failed to reset terminal: {}", e);
    }

    if let Err(e) = terminal.show_cursor() {
        eprintln!("Warning: Failed to show cursor: {}", e);
    }

    // Force flush output and clear screen
    use std::io::Write;
    if let Err(e) = io::stdout().flush() {
        eprintln!("Warning: Failed to flush stdout: {}", e);
    }

    // Print a few newlines to separate from any remaining output
    println!("\n\n");

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
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
                    if app.search_mode {
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
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app.current_tab == Tab::GraphView {
                                    if let Some(ref cfg) = app.cfg {
                                        app.graph_view
                                            .move_selection(cfg, NavigationDirection::Down);
                                    }
                                } else if app.current_tab == Tab::Functions {
                                    app.next_function();
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
                                } else {
                                    app.previous_instruction();
                                }
                            }
                            KeyCode::Enter if app.current_tab == Tab::Functions => {
                                app.jump_to_selected_function();
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
                            KeyCode::Char('3') => app.current_tab = Tab::ControlFlow,
                            KeyCode::Char('4') => app.current_tab = Tab::GraphView,
                            KeyCode::Char('5') => app.current_tab = Tab::HexDump,
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

fn ui(f: &mut Frame, app: &mut App) {
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
        Tab::ControlFlow => 2,
        Tab::GraphView => 3,
        Tab::HexDump => 4,
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
    let status = if app.search_mode {
        format!(
            "SEARCH: '{}' | Instructions: {} | Selected: {} | Tab: {:?} | ESC to exit search",
            app.search_query,
            app.filtered_instructions.len(),
            app.selected_instruction
                .map_or("None".to_string(), |i| (i + 1).to_string()),
            app.current_tab
        )
    } else {
        format!(
            "Instructions: {} | Functions: {} | Selected: {} | Tab: {:?} | '/' search, Enter jumps from Functions, h help, q quit",
            app.instructions.len(),
            app.functions.len(),
            app.selected_instruction
                .map_or("None".to_string(), |i| (i + 1).to_string()),
            app.current_tab
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
            Span::styled("  1-5        ", Style::default().fg(Color::Green)),
            Span::raw("- Select specific tab"),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", Style::default().fg(Color::Green)),
            Span::raw("- Jump to selected function entry"),
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
            Span::raw("- Control Flow"),
        ]),
        Line::from(vec![
            Span::styled("  4 ", Style::default().fg(Color::Yellow)),
            Span::raw("- Graph Analysis"),
        ]),
        Line::from(vec![
            Span::styled("  5 ", Style::default().fg(Color::Yellow)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Address, Instruction as CfgInstruction};

    fn instruction(address: u64, text: &str, size: usize) -> Instruction {
        Instruction {
            address,
            bytes: vec![0x90; size],
            text: text.to_string(),
        }
    }

    fn cfg_instruction(
        address: u64,
        mnemonic: &str,
        operands: &str,
        size: usize,
    ) -> CfgInstruction {
        CfgInstruction {
            address: Address(address),
            mnemonic: mnemonic.to_string(),
            operands: operands.to_string(),
            bytes: vec![0x90; size],
        }
    }

    #[test]
    fn function_jump_selects_entry_instruction() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
        );

        let callee_idx = app
            .functions
            .iter()
            .position(|function| function.entry == Address(0x2000))
            .unwrap();

        app.current_tab = Tab::Functions;
        app.selected_function = Some(callee_idx);
        app.function_list_state.select(Some(callee_idx));
        app.jump_to_selected_function();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(2));
        assert_eq!(app.instruction_list_state.selected(), Some(2));
    }
}
