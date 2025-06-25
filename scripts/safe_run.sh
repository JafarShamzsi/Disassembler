#!/bin/bash

# Safe wrapper script for the disassembler
# This ensures terminal is always reset properly

echo "🔧 Starting Disassembler with Safe Terminal Mode"
echo "================================================"

# Pre-execution cleanup
./emergency_reset.sh > /dev/null 2>&1

# Store original terminal settings
ORIGINAL_STTY=$(stty -g 2>/dev/null)

# Function to restore terminal
restore_terminal() {
    echo ""
    echo "🧹 Restoring terminal state..."
    
    # Kill any remaining processes
    pkill -9 -f "main.*notepad" 2>/dev/null || true
    pkill -9 -f "target/debug/main" 2>/dev/null || true
    
    # Restore original settings if available
    if [ -n "$ORIGINAL_STTY" ]; then
        stty "$ORIGINAL_STTY" 2>/dev/null || true
    fi
    
    # Emergency reset
    $(dirname "$0")/emergency_reset.sh > /dev/null 2>&1
    
    echo "✅ Terminal restored"
}

# Set up trap to always restore terminal
trap restore_terminal EXIT INT TERM

# Change to the project root directory
cd "$(dirname "$0")/.."

# Execute the disassembler with provided arguments
echo "🚀 Executing: ./target/debug/main $@"
echo ""

# Run the actual program
./target/debug/main "$@"

# Capture exit code
EXIT_CODE=$?

echo ""
echo "📋 Program finished with exit code: $EXIT_CODE"

# Terminal will be restored by the trap
exit $EXIT_CODE
