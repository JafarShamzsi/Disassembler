use crossterm::{
    cursor::MoveTo,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    style::ResetColor,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

use super::app::App;
use super::input::run_app;
use crate::arch::x86::Instruction;
use crate::graph::ControlFlowGraph;
use crate::parser::BinaryAnalysis;

struct TerminalCleanupGuard;

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        let _ = restore_terminal_state();
    }
}

pub fn run_tui(
    instructions: Vec<Instruction>,
    cfg: Option<ControlFlowGraph>,
    analysis: BinaryAnalysis,
) -> Result<(), Box<dyn std::error::Error>> {
    // AGGRESSIVE: Reset terminal state before we even start
    emergency_terminal_reset();

    // Setup signal handler for proper cleanup on Ctrl+C
    setup_panic_hook();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let _terminal_cleanup = TerminalCleanupGuard;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(instructions, cfg, analysis);
    let app_result = run_app(&mut terminal, app);

    // Comprehensive terminal cleanup - ensure we always restore terminal state
    cleanup_terminal(&mut terminal)?;

    // AGGRESSIVE: Force complete terminal reset after TUI
    emergency_terminal_reset();

    app_result?;
    Ok(())
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
        let _ = restore_terminal_state();
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
    restore_terminal_state()?;

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

fn restore_terminal_state() -> io::Result<()> {
    let _ = disable_raw_mode();
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        MoveTo(0, 0),
        ResetColor
    )?;
    use std::io::Write;
    io::stdout().flush()
}
