#!/usr/bin/env bash

# Safe wrapper script for the disassembler
# This ensures terminal is always reset properly

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_ROOT/target/debug/disassembler"

echo "Starting Disassembler with Safe Terminal Mode"
echo "================================================"

# Pre-execution cleanup
"$SCRIPT_DIR/emergency_reset.sh" > /dev/null 2>&1

# Store original terminal settings
ORIGINAL_STTY=$(stty -g 2>/dev/null)

# Function to restore terminal
restore_terminal() {
    echo ""
    echo "Restoring terminal state..."

    # Kill any remaining processes
    pkill -9 -f "target/debug/disassembler.*--tui" 2>/dev/null || true

    # Restore original settings if available
    if [ -n "$ORIGINAL_STTY" ]; then
        stty "$ORIGINAL_STTY" 2>/dev/null || true
    fi

    # Emergency reset
    "$SCRIPT_DIR/emergency_reset.sh" > /dev/null 2>&1

    echo "Terminal restored"
}

# Set up trap to always restore terminal
trap restore_terminal EXIT INT TERM

cd "$PROJECT_ROOT"

if [ ! -x "$BIN" ]; then
    echo "Debug binary not found; building first..."
    cargo build || exit $?
fi

# Execute the disassembler with provided arguments
echo "Executing: $BIN $*"
echo ""

# Run the actual program
"$BIN" "$@"

# Capture exit code
EXIT_CODE=$?

echo ""
echo "Program finished with exit code: $EXIT_CODE"

# Terminal will be restored by the trap
exit $EXIT_CODE
