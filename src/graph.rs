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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Entry,
    Standard,
    Thunk,
    Unknown,
}

impl FunctionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FunctionKind::Entry => "entry",
            FunctionKind::Standard => "standard",
            FunctionKind::Thunk => "thunk",
            FunctionKind::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionConfidence {
    Low,
    Medium,
    High,
}

impl FunctionConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            FunctionConfidence::Low => "low",
            FunctionConfidence::Medium => "medium",
            FunctionConfidence::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSummary {
    pub entry: Address,
    pub block_count: usize,
    pub instruction_count: usize,
    pub edge_count: usize,
    pub caller_count: usize,
    pub kind: FunctionKind,
    pub confidence: FunctionConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSeedSource {
    EntryPoint,
    Symbol,
    Export,
    Unwind,
}

impl FunctionSeedSource {
    fn priority(self) -> u8 {
        match self {
            FunctionSeedSource::EntryPoint => 2,
            FunctionSeedSource::Symbol
            | FunctionSeedSource::Export
            | FunctionSeedSource::Unwind => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionSeed {
    pub address: Address,
    pub source: FunctionSeedSource,
}

impl FunctionSeed {
    pub fn entry_point(address: Address) -> Self {
        Self {
            address,
            source: FunctionSeedSource::EntryPoint,
        }
    }

    pub fn symbol(address: Address) -> Self {
        Self {
            address,
            source: FunctionSeedSource::Symbol,
        }
    }

    pub fn export(address: Address) -> Self {
        Self {
            address,
            source: FunctionSeedSource::Export,
        }
    }

    pub fn unwind(address: Address) -> Self {
        Self {
            address,
            source: FunctionSeedSource::Unwind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallGraphFunction {
    pub summary: FunctionSummary,
    pub incoming_call_count: usize,
    pub outgoing_call_count: usize,
    pub import_thunk: Option<ExternalCallTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallGraphEdge {
    pub caller: Address,
    pub callee: Address,
    pub call_sites: Vec<Address>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCallTarget {
    pub address: Address,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoreturnCallTarget {
    pub address: Address,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallGraphExternalFunction {
    pub address: Address,
    pub label: String,
    pub incoming_call_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallGraphExternalEdge {
    pub caller: Address,
    pub target: Address,
    pub label: String,
    pub call_sites: Vec<Address>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallGraph {
    pub functions: Vec<CallGraphFunction>,
    pub edges: Vec<CallGraphEdge>,
    pub external_functions: Vec<CallGraphExternalFunction>,
    pub external_edges: Vec<CallGraphExternalEdge>,
}

impl CallGraph {
    pub fn total_edge_count(&self) -> usize {
        self.edges.len() + self.external_edges.len()
    }
}

pub struct ControlFlowGraph {
    pub blocks: HashMap<Address, BasicBlock>,
    pub edges: Vec<Edge>,
    graph: Graph<Address, EdgeType, Directed>,
    addr_to_node: HashMap<Address, NodeIndex>,
    function_seeds: HashMap<Address, FunctionSeedSource>,
    noreturn_call_targets: HashMap<Address, String>,
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
            function_seeds: HashMap::new(),
            noreturn_call_targets: HashMap::new(),
        }
    }

    /// Build CFG from a list of instructions
    pub fn build_from_instructions(&mut self, instructions: Vec<Instruction>) {
        self.build_from_instructions_with_function_seeds_and_noreturn_targets(
            instructions,
            std::iter::empty::<FunctionSeed>(),
            std::iter::empty::<NoreturnCallTarget>(),
        );
    }

    /// Build CFG from instructions and trusted function entry hints.
    pub fn build_from_instructions_with_function_seeds<I>(
        &mut self,
        instructions: Vec<Instruction>,
        function_seeds: I,
    ) where
        I: IntoIterator<Item = FunctionSeed>,
    {
        self.build_from_instructions_with_function_seeds_and_noreturn_targets(
            instructions,
            function_seeds,
            std::iter::empty::<NoreturnCallTarget>(),
        );
    }

    /// Build CFG from instructions, trusted function entries, and known noreturn call targets.
    pub fn build_from_instructions_with_function_seeds_and_noreturn_targets<I, J>(
        &mut self,
        instructions: Vec<Instruction>,
        function_seeds: I,
        noreturn_targets: J,
    ) where
        I: IntoIterator<Item = FunctionSeed>,
        J: IntoIterator<Item = NoreturnCallTarget>,
    {
        self.clear();

        if instructions.is_empty() {
            return;
        }

        let instruction_addresses: HashSet<Address> = instructions
            .iter()
            .map(|instruction| instruction.address)
            .collect();
        for seed in function_seeds {
            if instruction_addresses.contains(&seed.address) {
                Self::insert_function_seed(&mut self.function_seeds, seed);
            }
        }
        for target in noreturn_targets {
            if !target.label.is_empty() {
                self.noreturn_call_targets
                    .entry(target.address)
                    .or_insert(target.label);
            }
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

    fn clear(&mut self) {
        self.blocks.clear();
        self.edges.clear();
        self.graph = Graph::new();
        self.addr_to_node.clear();
        self.function_seeds.clear();
        self.noreturn_call_targets.clear();
    }

    fn insert_function_seed(seeds: &mut HashMap<Address, FunctionSeedSource>, seed: FunctionSeed) {
        seeds
            .entry(seed.address)
            .and_modify(|source| {
                if seed.source.priority() > source.priority() {
                    *source = seed.source;
                }
            })
            .or_insert(seed.source);
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

        leaders.extend(self.function_seeds.keys().copied());

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

                        if !self.is_noreturn_call_instruction(last_instr, instructions) {
                            // Continue after returning call
                            if let Some(next_addr) =
                                self.get_next_instruction_address(last_instr, instructions)
                            {
                                if self.blocks.contains_key(&next_addr) {
                                    edges_to_add.push((
                                        block_addr,
                                        next_addr,
                                        EdgeType::Unconditional,
                                    ));
                                }
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
        let boundary_entries = self.function_entries();
        let first_entry = self.first_function_entry();
        let mut summaries: Vec<FunctionSummary> = boundary_entries
            .iter()
            .filter_map(|entry| {
                let summary = self.summarize_function(*entry, &boundary_entries, first_entry);
                summary.is_reportable().then_some(summary)
            })
            .collect();

        summaries.sort_by_key(|summary| summary.entry);
        summaries
    }

    pub fn function_block_addresses(&self, entry: Address) -> Vec<Address> {
        let entries = self.function_entries();
        let first_entry = self.first_function_entry();
        let summary = self.summarize_function(entry, &entries, first_entry);
        if !entries.contains(&entry) || !summary.is_reportable() {
            return Vec::new();
        }

        let mut blocks: Vec<_> = self
            .collect_function_blocks(entry, &entries)
            .into_iter()
            .collect();
        blocks.sort();
        blocks
    }

    pub fn function_entry_containing_address(&self, address: Address) -> Option<Address> {
        let block_addr = if self.blocks.contains_key(&address) {
            Some(address)
        } else {
            self.blocks.iter().find_map(|(block_addr, block)| {
                block
                    .instructions
                    .iter()
                    .any(|instruction| instruction.address == address)
                    .then_some(*block_addr)
            })
        }?;

        let entries = self.function_entries();
        let first_entry = self.first_function_entry();
        let mut ordered_entries: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| {
                self.summarize_function(*entry, &entries, first_entry)
                    .is_reportable()
            })
            .collect();
        ordered_entries.sort();

        ordered_entries.into_iter().find(|entry| {
            self.collect_function_blocks(*entry, &entries)
                .contains(&block_addr)
        })
    }

    pub fn noreturn_call_target_count(&self) -> usize {
        self.noreturn_call_targets.len()
    }

    pub fn call_graph(&self) -> CallGraph {
        self.call_graph_with_external_targets(std::iter::empty::<ExternalCallTarget>())
    }

    pub fn call_graph_with_external_targets<I>(&self, external_targets: I) -> CallGraph
    where
        I: IntoIterator<Item = ExternalCallTarget>,
    {
        let external_targets = collect_external_call_targets(external_targets);
        let entries = self.function_entries();
        let first_entry = self.first_function_entry();
        let import_thunks = if external_targets.is_empty() {
            HashMap::new()
        } else {
            self.collect_import_thunks(&entries, &external_targets)
        };
        let mut ordered_entries: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| {
                self.summarize_function(*entry, &entries, first_entry)
                    .is_reportable()
                    || import_thunks.contains_key(entry)
            })
            .collect();
        ordered_entries.sort();

        let reportable_entries: HashSet<_> = ordered_entries.iter().copied().collect();
        let mut owner_by_block = HashMap::new();
        let mut summaries = Vec::new();

        for entry in &ordered_entries {
            let blocks = self.collect_function_blocks(*entry, &entries);
            for block in blocks {
                owner_by_block.entry(block).or_insert(*entry);
            }
            summaries.push(self.summarize_function(*entry, &entries, first_entry));
        }

        let mut edge_sites: HashMap<(Address, Address), Vec<Address>> = HashMap::new();
        for edge in self
            .edges
            .iter()
            .filter(|edge| edge.edge_type == EdgeType::Call)
        {
            let Some(caller) = owner_by_block.get(&edge.from).copied() else {
                continue;
            };
            let callee = owner_by_block
                .get(&edge.to)
                .copied()
                .or_else(|| reportable_entries.contains(&edge.to).then_some(edge.to));
            let Some(callee) = callee else {
                continue;
            };

            let call_site = self
                .blocks
                .get(&edge.from)
                .and_then(|block| block.instructions.last())
                .filter(|instruction| instruction.mnemonic.eq_ignore_ascii_case("call"))
                .map(|instruction| instruction.address)
                .unwrap_or(edge.from);

            edge_sites
                .entry((caller, callee))
                .or_default()
                .push(call_site);
        }

        let mut edges: Vec<CallGraphEdge> = edge_sites
            .into_iter()
            .map(|((caller, callee), mut call_sites)| {
                call_sites.sort();
                call_sites.dedup();
                CallGraphEdge {
                    caller,
                    callee,
                    call_sites,
                }
            })
            .collect();
        edges.sort_by_key(|edge| (edge.caller, edge.callee));

        let mut external_edge_sites: HashMap<(Address, Address), Vec<Address>> = HashMap::new();
        if !external_targets.is_empty() {
            for (block_addr, block) in &self.blocks {
                let Some(caller) = owner_by_block.get(block_addr).copied() else {
                    continue;
                };

                for instruction in &block.instructions {
                    if !instruction.mnemonic.eq_ignore_ascii_case("call") {
                        continue;
                    }
                    let Some(target) = self
                        .parse_jump_target(&instruction.operands, instruction.address.0)
                        .map(Address)
                    else {
                        continue;
                    };
                    if !external_targets.contains_key(&target) {
                        continue;
                    }

                    external_edge_sites
                        .entry((caller, target))
                        .or_default()
                        .push(instruction.address);
                }
            }

            for (entry, target) in &import_thunks {
                if !reportable_entries.contains(entry) {
                    continue;
                }
                external_edge_sites
                    .entry((*entry, target.address))
                    .or_default()
                    .push(*entry);
            }
        }

        let mut external_edges: Vec<CallGraphExternalEdge> = external_edge_sites
            .into_iter()
            .filter_map(|((caller, target), mut call_sites)| {
                call_sites.sort();
                call_sites.dedup();
                Some(CallGraphExternalEdge {
                    caller,
                    target,
                    label: external_targets.get(&target)?.clone(),
                    call_sites,
                })
            })
            .collect();
        external_edges.sort_by_key(|edge| (edge.caller, edge.target, edge.label.clone()));

        let incoming_counts = edges.iter().fold(HashMap::new(), |mut counts, edge| {
            *counts.entry(edge.callee).or_insert(0) += edge.call_sites.len();
            counts
        });
        let mut outgoing_counts = edges.iter().fold(HashMap::new(), |mut counts, edge| {
            *counts.entry(edge.caller).or_insert(0) += edge.call_sites.len();
            counts
        });
        for edge in &external_edges {
            *outgoing_counts.entry(edge.caller).or_insert(0) += edge.call_sites.len();
        }

        let external_incoming_counts =
            external_edges
                .iter()
                .fold(HashMap::new(), |mut counts, edge| {
                    *counts.entry(edge.target).or_insert(0) += edge.call_sites.len();
                    counts
                });
        let mut external_functions: Vec<CallGraphExternalFunction> = external_targets
            .into_iter()
            .filter_map(|(address, label)| {
                let incoming_call_count = external_incoming_counts.get(&address).copied()?;
                Some(CallGraphExternalFunction {
                    address,
                    label,
                    incoming_call_count,
                })
            })
            .collect();
        external_functions.sort_by_key(|function| (function.label.clone(), function.address));

        let mut functions: Vec<CallGraphFunction> = summaries
            .into_iter()
            .map(|summary| CallGraphFunction {
                incoming_call_count: incoming_counts
                    .get(&summary.entry)
                    .copied()
                    .unwrap_or_default(),
                outgoing_call_count: outgoing_counts
                    .get(&summary.entry)
                    .copied()
                    .unwrap_or_default(),
                import_thunk: import_thunks.get(&summary.entry).cloned(),
                summary,
            })
            .collect();
        functions.sort_by_key(|function| function.summary.entry);

        CallGraph {
            functions,
            edges,
            external_functions,
            external_edges,
        }
    }

    fn is_noreturn_call_instruction(
        &self,
        instruction: &Instruction,
        instructions: &[Instruction],
    ) -> bool {
        let Some(target) = self
            .parse_jump_target(&instruction.operands, instruction.address.0)
            .map(Address)
        else {
            return false;
        };

        self.noreturn_call_targets.contains_key(&target)
            || self
                .noreturn_import_thunk_target(target, instructions)
                .is_some()
    }

    fn noreturn_import_thunk_target(
        &self,
        entry: Address,
        instructions: &[Instruction],
    ) -> Option<NoreturnCallTarget> {
        let block = self.blocks.get(&entry)?;
        match block.instructions.as_slice() {
            [instruction] if instruction.mnemonic.eq_ignore_ascii_case("jmp") => {
                self.noreturn_target_from_instruction(instruction)
            }
            [instruction] if instruction.mnemonic.eq_ignore_ascii_case("call") => {
                let target = self.noreturn_target_from_instruction(instruction)?;
                let next = self.next_instruction(instruction, instructions)?;
                is_return_mnemonic(&next.mnemonic).then_some(target)
            }
            [call, ret]
                if call.mnemonic.eq_ignore_ascii_case("call")
                    && is_return_mnemonic(&ret.mnemonic) =>
            {
                self.noreturn_target_from_instruction(call)
            }
            _ => None,
        }
    }

    fn noreturn_target_from_instruction(
        &self,
        instruction: &Instruction,
    ) -> Option<NoreturnCallTarget> {
        let address = self
            .parse_jump_target(&instruction.operands, instruction.address.0)
            .map(Address)?;
        let label = self.noreturn_call_targets.get(&address)?.clone();
        Some(NoreturnCallTarget { address, label })
    }

    fn next_instruction<'a>(
        &self,
        instruction: &Instruction,
        instructions: &'a [Instruction],
    ) -> Option<&'a Instruction> {
        let next = self.get_next_instruction_address(instruction, instructions)?;
        instructions
            .iter()
            .find(|candidate| candidate.address == next)
    }

    fn collect_import_thunks(
        &self,
        function_entries: &HashSet<Address>,
        external_targets: &HashMap<Address, String>,
    ) -> HashMap<Address, ExternalCallTarget> {
        function_entries
            .iter()
            .filter_map(|entry| {
                let target = self.import_thunk_target(*entry, external_targets)?;
                Some((*entry, target))
            })
            .collect()
    }

    fn import_thunk_target(
        &self,
        entry: Address,
        external_targets: &HashMap<Address, String>,
    ) -> Option<ExternalCallTarget> {
        let block = self.blocks.get(&entry)?;
        match block.instructions.as_slice() {
            [instruction] if instruction.mnemonic.eq_ignore_ascii_case("jmp") => {
                self.external_target_from_instruction(instruction, external_targets)
            }
            [instruction] if instruction.mnemonic.eq_ignore_ascii_case("call") => {
                let target =
                    self.external_target_from_instruction(instruction, external_targets)?;
                let returns_after_call = block
                    .successors
                    .iter()
                    .filter_map(|successor| self.blocks.get(successor))
                    .any(is_return_only_block);
                returns_after_call.then_some(target)
            }
            [call, ret]
                if call.mnemonic.eq_ignore_ascii_case("call")
                    && is_return_mnemonic(&ret.mnemonic) =>
            {
                self.external_target_from_instruction(call, external_targets)
            }
            _ => None,
        }
    }

    fn external_target_from_instruction(
        &self,
        instruction: &Instruction,
        external_targets: &HashMap<Address, String>,
    ) -> Option<ExternalCallTarget> {
        let address = self
            .parse_jump_target(&instruction.operands, instruction.address.0)
            .map(Address)?;
        let label = external_targets.get(&address)?.clone();
        Some(ExternalCallTarget { address, label })
    }

    fn first_function_entry(&self) -> Option<Address> {
        self.function_seeds
            .iter()
            .filter_map(|(address, source)| {
                (*source == FunctionSeedSource::EntryPoint && self.blocks.contains_key(address))
                    .then_some(*address)
            })
            .min()
            .or_else(|| self.blocks.keys().min().copied())
    }

    fn function_entries(&self) -> HashSet<Address> {
        let mut entries = HashSet::new();

        if let Some(first) = self.first_function_entry() {
            entries.insert(first);
        }

        for (&addr, block) in &self.blocks {
            if is_x86_prologue(block) {
                entries.insert(addr);
            }
        }

        for address in self.function_seeds.keys() {
            if self.blocks.contains_key(address) {
                entries.insert(*address);
            }
        }

        for edge in &self.edges {
            if edge.edge_type == EdgeType::Call && self.blocks.contains_key(&edge.to) {
                entries.insert(edge.to);
            }
        }

        entries
    }

    fn collect_function_blocks(
        &self,
        entry: Address,
        function_entries: &HashSet<Address>,
    ) -> HashSet<Address> {
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

        visited
    }

    fn summarize_function(
        &self,
        entry: Address,
        function_entries: &HashSet<Address>,
        first_entry: Option<Address>,
    ) -> FunctionSummary {
        let visited = self.collect_function_blocks(entry, function_entries);

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

        let has_prologue = self.blocks.get(&entry).is_some_and(is_x86_prologue);
        let has_seed = self.function_seeds.contains_key(&entry);
        let kind = if first_entry == Some(entry) {
            FunctionKind::Entry
        } else if has_prologue || has_seed {
            FunctionKind::Standard
        } else if instruction_count <= 2 && edge_count == 0 {
            FunctionKind::Thunk
        } else {
            FunctionKind::Unknown
        };
        let confidence = match kind {
            FunctionKind::Entry | FunctionKind::Standard => FunctionConfidence::High,
            FunctionKind::Unknown if instruction_count >= 4 || edge_count > 0 => {
                FunctionConfidence::Medium
            }
            FunctionKind::Thunk if caller_count >= 8 => FunctionConfidence::Medium,
            _ => FunctionConfidence::Low,
        };

        FunctionSummary {
            entry,
            block_count: visited.len(),
            instruction_count,
            edge_count,
            caller_count,
            kind,
            confidence,
        }
    }
}

impl FunctionSummary {
    pub fn is_reportable(&self) -> bool {
        matches!(self.kind, FunctionKind::Entry | FunctionKind::Standard)
            || self.confidence >= FunctionConfidence::Medium
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_known_noreturn_symbol(name: &str) -> bool {
    let normalized = normalize_symbol_name(name);
    matches!(
        normalized.as_str(),
        "abort"
            | "exit"
            | "exitprocess"
            | "exitthread"
            | "fastfail"
            | "longjmp"
            | "quick_exit"
            | "raisefailfastexception"
            | "rtlfailfast"
            | "rtlexituserprocess"
            | "siglongjmp"
            | "terminate"
    )
}

fn normalize_symbol_name(name: &str) -> String {
    let symbol = name
        .rsplit_once('!')
        .map(|(_, symbol)| symbol)
        .unwrap_or(name);
    let mut name = symbol
        .split_once("@@")
        .map(|(symbol, _)| symbol)
        .unwrap_or(symbol)
        .to_ascii_lowercase();

    if let Some((symbol, suffix)) = name.rsplit_once('@') {
        if suffix.chars().all(|ch| ch.is_ascii_digit()) {
            name = symbol.to_string();
        }
    }

    while name.starts_with('_') {
        name.remove(0);
    }

    name
}

fn collect_external_call_targets<I>(targets: I) -> HashMap<Address, String>
where
    I: IntoIterator<Item = ExternalCallTarget>,
{
    let mut by_address = HashMap::new();
    for target in targets {
        if target.label.is_empty() {
            continue;
        }
        by_address.entry(target.address).or_insert(target.label);
    }
    by_address
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

fn is_return_only_block(block: &BasicBlock) -> bool {
    matches!(
        block.instructions.as_slice(),
        [instruction] if is_return_mnemonic(&instruction.mnemonic)
    )
}

fn is_return_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic.to_ascii_lowercase().as_str(),
        "ret" | "retn" | "retf" | "iret" | "iretd" | "iretq"
    )
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
        assert_eq!(callee.kind, FunctionKind::Standard);
        assert_eq!(callee.confidence, FunctionConfidence::High);
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

    #[test]
    fn reports_unwind_seeded_functions_without_calls_or_prologues() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                instr(0x1000, "ret", "", 1),
                instr(0x2000, "sub", "rsp,8", 4),
                instr(0x2004, "nop", "", 1),
                instr(0x2005, "ret", "", 1),
            ],
            [FunctionSeed::unwind(Address(0x2000))],
        );

        let seeded = cfg
            .function_summaries()
            .into_iter()
            .find(|summary| summary.entry == Address(0x2000))
            .unwrap();

        assert_eq!(seeded.kind, FunctionKind::Standard);
        assert_eq!(seeded.confidence, FunctionConfidence::High);
        assert_eq!(seeded.instruction_count, 3);
        assert_eq!(
            cfg.function_entry_containing_address(Address(0x2004)),
            Some(Address(0x2000))
        );
    }

    #[test]
    fn entry_point_seed_controls_primary_function_entry() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                instr(0x1000, "ret", "", 1),
                instr(0x2000, "sub", "rsp,8", 4),
                instr(0x2004, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x2000))],
        );

        let summaries = cfg.function_summaries();

        assert!(summaries.iter().any(|summary| {
            summary.entry == Address(0x2000)
                && summary.kind == FunctionKind::Entry
                && summary.confidence == FunctionConfidence::High
        }));
        assert!(!summaries
            .iter()
            .any(|summary| summary.entry == Address(0x1000)));
    }

    #[test]
    fn hides_low_confidence_tiny_call_targets_from_public_functions() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            instr(0x1000, "call", "0000000000002000h", 5),
            instr(0x1005, "ret", "", 1),
            instr(0x2000, "ret", "", 1),
        ]);

        let summaries = cfg.function_summaries();

        assert!(summaries
            .iter()
            .any(|summary| summary.entry == Address(0x1000)
                && summary.kind == FunctionKind::Entry
                && summary.confidence == FunctionConfidence::High));
        assert!(!summaries
            .iter()
            .any(|summary| summary.entry == Address(0x2000)));
        assert!(cfg.function_block_addresses(Address(0x2000)).is_empty());
        assert_eq!(cfg.function_entry_containing_address(Address(0x2000)), None);

        let call_graph = cfg.call_graph();
        assert_eq!(call_graph.functions.len(), 1);
        assert!(call_graph.edges.is_empty());
    }

    #[test]
    fn recognizes_common_noreturn_symbol_names() {
        assert!(is_known_noreturn_symbol("ExitProcess"));
        assert!(is_known_noreturn_symbol("kernel32.dll!ExitThread"));
        assert!(is_known_noreturn_symbol("_exit"));
        assert!(is_known_noreturn_symbol("exit@@GLIBC_2.2.5"));
        assert!(!is_known_noreturn_symbol("CreateFileW"));
        assert!(!is_known_noreturn_symbol("memcpy"));
    }

    #[test]
    fn suppresses_fallthrough_after_direct_noreturn_import_calls() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds_and_noreturn_targets(
            vec![
                instr(0x1000, "call", "qword [rel 0000000000003000h]", 6),
                instr(0x1006, "mov", "eax,1", 5),
                instr(0x100b, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
            [NoreturnCallTarget {
                address: Address(0x3000),
                label: "kernel32.dll!ExitProcess".to_string(),
            }],
        );

        assert_eq!(cfg.noreturn_call_target_count(), 1);
        assert!(cfg.edges.is_empty());
        assert!(cfg.blocks[&Address(0x1000)].successors.is_empty());
        assert_eq!(
            cfg.function_summaries()
                .into_iter()
                .find(|summary| summary.entry == Address(0x1000))
                .unwrap()
                .instruction_count,
            1
        );
    }

    #[test]
    fn suppresses_fallthrough_after_calls_to_noreturn_import_thunks() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds_and_noreturn_targets(
            vec![
                instr(0x1000, "call", "0000000000002000h", 5),
                instr(0x1005, "mov", "eax,1", 5),
                instr(0x100a, "ret", "", 1),
                instr(0x2000, "jmp", "qword [rel 0000000000003000h]", 6),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
            [NoreturnCallTarget {
                address: Address(0x3000),
                label: "kernel32.dll!ExitProcess".to_string(),
            }],
        );

        assert!(cfg.edges.iter().any(|edge| {
            edge.from == Address(0x1000)
                && edge.to == Address(0x2000)
                && edge.edge_type == EdgeType::Call
        }));
        assert!(!cfg
            .edges
            .iter()
            .any(|edge| edge.from == Address(0x1000) && edge.to == Address(0x1005)));
        assert!(!cfg.blocks[&Address(0x1000)]
            .successors
            .contains(&Address(0x1005)));
    }
    #[test]
    fn builds_external_import_call_graph_edges() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                instr(0x1000, "sub", "rsp,28h", 4),
                instr(0x1004, "call", "qword [rel 0000000000003000h]", 6),
                instr(0x100a, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
        );

        let call_graph = cfg.call_graph_with_external_targets([ExternalCallTarget {
            address: Address(0x3000),
            label: "kernel32.dll!ExitProcess".to_string(),
        }]);

        assert_eq!(call_graph.functions.len(), 1);
        assert!(call_graph.edges.is_empty());
        assert_eq!(call_graph.external_functions.len(), 1);
        assert_eq!(call_graph.external_functions[0].incoming_call_count, 1);
        assert_eq!(call_graph.external_edges.len(), 1);
        assert_eq!(call_graph.external_edges[0].caller, Address(0x1000));
        assert_eq!(call_graph.external_edges[0].target, Address(0x3000));
        assert_eq!(
            call_graph.external_edges[0].label,
            "kernel32.dll!ExitProcess"
        );
        assert_eq!(
            call_graph.external_edges[0].call_sites,
            vec![Address(0x1004)]
        );
        assert_eq!(call_graph.functions[0].outgoing_call_count, 1);
        assert_eq!(call_graph.total_edge_count(), 1);
    }

    #[test]
    fn resolves_low_confidence_import_jump_thunks() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                instr(0x1000, "call", "0000000000002000h", 5),
                instr(0x1005, "ret", "", 1),
                instr(0x2000, "jmp", "qword [rel 0000000000003000h]", 6),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
        );

        assert!(!cfg
            .function_summaries()
            .iter()
            .any(|summary| summary.entry == Address(0x2000)));

        let call_graph = cfg.call_graph_with_external_targets([ExternalCallTarget {
            address: Address(0x3000),
            label: "kernel32.dll!ExitProcess".to_string(),
        }]);

        let thunk = call_graph
            .functions
            .iter()
            .find(|function| function.summary.entry == Address(0x2000))
            .unwrap();
        assert_eq!(thunk.summary.kind, FunctionKind::Thunk);
        assert_eq!(thunk.summary.confidence, FunctionConfidence::Low);
        assert_eq!(
            thunk
                .import_thunk
                .as_ref()
                .map(|target| target.label.as_str()),
            Some("kernel32.dll!ExitProcess")
        );
        assert!(call_graph.edges.iter().any(|edge| {
            edge.caller == Address(0x1000)
                && edge.callee == Address(0x2000)
                && edge.call_sites == vec![Address(0x1000)]
        }));
        assert!(call_graph.external_edges.iter().any(|edge| {
            edge.caller == Address(0x2000)
                && edge.target == Address(0x3000)
                && edge.label == "kernel32.dll!ExitProcess"
                && edge.call_sites == vec![Address(0x2000)]
        }));
    }

    #[test]
    fn resolves_call_return_import_thunks_split_across_blocks() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                instr(0x1000, "call", "0000000000002000h", 5),
                instr(0x1005, "ret", "", 1),
                instr(0x2000, "call", "qword [rel 0000000000003000h]", 6),
                instr(0x2006, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
        );

        let call_graph = cfg.call_graph_with_external_targets([ExternalCallTarget {
            address: Address(0x3000),
            label: "kernel32.dll!ExitProcess".to_string(),
        }]);

        let thunk = call_graph
            .functions
            .iter()
            .find(|function| function.summary.entry == Address(0x2000))
            .unwrap();
        assert_eq!(
            thunk
                .import_thunk
                .as_ref()
                .map(|target| target.label.as_str()),
            Some("kernel32.dll!ExitProcess")
        );
        assert!(call_graph.external_edges.iter().any(|edge| {
            edge.caller == Address(0x2000)
                && edge.target == Address(0x3000)
                && edge.call_sites == vec![Address(0x2000)]
        }));
    }
    #[test]
    fn builds_function_level_call_graph() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            instr(0x1000, "call", "0000000000002000h", 5),
            instr(0x1005, "call", "0000000000003000h", 5),
            instr(0x100a, "ret", "", 1),
            instr(0x2000, "push", "rbp", 1),
            instr(0x2001, "mov", "rbp,rsp", 3),
            instr(0x2004, "call", "0000000000003000h", 5),
            instr(0x2009, "ret", "", 1),
            instr(0x3000, "push", "rbp", 1),
            instr(0x3001, "mov", "rbp,rsp", 3),
            instr(0x3004, "ret", "", 1),
        ]);

        let call_graph = cfg.call_graph();

        assert_eq!(call_graph.functions.len(), 3);
        assert!(call_graph.edges.iter().any(|edge| {
            edge.caller == Address(0x1000)
                && edge.callee == Address(0x2000)
                && edge.call_sites == vec![Address(0x1000)]
        }));
        assert!(call_graph.edges.iter().any(|edge| {
            edge.caller == Address(0x1000)
                && edge.callee == Address(0x3000)
                && edge.call_sites == vec![Address(0x1005)]
        }));
        assert!(call_graph.edges.iter().any(|edge| {
            edge.caller == Address(0x2000)
                && edge.callee == Address(0x3000)
                && edge.call_sites == vec![Address(0x2004)]
        }));

        let callee = call_graph
            .functions
            .iter()
            .find(|function| function.summary.entry == Address(0x3000))
            .unwrap();
        assert_eq!(callee.incoming_call_count, 2);
    }

    #[test]
    fn reports_function_blocks_and_containing_function() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            instr(0x1000, "call", "0000000000002000h", 5),
            instr(0x1005, "ret", "", 1),
            instr(0x2000, "push", "rbp", 1),
            instr(0x2001, "mov", "rbp,rsp", 3),
            instr(0x2004, "je", "0000000000002010h", 2),
            instr(0x2006, "ret", "", 1),
            instr(0x2010, "ret", "", 1),
        ]);

        let blocks = cfg.function_block_addresses(Address(0x2000));

        assert_eq!(
            blocks,
            vec![Address(0x2000), Address(0x2006), Address(0x2010)]
        );
        assert_eq!(
            cfg.function_entry_containing_address(Address(0x2004)),
            Some(Address(0x2000))
        );
        assert_eq!(
            cfg.function_entry_containing_address(Address(0x1000)),
            Some(Address(0x1000))
        );
        assert!(cfg.function_block_addresses(Address(0x9999)).is_empty());
    }
}
