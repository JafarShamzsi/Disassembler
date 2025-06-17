use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::disassembler::Instruction;
use crate::graph::ControlFlowGraph;

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Instructions,
    ControlFlow,
    HexDump,
}

pub struct App {
    pub instructions: Vec<Instruction>,
    pub cfg: Option<ControlFlowGraph>,
    pub current_tab: Tab,
    pub instruction_list_state: ListState,
    pub selected_instruction: Option<usize>,
    pub scroll_offset: usize,
    pub show_help: bool,
}

impl App {
    pub fn new(instructions: Vec<Instruction>, cfg: Option<ControlFlowGraph>) -> Self {
        let mut app = Self {
            instructions,
            cfg,
            current_tab: Tab::Instructions,
            instruction_list_state: ListState::default(),
            selected_instruction: None,
            scroll_offset: 0,
            show_help: false,
        };
        
        if !app.instructions.is_empty() {
            app.instruction_list_state.select(Some(0));
            app.selected_instruction = Some(0);
        }
        
        app
    }

    pub fn next_instruction(&mut self) {
        if self.instructions.is_empty() {
            return;
        }
        
        let i = match self.instruction_list_state.selected() {
            Some(i) => {
                if i >= self.instructions.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.instruction_list_state.select(Some(i));
        self.selected_instruction = Some(i);
    }

    pub fn previous_instruction(&mut self) {
        if self.instructions.is_empty() {
            return;
        }
        
        let i = match self.instruction_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.instructions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.instruction_list_state.select(Some(i));
        self.selected_instruction = Some(i);
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::ControlFlow,
            Tab::ControlFlow => Tab::HexDump,
            Tab::HexDump => Tab::Instructions,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::HexDump,
            Tab::ControlFlow => Tab::Instructions,
            Tab::HexDump => Tab::ControlFlow,
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}

pub fn run_tui(instructions: Vec<Instruction>, cfg: Option<ControlFlowGraph>) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(instructions, cfg);
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('h') | KeyCode::F(1) => app.toggle_help(),
                    KeyCode::Down | KeyCode::Char('j') => app.next_instruction(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous_instruction(),
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Char('1') => app.current_tab = Tab::Instructions,
                    KeyCode::Char('2') => app.current_tab = Tab::ControlFlow,
                    KeyCode::Char('3') => app.current_tab = Tab::HexDump,
                    _ => {}
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
        .split(f.size());

    // Render tabs
    let tab_titles: Vec<Line> = vec!["Instructions", "Control Flow", "Hex Dump"]
        .iter()
        .cloned()
        .map(Line::from)
        .collect();
    
    let selected_tab = match app.current_tab {
        Tab::Instructions => 0,
        Tab::ControlFlow => 1,
        Tab::HexDump => 2,
    };
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Sharingan Disassembler"))
        .select(selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));
    
    f.render_widget(tabs, chunks[0]);

    // Render main content based on selected tab
    match app.current_tab {
        Tab::Instructions => render_instructions(f, app, chunks[1]),
        Tab::ControlFlow => render_control_flow(f, app, chunks[1]),
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

    // Instruction list
    let instructions: Vec<ListItem> = app
        .instructions
        .iter()
        .enumerate()
        .map(|(i, instr)| {
            let style = if Some(i) == app.selected_instruction {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            let content = format!("{:#08x}: {}", instr.address, instr.text);
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let instructions_list = List::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Instructions"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Blue))
        .highlight_symbol("> ");

    f.render_stateful_widget(instructions_list, chunks[0], &mut app.instruction_list_state);

    // Instruction details
    render_instruction_details(f, app, chunks[1]);
}

fn render_instruction_details(f: &mut Frame, app: &App, area: Rect) {
    let selected_instr = app.selected_instruction
        .and_then(|i| app.instructions.get(i));

    let content = if let Some(instr) = selected_instr {
        let bytes_str = instr.bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        
        format!(
            "Address: {:#08x}\n\nInstruction: {}\n\nBytes: {}\n\nLength: {} bytes",
            instr.address,
            instr.text,
            bytes_str,
            instr.bytes.len()
        )
    } else {
        "No instruction selected".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_control_flow(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(cfg) = &app.cfg {
        let mut output = String::new();
        
        let mut sorted_blocks: Vec<_> = cfg.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);
        
        for (addr, block) in sorted_blocks {
            output.push_str(&format!("Block {}:\n", addr));
            for instr in &block.instructions {
                output.push_str(&format!("  {}\n", instr));
            }
            
            if !block.successors.is_empty() {
                output.push_str(&format!("  -> Successors: {:?}\n", block.successors));
            }
            output.push('\n');
        }
        
        output
    } else {
        "No control flow graph available\n\nUse --cfg flag to generate CFG".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Control Flow Graph"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_hex_dump(f: &mut Frame, app: &App, area: Rect) {
    let selected_instr = app.selected_instruction
        .and_then(|i| app.instructions.get(i));

    let content = if let Some(instr) = selected_instr {
        let mut output = String::new();
        
        // Create hex dump for the instruction bytes
        for (i, chunk) in instr.bytes.chunks(16).enumerate() {
            let offset = i * 16;
            output.push_str(&format!("{:08x}: ", instr.address + offset as u64));
            
            // Hex bytes
            for (j, byte) in chunk.iter().enumerate() {
                if j == 8 {
                    output.push(' '); // Extra space in the middle
                }
                output.push_str(&format!("{:02x} ", byte));
            }
            
            // Pad if less than 16 bytes
            for j in chunk.len()..16 {
                if j == 8 {
                    output.push(' ');
                }
                output.push_str("   ");
            }
            
            output.push_str(" |");
            
            // ASCII representation
            for byte in chunk {
                let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                };
                output.push(ch);
            }
            
            output.push_str("|\n");
        }
        
        output
    } else {
        "No instruction selected".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Hex Dump"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "Instructions: {} | Selected: {} | Tab: {:?} | Press 'h' for help, 'q' to quit",
        app.instructions.len(),
        app.selected_instruction.map_or("None".to_string(), |i| (i + 1).to_string()),
        app.current_tab
    );

    let paragraph = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 70, f.size());
    
    let help_text = Text::from(vec![
        Line::from("Keyboard Shortcuts:"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/k        - Previous instruction"),
        Line::from("  ↓/j        - Next instruction"),
        Line::from("  Tab        - Next tab"),
        Line::from("  Shift+Tab  - Previous tab"),
        Line::from("  1/2/3      - Select specific tab"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  h/F1       - Toggle this help"),
        Line::from("  q          - Quit application"),
        Line::from(""),
        Line::from("Press 'h' again to close this help"),
    ]);

    f.render_widget(Clear, area);
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    
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