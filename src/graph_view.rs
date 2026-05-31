use std::collections::HashMap;

use crate::graph::{Address, BasicBlock, ControlFlowGraph, EdgeType};

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct BlockLayout {
    pub address: Address,
    pub position: Point,
    pub size: Point,           // width, height
    pub level: i32,            // hierarchical level for layout
    pub block_type: BlockType, // Type of block for color coding
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Entry,       // Function entry points
    Exit,        // Function exits (returns)
    Conditional, // Blocks ending with conditional jumps
    Call,        // Blocks containing function calls
    Loop,        // Blocks that are part of loops
    Normal,      // Regular basic blocks
    Data,        // Data blocks (non-executable)
}

#[derive(Debug, Clone)]
pub struct EdgeLayout {
    pub from: Address,
    pub to: Address,
    pub edge_type: EdgeType,
    pub points: Vec<Point>, // Path points for edge routing
}

#[derive(Debug)]
pub struct GraphViewport {
    pub center_x: f64,
    pub center_y: f64,
    pub zoom: f64,
    pub width: f64,
    pub height: f64,
}

impl GraphViewport {
    pub fn new() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 1.0,
            width: 100.0,
            height: 50.0,
        }
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.center_x += dx / self.zoom;
        self.center_y += dy / self.zoom;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.1);
    }

    pub fn center_on(&mut self, x: f64, y: f64) {
        self.center_x = x;
        self.center_y = y;
    }

    // Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_x: f64, world_y: f64) -> (f64, f64) {
        let screen_x = (world_x - self.center_x) * self.zoom + self.width / 2.0;
        let screen_y = (world_y - self.center_y) * self.zoom + self.height / 2.0;
        (screen_x, screen_y)
    }

    // Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_x: f64, screen_y: f64) -> (f64, f64) {
        let world_x = (screen_x - self.width / 2.0) / self.zoom + self.center_x;
        let world_y = (screen_y - self.height / 2.0) / self.zoom + self.center_y;
        (world_x, world_y)
    }

    pub fn update_size(&mut self, width: u16, height: u16) {
        self.width = width as f64;
        self.height = height as f64;
    }
}

impl Default for GraphViewport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GraphView {
    pub blocks: HashMap<Address, BlockLayout>,
    pub edges: Vec<EdgeLayout>,
    pub viewport: GraphViewport,
    pub selected_block: Option<Address>,
    pub layout_computed: bool,
}

impl GraphView {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            edges: Vec::new(),
            viewport: GraphViewport::new(),
            selected_block: None,
            layout_computed: false,
        }
    }

    pub fn build_from_cfg(&mut self, cfg: &ControlFlowGraph) {
        self.blocks.clear();
        self.edges.clear();

        // Calculate block layouts using hierarchical layout
        self.compute_hierarchical_layout(cfg);

        // Create edge layouts
        self.compute_edge_layouts(cfg);

        self.layout_computed = true;

        // Select the first block if available and center viewport
        if self.selected_block.is_none() {
            if let Some(first_addr) = self.blocks.keys().next().copied() {
                self.selected_block = Some(first_addr);
            }
        }

        // Always center on selected block after layout
        if let Some(selected) = self.selected_block {
            if let Some(block) = self.blocks.get(&selected) {
                self.viewport.center_on(block.position.x, block.position.y);
            }
        }
    }

    fn compute_hierarchical_layout(&mut self, cfg: &ControlFlowGraph) {
        if cfg.blocks.is_empty() {
            return;
        }

        // eprintln!("DEBUG: Starting layout computation for {} blocks", cfg.blocks.len());

        // Use simple grid layout for all graphs for now
        self.compute_grid_layout(cfg);
    }

    fn compute_grid_layout(&mut self, cfg: &ControlFlowGraph) {
        // eprintln!("DEBUG: Using grid layout for {} blocks", cfg.blocks.len());

        let blocks_per_row = (cfg.blocks.len() as f64).sqrt().ceil() as usize;
        let block_width = 50.0;
        let block_height = 12.0;
        let spacing_x = 80.0;
        let spacing_y = 25.0;

        let mut sorted_blocks: Vec<_> = cfg.blocks.keys().collect();
        sorted_blocks.sort();

        for (i, &addr) in sorted_blocks.iter().enumerate() {
            let row = i / blocks_per_row;
            let col = i % blocks_per_row;

            let x = col as f64 * spacing_x - (blocks_per_row as f64 * spacing_x) / 2.0;
            let y = row as f64 * spacing_y;

            let block_type = self.classify_block_type(cfg, *addr);

            self.blocks.insert(
                *addr,
                BlockLayout {
                    address: *addr,
                    position: Point::new(x, y),
                    size: Point::new(block_width, block_height),
                    level: row as i32,
                    block_type,
                },
            );
        }

        // eprintln!("DEBUG: Grid layout: {} rows, {} blocks positioned",
        //          (sorted_blocks.len() + blocks_per_row - 1) / blocks_per_row,
        //          sorted_blocks.len());
    }

    fn compute_edge_layouts(&mut self, cfg: &ControlFlowGraph) {
        for edge in &cfg.edges {
            if let (Some(from_block), Some(to_block)) =
                (self.blocks.get(&edge.from), self.blocks.get(&edge.to))
            {
                // Simple straight line for now
                let from_point = Point::new(
                    from_block.position.x + from_block.size.x / 2.0,
                    from_block.position.y + from_block.size.y,
                );
                let to_point = Point::new(
                    to_block.position.x + to_block.size.x / 2.0,
                    to_block.position.y,
                );

                self.edges.push(EdgeLayout {
                    from: edge.from,
                    to: edge.to,
                    edge_type: edge.edge_type.clone(),
                    points: vec![from_point, to_point],
                });
            }
        }
    }

    pub fn move_selection(&mut self, cfg: &ControlFlowGraph, direction: NavigationDirection) {
        if let Some(current) = self.selected_block {
            let new_selection = match direction {
                NavigationDirection::Up => {
                    // Find predecessor with highest address
                    if let Some(block) = cfg.blocks.get(&current) {
                        block.predecessors.iter().max().copied()
                    } else {
                        None
                    }
                }
                NavigationDirection::Down => {
                    // Find successor with lowest address
                    if let Some(block) = cfg.blocks.get(&current) {
                        block.successors.iter().min().copied()
                    } else {
                        None
                    }
                }
                NavigationDirection::Left => {
                    // Find block at same level to the left
                    self.find_block_in_direction(current, -1.0, 0.0)
                }
                NavigationDirection::Right => {
                    // Find block at same level to the right
                    self.find_block_in_direction(current, 1.0, 0.0)
                }
            };

            if let Some(new_addr) = new_selection {
                self.selected_block = Some(new_addr);
                if let Some(block) = self.blocks.get(&new_addr) {
                    self.viewport.center_on(block.position.x, block.position.y);
                }
            }
        }
    }

    fn find_block_in_direction(&self, current: Address, dx: f64, dy: f64) -> Option<Address> {
        if let Some(current_block) = self.blocks.get(&current) {
            let mut best_addr = None;
            let mut best_distance = f64::INFINITY;

            for (&addr, block) in &self.blocks {
                if addr == current {
                    continue;
                }

                let diff_x = block.position.x - current_block.position.x;
                let diff_y = block.position.y - current_block.position.y;

                // For horizontal movement, prioritize blocks on same level
                if dx != 0.0 && dy == 0.0 {
                    // Check if block is in the desired horizontal direction
                    if (dx > 0.0 && diff_x <= 0.0) || (dx < 0.0 && diff_x >= 0.0) {
                        continue;
                    }

                    // Prioritize blocks on the same level (same Y coordinate)
                    let level_diff = (block.position.y - current_block.position.y).abs();
                    let distance = diff_x.abs() + level_diff * 2.0; // Weight level difference more

                    if distance < best_distance {
                        best_distance = distance;
                        best_addr = Some(addr);
                    }
                } else {
                    // For general direction movement
                    // Check if block is in the desired direction
                    if (dx > 0.0 && diff_x <= 0.0)
                        || (dx < 0.0 && diff_x >= 0.0)
                        || (dy > 0.0 && diff_y <= 0.0)
                        || (dy < 0.0 && diff_y >= 0.0)
                    {
                        continue;
                    }

                    let distance = (diff_x * diff_x + diff_y * diff_y).sqrt();
                    if distance < best_distance {
                        best_distance = distance;
                        best_addr = Some(addr);
                    }
                }
            }

            best_addr
        } else {
            None
        }
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.viewport.pan(dx, dy);
    }

    pub fn zoom_in(&mut self) {
        self.viewport.zoom_in();
    }

    pub fn zoom_out(&mut self) {
        self.viewport.zoom_out();
    }

    pub fn center_on_selected(&mut self) {
        if let Some(selected) = self.selected_block {
            if let Some(block) = self.blocks.get(&selected) {
                self.viewport.center_on(block.position.x, block.position.y);
            }
        }
    }

    pub fn get_selected_block<'a>(&self, cfg: &'a ControlFlowGraph) -> Option<&'a BasicBlock> {
        self.selected_block.and_then(|addr| cfg.blocks.get(&addr))
    }

    fn classify_block_type(&self, cfg: &ControlFlowGraph, addr: Address) -> BlockType {
        if let Some(block) = cfg.blocks.get(&addr) {
            // Check if this is an entry block (no predecessors)
            if block.predecessors.is_empty() {
                return BlockType::Entry;
            }

            // Check if this is an exit block (no successors or ends with return)
            if block.successors.is_empty() {
                return BlockType::Exit;
            }

            // Check the last instruction to determine block type
            if let Some(last_instr) = block.instructions.last() {
                let mnemonic = last_instr.mnemonic.to_lowercase();

                match mnemonic.as_str() {
                    // Return instructions
                    "ret" | "retn" | "retf" | "iret" | "iretd" | "iretq" => BlockType::Exit,

                    // Conditional jumps
                    "je" | "jne" | "jl" | "jg" | "jle" | "jge" | "ja" | "jae" | "jb" | "jbe"
                    | "jc" | "jnc" | "jo" | "jno" | "js" | "jns" | "jp" | "jpe" | "jnp" | "jpo"
                    | "jcxz" | "jecxz" | "jrcxz" => BlockType::Conditional,

                    // Loop instructions
                    "loop" | "loope" | "loopne" | "loopnz" | "loopz" => BlockType::Loop,

                    // Call instructions
                    "call" => BlockType::Call,

                    _ => {
                        // Check if any instruction in the block is a call
                        let has_call = block
                            .instructions
                            .iter()
                            .any(|instr| instr.mnemonic.to_lowercase() == "call");

                        if has_call {
                            BlockType::Call
                        } else {
                            // Check if this block is part of a loop (has back edges)
                            let is_loop = block.successors.iter().any(|&succ| succ <= addr); // Simple back-edge detection

                            if is_loop {
                                BlockType::Loop
                            } else {
                                BlockType::Normal
                            }
                        }
                    }
                }
            } else {
                // Empty block or data block
                BlockType::Data
            }
        } else {
            BlockType::Normal
        }
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationDirection {
    Up,
    Down,
    Left,
    Right,
}
