#!/bin/bash

echo "🚨 EMERGENCY Terminal Reset & Process Cleanup"
echo "=============================================="

# Kill any hanging processes
echo "1. Killing any hanging disassembler processes..."
pkill -9 -f "main.*notepad" 2>/dev/null || true
pkill -9 -f "target/debug/main" 2>/dev/null || true
pkill -9 -f "target/release/main" 2>/dev/null || true

# Wait a moment
sleep 1

# Reset terminal completely
echo "2. Performing complete terminal reset..."

# Disable raw mode if it's enabled
printf '\033[?1049l'  # Exit alternate screen
printf '\033[?25h'    # Show cursor
printf '\033[0m'      # Reset all attributes
printf '\033c'        # Full terminal reset
printf '\033[!p'      # Soft terminal reset
printf '\033[?1000l'  # Disable mouse tracking
printf '\033[?1002l'  # Disable button event mouse tracking
printf '\033[?1003l'  # Disable any motion mouse tracking
printf '\033[?1006l'  # Disable extended mouse tracking

# Clear screen and move cursor to top
clear
tput reset 2>/dev/null || true
tput cnorm 2>/dev/null || true

# Force flush any remaining output
exec 2>/dev/null
exec 1>/dev/null
exec 0</dev/null

# Reopen standard streams
exec 1>/dev/tty
exec 2>/dev/tty
exec 0</dev/tty

echo ""
echo "3. Terminal reset complete!"
echo ""
echo "If you're still seeing ANSI escape sequences:"
echo "   1. Close this terminal completely"
echo "   2. Open a new terminal window"
echo "   3. The issue should be resolved"

# Test terminal state
echo ""
echo "Terminal test: If you can see this message clearly, the reset worked."
