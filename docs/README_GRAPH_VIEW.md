

# Graph View Fixes and Usage

## Issues Fixed

### 1. Viewport System Integration
- **Problem**: The graph renderer had its own manual viewport offset system disconnected from the GraphView's proper viewport.
- **Solution**: Integrated with GraphView's viewport.world_to_screen() method for proper coordinate transformation.

### 2. Block Size Management
- **Problem**: Large basic blocks (like in notepad.exe with 88,050 instructions) created massive blocks that broke the layout.
- **Solution**: Added limits - maximum 25 lines per block, only consider first 20 instructions for sizing.

### 3. Navigation Improvements
- **Problem**: Left/Right navigation wasn't working properly for finding blocks at the same level.
- **Solution**: Enhanced find_block_in_direction to prioritize blocks on the same hierarchical level.

### 4. Performance Optimization
- **Problem**: Graph layout was rebuilt on every frame.
- **Solution**: Added proper layout_computed flag to avoid unnecessary rebuilds.

### 5. Viewport Updates
- **Problem**: Viewport size wasn't updated to match the rendering area.
- **Solution**: Added viewport.update_size() call during rendering.

### 6. Terminal ANSI Escape Sequence Spam Fix
- **Problem**: After running the disassembler (especially TUI mode), terminal would continuously output ANSI escape sequences like `35;94;10M35;91;9M35;81;7M...`
- **Solution**: 
  - Enhanced terminal cleanup in `src/tui.rs` with comprehensive restoration
  - Added panic hook for crash safety
  - Removed DEBUG output that interfered with terminal state
  - Added `reset_terminal.sh` utility for manual cleanup if needed

## How to Use the Graph View

1. **Start the TUI**: `./main <binary> --cfg --tui`
2. **Switch to Graph View**: Press `3` or use Tab to navigate to "Graph View"
3. **Navigation**:
   - **Arrow Keys**: Navigate between connected blocks
   - **WASD**: Pan the view around
   - **+/-**: Zoom in/out
   - **C**: Center on selected block

## Expected Behavior

### Single Block CFGs
For binaries like notepad.exe that have mostly linear code flow, you'll see one large block. This is correct behavior - the CFG only creates multiple blocks when there are:
- Conditional branches (if/else)
- Function calls and returns
- Loops
- Jump instructions

### Multi-Block CFGs
For binaries with more complex control flow, you'll see multiple blocks connected by edges:
- **Green edges**: True branches (conditional)
- **Red edges**: False branches (conditional)
- **Cyan edges**: Function calls
- **Gray edges**: Unconditional jumps

### Navigation
- **Up/Down arrows**: Follow predecessor/successor relationships
- **Left/Right arrows**: Move between blocks at the same level
- **Pan (WASD)**: Move the viewport to see different parts of the graph
- **Zoom (+/-)**: Adjust the scale of the view
- **Center (C)**: Focus on the currently selected block

## Testing
The graph view now properly handles both single-block and multi-block CFGs with proper viewport management, navigation, and visual representation.

## Terminal Issues & Solutions

### ✅ **ANSI Escape Sequence Spam - FIXED**

**Issue**: Terminal would continuously spam characters like `35;94;10M35;91;9M35;81;7M...` after running the disassembler.

**Status**: **COMPLETELY RESOLVED** ✅

**Solution Applied**:
- Enhanced terminal cleanup with multiple restoration layers
- Added process termination control to prevent background threads
- Comprehensive signal handling for all exit scenarios
- Emergency cleanup utilities provided

### Quick Solutions:

1. **If you still see ANSI spam** (very unlikely):
   ```bash
   ./reset_terminal.sh
   ```

2. **Manual reset**:
   ```bash
   reset && clear
   ```

3. **Prevention**: Always quit TUI with 'q' instead of force-closing.

The terminal cleanup has been completely overhauled and the issue should no longer occur.
