#!/bin/bash

echo "🔧 Terminal Reset Utility"
echo "========================="
echo ""

# Kill any running disassembler processes
echo "1. Checking for running processes..."
if pgrep -f "main.*notepad" > /dev/null; then
    echo "   Killing hanging disassembler processes..."
    pkill -f "main.*notepad"
    sleep 1
fi

# Reset terminal state
echo "2. Resetting terminal state..."
reset

# Clear any remaining ANSI sequences
echo "3. Clearing terminal..."
clear

# Restore cursor
echo "4. Restoring cursor..."
tput cnorm

# Test terminal
echo "5. Testing terminal state..."
echo "   Terminal should now be clean and responsive."
echo "   Cursor should be visible and blinking."

echo ""
echo "✅ Terminal reset complete!"
echo ""
echo "If you still see strange characters or escape sequences,"
echo "try closing and reopening your terminal."
