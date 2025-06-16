use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address(pub u64);

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub address: Address,
    pub mnemonic: String,
    pub operands: String,
    pub bytes: Vec<u8>,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} {}", self.address, self.mnemonic, self.operands)
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub successors: Vec<Address>,
    pub predecessors: Vec<Address>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Unconditional,
    ConditionalTrue,
    ConditionalFalse,
    Call,
    Return,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: Address,
    pub to: Address,
    pub edge_type: EdgeType,
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub blocks: HashMap<Address, BasicBlock>,
    pub edges: Vec<Edge>,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn build_from_instructions(&mut self, instructions: Vec<Instruction>) {
        if instructions.is_empty() {
            return;
        }

        let block_starts = self.find_block_boundaries(&instructions);
        
        self.create_basic_blocks(&instructions, &block_starts);
        
        self.analyze_control_flow();
    }

    fn find_block_boundaries(&self, instructions: &[Instruction]) -> HashSet<Address> {
        let mut boundaries = HashSet::new();
        
        if let Some(first) = instructions.first() {
            boundaries.insert(first.address);
        }

        for instr in instructions {
            match instr.mnemonic.to_lowercase().as_str() {
                "jmp" | "je" | "jne" | "jl" | "jg" | "jle" | "jge" | "ja" | "jae" | "jb" | "jbe" => {
                    if let Some(target) = self.parse_jump_target(&instr.operands) {
                        boundaries.insert(Address(target));
                    }
                }
                "call" => {
                    if let Some(target) = self.parse_jump_target(&instr.operands) {
                        boundaries.insert(Address(target));
                    }
                }
                "ret" => {}
                _ => {}
            }
        }

        boundaries
    }

    fn parse_jump_target(&self, operands: &str) -> Option<u64> {
        if operands.starts_with("0x") {
            u64::from_str_radix(&operands[2..], 16).ok()
        } else {
            operands.parse::<u64>().ok()
        }
    }

    fn create_basic_blocks(&mut self, instructions: &[Instruction], boundaries: &HashSet<Address>) {
        let mut current_block = Vec::new();
        let mut current_start = None;

        for instr in instructions {
            if boundaries.contains(&instr.address) && !current_block.is_empty() {
                if let Some(start_addr) = current_start {
                    let block = BasicBlock {
                        instructions: current_block.clone(),
                        successors: Vec::new(),
                        predecessors: Vec::new(),
                    };
                    self.blocks.insert(start_addr, block);
                }
                current_block.clear();
            }

            if current_block.is_empty() {
                current_start = Some(instr.address);
            }
            current_block.push(instr.clone());
        }

        if !current_block.is_empty() {
            if let Some(start_addr) = current_start {
                let block = BasicBlock {
                    instructions: current_block,
                    successors: Vec::new(),
                    predecessors: Vec::new(),
                };
                self.blocks.insert(start_addr, block);
            }
        }
    }

    fn analyze_control_flow(&mut self) {
        for block_addr in self.blocks.keys().cloned().collect::<Vec<_>>() {
            if let Some(block) = self.blocks.get(&block_addr).cloned() {
                if let Some(last_instr) = block.instructions.last() {
                    match last_instr.mnemonic.to_lowercase().as_str() {
                        "jmp" => {
                            if let Some(target) = self.parse_jump_target(&last_instr.operands) {
                                self.add_edge(block_addr, Address(target), EdgeType::Unconditional);
                            }
                        }
                        "je" | "jne" | "jl" | "jg" | "jle" | "jge" => {
                            if let Some(target) = self.parse_jump_target(&last_instr.operands) {
                                self.add_edge(block_addr, Address(target), EdgeType::ConditionalTrue);
                                
                                let next_addr = Address(last_instr.address.0 + last_instr.bytes.len() as u64);
                                self.add_edge(block_addr, next_addr, EdgeType::ConditionalFalse);
                            }
                        }
                        "call" => {
                            if let Some(target) = self.parse_jump_target(&last_instr.operands) {
                                self.add_edge(block_addr, Address(target), EdgeType::Call);
                                
                                let next_addr = Address(last_instr.address.0 + last_instr.bytes.len() as u64);
                                self.add_edge(block_addr, next_addr, EdgeType::Unconditional);
                            }
                        }
                        "ret" => {
                        }
                        _ => {
                            let next_addr = Address(last_instr.address.0 + last_instr.bytes.len() as u64);
                            if self.blocks.contains_key(&next_addr) {
                                self.add_edge(block_addr, next_addr, EdgeType::Unconditional);
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_edge(&mut self, from: Address, to: Address, edge_type: EdgeType) {
        self.edges.push(Edge { from, to, edge_type });
        
        if let Some(from_block) = self.blocks.get_mut(&from) {
            if !from_block.successors.contains(&to) {
                from_block.successors.push(to);
            }
        }
        
        if let Some(to_block) = self.blocks.get_mut(&to) {
            if !to_block.predecessors.contains(&from) {
                to_block.predecessors.push(from);
            }
        }
    }

    pub fn display_simple(&self) {
        println!("=== Control Flow Graph ===");
        
        let mut sorted_blocks: Vec<_> = self.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);
        
        for (addr, block) in sorted_blocks {
            println!("\nBlock {}:", addr);
            for instr in &block.instructions {
                println!("  {}", instr);
            }
            
            if !block.successors.is_empty() {
                println!("  -> Successors: {:?}", block.successors);
            }
        }
    }

    pub fn display_ascii(&self) {
        println!("=== Control Flow Graph (ASCII) ===\n");
        
        let mut sorted_blocks: Vec<_> = self.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);
        
        for (addr, block) in sorted_blocks {
            println!("┌─ Block {} ─┐", addr);
            for instr in &block.instructions {
                println!("│ {} │", instr);
            }
            
            for successor in &block.successors {
                let edge = self.edges.iter()
                    .find(|e| e.from == *addr && e.to == *successor);
                
                if let Some(edge) = edge {
                    match edge.edge_type {
                        EdgeType::ConditionalTrue => println!("└─[TRUE]──> {}", successor),
                        EdgeType::ConditionalFalse => println!("└─[FALSE]─> {}", successor),
                        EdgeType::Call => println!("└─[CALL]──> {}", successor),
                        _ => println!("└────────> {}", successor),
                    }
                }
            }
            println!();
        }
    }
}