use petgraph::graph::NodeIndex;
use petgraph::{Directed, Graph};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSummary {
    pub entry: Address,
    pub block_count: usize,
    pub instruction_count: usize,
    pub edge_count: usize,
    pub caller_count: usize,
}

pub struct ControlFlowGraph {
    pub blocks: HashMap<Address, BasicBlock>,
    pub edges: Vec<Edge>,
    graph: Graph<Address, EdgeType, Directed>,
    addr_to_node: HashMap<Address, NodeIndex>,
}

impl std::fmt::Debug for ControlFlowGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlFlowGraph")
            .field("blocks", &self.blocks.len())
            .field("edges", &self.edges.len())
            .finish()
    }
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            edges: Vec::new(),
            graph: Graph::new(),
            addr_to_node: HashMap::new(),
        }
    }

    /// Build CFG from a list of instructions
    pub fn build_from_instructions(&mut self, instructions: Vec<Instruction>) {
        if instructions.is_empty() {
            return;
        }

        // Step 1: Find all block leaders (starting addresses of basic blocks)
        let leaders = self.find_block_leaders(&instructions);

        // eprintln!("DEBUG: Found {} block leaders", leaders.len());

        // Step 2: Create basic blocks
        self.create_basic_blocks(&instructions, &leaders);

        // eprintln!("DEBUG: Created {} basic blocks", self.blocks.len());

        // Step 3: Analyze control flow and create edges
        self.analyze_control_flow(&instructions);

        // eprintln!("DEBUG: Created {} edges", self.edges.len());
    }

    /// Find leaders (first instruction of each basic block)
    fn find_block_leaders(&self, instructions: &[Instruction]) -> HashSet<Address> {
        let mut leaders = HashSet::new();

        // First instruction is always a leader
        if let Some(first) = instructions.first() {
            leaders.insert(first.address);
        }

        // Create address to instruction map for quick lookup
        let addr_to_instr: HashMap<Address, &Instruction> = instructions
            .iter()
            .map(|instr| (instr.address, instr))
            .collect();

        // Find all jump targets and add them as leaders
        for instr in instructions {
            if self.is_control_flow_instruction(&instr.mnemonic) {
                // Target of jump/call is a leader
                if let Some(target) = self.parse_jump_target(&instr.operands, instr.address.0) {
                    let target_addr = Address(target);
                    if addr_to_instr.contains_key(&target_addr) {
                        leaders.insert(target_addr);
                    }
                }

                // Instruction after a control flow instruction is a leader
                if let Some(next_addr) = self.get_next_instruction_address(instr, instructions) {
                    leaders.insert(next_addr);
                }
            }
        }

        // Add leaders for function boundaries (heuristic)
        for (i, instr) in instructions.iter().enumerate() {
            // Common function prologue patterns
            if instr.mnemonic.to_lowercase() == "push"
                && instr.operands.contains("rbp")
                && i + 1 < instructions.len()
            {
                let next = &instructions[i + 1];
                if next.mnemonic.to_lowercase() == "mov"
                    && next.operands.contains("rbp")
                    && next.operands.contains("rsp")
                {
                    leaders.insert(instr.address);
                }
            }
        }

        leaders
    }

    /// Check if instruction is a control flow instruction
    fn is_control_flow_instruction(&self, mnemonic: &str) -> bool {
        let mnem = mnemonic.to_lowercase();
        matches!(
            mnem.as_str(),
            "jmp"
                | "je"
                | "jne"
                | "jl"
                | "jg"
                | "jle"
                | "jge"
                | "ja"
                | "jae"
                | "jb"
                | "jbe"
                | "jc"
                | "jnc"
                | "jo"
                | "jno"
                | "js"
                | "jns"
                | "jp"
                | "jpe"
                | "jnp"
                | "jpo"
                | "jcxz"
                | "jecxz"
                | "jrcxz"
                | "loop"
                | "loope"
                | "loopne"
                | "loopnz"
                | "loopz"
                | "call"
                | "ret"
                | "retn"
                | "retf"
                | "iret"
                | "iretd"
                | "iretq"
        )
    }

    /// Get the address of the next instruction
    fn get_next_instruction_address(
        &self,
        instr: &Instruction,
        instructions: &[Instruction],
    ) -> Option<Address> {
        let next_addr = instr.address.0 + instr.bytes.len() as u64;

        // Check if there's actually an instruction at this address
        for next_instr in instructions {
            if next_instr.address.0 == next_addr {
                return Some(next_instr.address);
            }
        }
        None
    }

    /// Parse jump target from operands
    fn parse_jump_target(&self, operands: &str, current_addr: u64) -> Option<u64> {
        let operands = operands.trim();

        // Direct hex addresses (0x...)
        if let Some(addr) = operands.strip_prefix("0x") {
            return u64::from_str_radix(addr, 16).ok();
        }

        if let Some(addr) = parse_nasm_hex(operands) {
            return Some(addr);
        }

        // Relative addresses (rel ...)
        if let Some(addr_part) = operands.strip_prefix("rel ") {
            let addr_part = addr_part.trim();
            if let Some(addr) = addr_part.strip_prefix("0x") {
                return u64::from_str_radix(addr, 16).ok();
            }
            if let Some(addr) = parse_nasm_hex(addr_part) {
                return Some(addr);
            }
        }

        // Short relative jumps (just a number or +/-)
        if let Ok(offset) = operands.parse::<i32>() {
            return Some((current_addr as i64 + offset as i64) as u64);
        }

        // Look for any hex address in the operands
        for part in operands.split_whitespace() {
            let part = part.trim_matches(|ch: char| {
                matches!(ch, '[' | ']' | '(' | ')' | ',' | ':' | '+' | '-')
            });
            if let Some(addr_part) = part.strip_prefix("0x") {
                if let Ok(addr) = u64::from_str_radix(addr_part, 16) {
                    return Some(addr);
                }
            }
            if let Some(addr) = parse_nasm_hex(part) {
                return Some(addr);
            }
        }

        None
    }

    /// Create basic blocks from instructions and leaders
    fn create_basic_blocks(&mut self, instructions: &[Instruction], leaders: &HashSet<Address>) {
        let mut current_block = Vec::new();
        let mut current_leader = None;

        for instr in instructions {
            // If this is a leader and we have a current block, finish it
            if leaders.contains(&instr.address) && !current_block.is_empty() {
                if let Some(leader_addr) = current_leader {
                    let block = BasicBlock {
                        instructions: current_block.clone(),
                        successors: Vec::new(),
                        predecessors: Vec::new(),
                    };
                    self.blocks.insert(leader_addr, block);
                }
                current_block.clear();
            }

            // Start new block if needed
            if current_block.is_empty() {
                current_leader = Some(instr.address);
            }

            current_block.push(instr.clone());

            // End block after terminating instructions
            let mnemonic = instr.mnemonic.to_lowercase();
            if matches!(
                mnemonic.as_str(),
                "ret" | "retn" | "retf" | "jmp" | "hlt" | "ud2"
            ) {
                if let Some(leader_addr) = current_leader {
                    let block = BasicBlock {
                        instructions: current_block.clone(),
                        successors: Vec::new(),
                        predecessors: Vec::new(),
                    };
                    self.blocks.insert(leader_addr, block);
                }
                current_block.clear();
                current_leader = None;
            }
        }

        // Finish last block if exists
        if !current_block.is_empty() {
            if let Some(leader_addr) = current_leader {
                let block = BasicBlock {
                    instructions: current_block,
                    successors: Vec::new(),
                    predecessors: Vec::new(),
                };
                self.blocks.insert(leader_addr, block);
            }
        }
    }

    /// Analyze control flow and create edges between blocks
    fn analyze_control_flow(&mut self, instructions: &[Instruction]) {
        // Create petgraph nodes for each block
        for &addr in self.blocks.keys() {
            let node = self.graph.add_node(addr);
            self.addr_to_node.insert(addr, node);
        }

        // Collect edges first to avoid borrowing issues
        let mut edges_to_add = Vec::new();

        // Analyze each block's last instruction to determine successors
        for (&block_addr, block) in &self.blocks {
            if let Some(last_instr) = block.instructions.last() {
                let mnemonic = last_instr.mnemonic.to_lowercase();

                match mnemonic.as_str() {
                    // Unconditional jump
                    "jmp" => {
                        if let Some(target) =
                            self.parse_jump_target(&last_instr.operands, last_instr.address.0)
                        {
                            let target_addr = Address(target);
                            if self.blocks.contains_key(&target_addr) {
                                edges_to_add.push((
                                    block_addr,
                                    target_addr,
                                    EdgeType::Unconditional,
                                ));
                            }
                        }
                    }

                    // Conditional jumps
                    "je" | "jne" | "jl" | "jg" | "jle" | "jge" | "ja" | "jae" | "jb" | "jbe"
                    | "jc" | "jnc" | "jo" | "jno" | "js" | "jns" | "jp" | "jpe" | "jnp" | "jpo"
                    | "jcxz" | "jecxz" | "jrcxz" | "loop" | "loope" | "loopne" | "loopnz"
                    | "loopz" => {
                        // True branch
                        if let Some(target) =
                            self.parse_jump_target(&last_instr.operands, last_instr.address.0)
                        {
                            let target_addr = Address(target);
                            if self.blocks.contains_key(&target_addr) {
                                edges_to_add.push((
                                    block_addr,
                                    target_addr,
                                    EdgeType::ConditionalTrue,
                                ));
                            }
                        }

                        // False branch (fall through)
                        if let Some(next_addr) =
                            self.get_next_instruction_address(last_instr, instructions)
                        {
                            if self.blocks.contains_key(&next_addr) {
                                edges_to_add.push((
                                    block_addr,
                                    next_addr,
                                    EdgeType::ConditionalFalse,
                                ));
                            }
                        }
                    }

                    // Function call
                    "call" => {
                        if let Some(target) =
                            self.parse_jump_target(&last_instr.operands, last_instr.address.0)
                        {
                            let target_addr = Address(target);
                            if self.blocks.contains_key(&target_addr) {
                                edges_to_add.push((block_addr, target_addr, EdgeType::Call));
                            }
                        }

                        // Continue after call
                        if let Some(next_addr) =
                            self.get_next_instruction_address(last_instr, instructions)
                        {
                            if self.blocks.contains_key(&next_addr) {
                                edges_to_add.push((block_addr, next_addr, EdgeType::Unconditional));
                            }
                        }
                    }

                    // Return - no successors
                    "ret" | "retn" | "retf" | "iret" | "iretd" | "iretq" => {
                        // No outgoing edges
                    }

                    // Default - fall through to next block
                    _ => {
                        if let Some(next_addr) =
                            self.get_next_instruction_address(last_instr, instructions)
                        {
                            if self.blocks.contains_key(&next_addr) {
                                edges_to_add.push((block_addr, next_addr, EdgeType::Unconditional));
                            }
                        }
                    }
                }
            }
        }

        // Now add all the edges
        for (from, to, edge_type) in edges_to_add {
            self.add_edge(from, to, edge_type);
        }
    }

    /// Add an edge between two blocks
    fn add_edge(&mut self, from: Address, to: Address, edge_type: EdgeType) {
        // Add to our edge list
        self.edges.push(Edge {
            from,
            to,
            edge_type: edge_type.clone(),
        });

        // Update block successor/predecessor lists
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

        // Add to petgraph
        if let (Some(&from_node), Some(&to_node)) =
            (self.addr_to_node.get(&from), self.addr_to_node.get(&to))
        {
            self.graph.add_edge(from_node, to_node, edge_type);
        }
    }

    pub fn display_simple(&self) {
        println!("=== Control Flow Graph ===");
        println!("Blocks: {}, Edges: {}", self.blocks.len(), self.edges.len());

        let mut sorted_blocks: Vec<_> = self.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);

        for (addr, block) in sorted_blocks {
            println!(
                "\nBlock {} ({} instructions):",
                addr,
                block.instructions.len()
            );
            for instr in &block.instructions {
                println!("  {}", instr);
            }

            if !block.successors.is_empty() {
                println!("  -> Successors: {:?}", block.successors);
            }
        }
    }

    pub fn display_ascii(&self) {
        println!("=== Control Flow Graph (ASCII) ===");
        println!(
            "Blocks: {}, Edges: {}\n",
            self.blocks.len(),
            self.edges.len()
        );

        let mut sorted_blocks: Vec<_> = self.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);

        for (addr, block) in sorted_blocks {
            println!("┌─ Block {} ({} instrs) ─┐", addr, block.instructions.len());

            // Show first few instructions of the block
            for instr in block.instructions.iter().take(5) {
                println!("│ {} │", instr);
            }

            if block.instructions.len() > 5 {
                println!("│ ... {} more instructions │", block.instructions.len() - 5);
            }

            // Show outgoing edges
            for successor in &block.successors {
                let edge = self
                    .edges
                    .iter()
                    .find(|e| e.from == *addr && e.to == *successor);

                if let Some(edge) = edge {
                    match edge.edge_type {
                        EdgeType::ConditionalTrue => println!("└─[TRUE]──> {}", successor),
                        EdgeType::ConditionalFalse => println!("└─[FALSE]─> {}", successor),
                        EdgeType::Call => println!("└─[CALL]──> {}", successor),
                        EdgeType::Return => println!("└─[RET]───> {}", successor),
                        _ => println!("└────────> {}", successor),
                    }
                }
            }
            println!();
        }
    }

    pub fn function_summaries(&self) -> Vec<FunctionSummary> {
        let entries = self.function_entries();
        let mut summaries: Vec<FunctionSummary> = entries
            .iter()
            .map(|entry| self.summarize_function(*entry, &entries))
            .collect();

        summaries.sort_by_key(|summary| summary.entry);
        summaries
    }

    fn function_entries(&self) -> HashSet<Address> {
        let mut entries = HashSet::new();

        if let Some(first) = self.blocks.keys().min().copied() {
            entries.insert(first);
        }

        for (&addr, block) in &self.blocks {
            if is_x86_prologue(block) {
                entries.insert(addr);
            }
        }

        for edge in &self.edges {
            if edge.edge_type == EdgeType::Call && self.blocks.contains_key(&edge.to) {
                entries.insert(edge.to);
            }
        }

        entries
    }

    fn summarize_function(
        &self,
        entry: Address,
        function_entries: &HashSet<Address>,
    ) -> FunctionSummary {
        let mut visited = HashSet::new();
        let mut stack = vec![entry];

        while let Some(addr) = stack.pop() {
            if !visited.insert(addr) {
                continue;
            }

            let Some(block) = self.blocks.get(&addr) else {
                continue;
            };

            for successor in &block.successors {
                if *successor != entry && function_entries.contains(successor) {
                    continue;
                }
                stack.push(*successor);
            }
        }

        let instruction_count = visited
            .iter()
            .filter_map(|addr| self.blocks.get(addr))
            .map(|block| block.instructions.len())
            .sum();

        let edge_count = self
            .edges
            .iter()
            .filter(|edge| visited.contains(&edge.from) && visited.contains(&edge.to))
            .count();

        let caller_count = self
            .edges
            .iter()
            .filter(|edge| edge.edge_type == EdgeType::Call && edge.to == entry)
            .map(|edge| edge.from)
            .collect::<HashSet<_>>()
            .len();

        FunctionSummary {
            entry,
            block_count: visited.len(),
            instruction_count,
            edge_count,
            caller_count,
        }
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

fn is_x86_prologue(block: &BasicBlock) -> bool {
    let Some(first) = block.instructions.first() else {
        return false;
    };
    let Some(second) = block.instructions.get(1) else {
        return false;
    };

    first.mnemonic.eq_ignore_ascii_case("push")
        && first.operands.contains("rbp")
        && second.mnemonic.eq_ignore_ascii_case("mov")
        && second.operands.contains("rbp")
        && second.operands.contains("rsp")
}

fn parse_nasm_hex(token: &str) -> Option<u64> {
    let token =
        token.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | ',' | ':' | '+' | '-'));
    let digits = token
        .strip_suffix('h')
        .or_else(|| token.strip_suffix('H'))?;

    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    u64::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instr(address: u64, mnemonic: &str, operands: &str, size: usize) -> Instruction {
        Instruction {
            address: Address(address),
            mnemonic: mnemonic.to_string(),
            operands: operands.to_string(),
            bytes: vec![0x90; size],
        }
    }

    #[test]
    fn summarizes_inferred_functions_from_calls() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            instr(0x1000, "call", "0x2000", 5),
            instr(0x1005, "ret", "", 1),
            instr(0x2000, "push", "rbp", 1),
            instr(0x2001, "mov", "rbp,rsp", 3),
            instr(0x2004, "ret", "", 1),
        ]);

        let summaries = cfg.function_summaries();

        assert!(summaries
            .iter()
            .any(|summary| summary.entry == Address(0x1000)));
        let callee = summaries
            .iter()
            .find(|summary| summary.entry == Address(0x2000))
            .unwrap();
        assert_eq!(callee.caller_count, 1);
        assert_eq!(callee.instruction_count, 3);
    }

    #[test]
    fn parses_nasm_style_call_targets() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            instr(0x1000, "call", "0000000000002000h", 5),
            instr(0x1005, "ret", "", 1),
            instr(0x2000, "push", "rbp", 1),
            instr(0x2001, "mov", "rbp,rsp", 3),
            instr(0x2004, "ret", "", 1),
        ]);

        let callee = cfg
            .function_summaries()
            .into_iter()
            .find(|summary| summary.entry == Address(0x2000))
            .unwrap();

        assert_eq!(callee.caller_count, 1);
    }
}
