use ratatui::widgets::ListState;

use crate::arch::x86::Instruction;
use crate::graph::{Address, ControlFlowGraph, EdgeType, FunctionSummary};
use crate::graph_renderer::GraphRenderer;
use crate::graph_view::GraphView;
use crate::parser::{BinaryAnalysis, ImportSummary, StringSummary, SymbolSummary};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Instructions,
    Functions,
    Names,
    Xrefs,
    ControlFlow,
    GraphView,
    HexDump,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameItem {
    Import(ImportSummary),
    Symbol(SymbolSummary),
    String(StringSummary),
}

#[derive(Debug, Clone, PartialEq)]
pub struct XrefItem {
    pub from: Address,
    pub to: Address,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationLocation {
    pub tab: Tab,
    pub instruction: Option<usize>,
    pub function: Option<usize>,
    pub name: Option<usize>,
    pub xref: Option<usize>,
    pub graph_block: Option<Address>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddressContext {
    pub address: Address,
    pub block: Option<Address>,
    pub containing_function: Option<FunctionSummary>,
    pub nearest_name: Option<NameItem>,
    pub incoming_xrefs: Vec<XrefItem>,
    pub outgoing_xrefs: Vec<XrefItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchMatch {
    Instruction(usize),
    Function(usize),
    Name(usize),
    Xref(usize),
}

impl XrefItem {
    pub(crate) fn kind(&self) -> &'static str {
        match self.edge_type {
            EdgeType::Call => "call",
            EdgeType::ConditionalTrue => "true",
            EdgeType::ConditionalFalse => "false",
            EdgeType::Unconditional => "jump",
            EdgeType::Return => "return",
        }
    }
}

impl NameItem {
    pub(crate) fn address(&self) -> Option<u64> {
        match self {
            NameItem::Import(import) => import.address,
            NameItem::Symbol(symbol) => symbol.address,
            NameItem::String(string) => Some(string.address),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            NameItem::Import(_) => "import",
            NameItem::Symbol(symbol) => symbol.kind.as_str(),
            NameItem::String(_) => "string",
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            NameItem::Import(import) => {
                let library = import.library.as_deref().unwrap_or("unknown");
                format!("{library}!{}", import.name)
            }
            NameItem::Symbol(symbol) => symbol.name.clone(),
            NameItem::String(string) => truncate_for_display(&string.value, 72),
        }
    }
}

pub struct App {
    pub instructions: Vec<Instruction>,
    pub cfg: Option<ControlFlowGraph>,
    pub analysis: BinaryAnalysis,
    pub current_tab: Tab,
    pub instruction_list_state: ListState,
    pub function_list_state: ListState,
    pub name_list_state: ListState,
    pub xref_list_state: ListState,
    pub selected_instruction: Option<usize>,
    pub selected_function: Option<usize>,
    pub selected_name: Option<usize>,
    pub selected_xref: Option<usize>,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub address_jump_mode: bool,
    pub address_jump_query: String,
    pub status_message: Option<String>,
    pub filtered_instructions: Vec<usize>,
    pub search_matches: Vec<SearchMatch>,
    pub selected_search_match: Option<usize>,
    pub instruction_display_cache: Vec<String>,
    pub last_search_query: String,
    pub functions: Vec<FunctionSummary>,
    pub names: Vec<NameItem>,
    pub xrefs: Vec<XrefItem>,
    pub back_stack: Vec<NavigationLocation>,
    pub forward_stack: Vec<NavigationLocation>,
    pub graph_view: GraphView,
    pub graph_renderer: GraphRenderer,
}

impl App {
    pub fn new(
        instructions: Vec<Instruction>,
        cfg: Option<ControlFlowGraph>,
        analysis: BinaryAnalysis,
    ) -> Self {
        let instruction_display_cache: Vec<String> = instructions
            .iter()
            .map(|instr| format!("{:#08x}: {}", instr.address, instr.text))
            .collect();

        let mut graph_view = GraphView::new();
        let functions = cfg
            .as_ref()
            .map(ControlFlowGraph::function_summaries)
            .unwrap_or_default();
        let names = build_name_items(&analysis);
        let xrefs = cfg.as_ref().map(build_xref_items).unwrap_or_default();

        // Initialize graph view with CFG if available
        if let Some(ref cfg) = cfg {
            graph_view.build_from_cfg(cfg);
        }

        let mut app = Self {
            instructions,
            cfg,
            analysis,
            current_tab: Tab::Instructions,
            instruction_list_state: ListState::default(),
            function_list_state: ListState::default(),
            name_list_state: ListState::default(),
            xref_list_state: ListState::default(),
            selected_instruction: None,
            selected_function: None,
            selected_name: None,
            selected_xref: None,
            scroll_offset: 0,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            address_jump_mode: false,
            address_jump_query: String::new(),
            status_message: None,
            filtered_instructions: Vec::new(),
            search_matches: Vec::new(),
            selected_search_match: None,
            instruction_display_cache,
            last_search_query: String::new(),
            functions,
            names,
            xrefs,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            graph_view,
            graph_renderer: GraphRenderer::default(),
        };

        if !app.instructions.is_empty() {
            app.instruction_list_state.select(Some(0));
            app.selected_instruction = Some(0);
            app.filtered_instructions = (0..app.instructions.len()).collect();
        }

        if !app.functions.is_empty() {
            app.function_list_state.select(Some(0));
            app.selected_function = Some(0);
        }

        if !app.names.is_empty() {
            app.name_list_state.select(Some(0));
            app.selected_name = Some(0);
        }

        if !app.xrefs.is_empty() {
            app.xref_list_state.select(Some(0));
            app.selected_xref = Some(0);
        }

        app
    }

    pub fn next_instruction(&mut self) {
        if self.filtered_instructions.is_empty() {
            return;
        }

        if let Some(current_pos) = self.instruction_list_state.selected() {
            let next_pos =
                (current_pos + 1).min(self.filtered_instructions.len().saturating_sub(1));
            self.instruction_list_state.select(Some(next_pos));

            if let Some(&instruction_idx) = self.filtered_instructions.get(next_pos) {
                if instruction_idx < self.instructions.len() {
                    self.selected_instruction = Some(instruction_idx);
                }
            }
        }
    }

    pub fn previous_instruction(&mut self) {
        if self.filtered_instructions.is_empty() {
            return;
        }

        if let Some(current_pos) = self.instruction_list_state.selected() {
            let prev_pos = current_pos.saturating_sub(1);
            self.instruction_list_state.select(Some(prev_pos));

            if let Some(&instruction_idx) = self.filtered_instructions.get(prev_pos) {
                if instruction_idx < self.instructions.len() {
                    self.selected_instruction = Some(instruction_idx);
                }
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::Functions,
            Tab::Functions => Tab::Names,
            Tab::Names => Tab::Xrefs,
            Tab::Xrefs => Tab::ControlFlow,
            Tab::ControlFlow => Tab::GraphView,
            Tab::GraphView => Tab::HexDump,
            Tab::HexDump => Tab::Instructions,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Instructions => Tab::HexDump,
            Tab::Functions => Tab::Instructions,
            Tab::Names => Tab::Functions,
            Tab::Xrefs => Tab::Names,
            Tab::ControlFlow => Tab::Xrefs,
            Tab::GraphView => Tab::ControlFlow,
            Tab::HexDump => Tab::GraphView,
        };
    }

    pub fn next_function(&mut self) {
        if self.functions.is_empty() {
            return;
        }

        if let Some(current) = self.function_list_state.selected() {
            let next = (current + 1).min(self.functions.len().saturating_sub(1));
            self.function_list_state.select(Some(next));
            self.selected_function = Some(next);
        }
    }

    pub fn previous_function(&mut self) {
        if self.functions.is_empty() {
            return;
        }

        if let Some(current) = self.function_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.function_list_state.select(Some(previous));
            self.selected_function = Some(previous);
        }
    }

    pub fn next_name(&mut self) {
        if self.names.is_empty() {
            return;
        }

        if let Some(current) = self.name_list_state.selected() {
            let next = (current + 1).min(self.names.len().saturating_sub(1));
            self.name_list_state.select(Some(next));
            self.selected_name = Some(next);
        }
    }

    pub fn previous_name(&mut self) {
        if self.names.is_empty() {
            return;
        }

        if let Some(current) = self.name_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.name_list_state.select(Some(previous));
            self.selected_name = Some(previous);
        }
    }

    pub fn next_xref(&mut self) {
        if self.xrefs.is_empty() {
            return;
        }

        if let Some(current) = self.xref_list_state.selected() {
            let next = (current + 1).min(self.xrefs.len().saturating_sub(1));
            self.xref_list_state.select(Some(next));
            self.selected_xref = Some(next);
        }
    }

    pub fn previous_xref(&mut self) {
        if self.xrefs.is_empty() {
            return;
        }

        if let Some(current) = self.xref_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.xref_list_state.select(Some(previous));
            self.selected_xref = Some(previous);
        }
    }

    pub fn jump_to_selected_function(&mut self) {
        let Some(function_idx) = self.selected_function else {
            return;
        };
        let Some(function) = self.functions.get(function_idx) else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_by_address(function.entry) else {
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
    }

    pub fn jump_to_selected_name(&mut self) {
        let Some(name_idx) = self.selected_name else {
            return;
        };
        let Some(address) = self.names.get(name_idx).and_then(NameItem::address) else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(Address(address)) else {
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
    }

    pub fn jump_to_selected_xref(&mut self) {
        let Some(xref_idx) = self.selected_xref else {
            return;
        };
        let Some(xref) = self.xrefs.get(xref_idx) else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(xref.from) else {
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
    }

    pub fn enter_address_jump_mode(&mut self) {
        self.address_jump_mode = true;
        self.address_jump_query.clear();
        self.status_message = Some("Enter address and press Enter".to_string());
    }

    pub fn exit_address_jump_mode(&mut self) {
        self.address_jump_mode = false;
        self.address_jump_query.clear();
    }

    pub fn jump_to_address_query(&mut self) {
        let query = self.address_jump_query.trim();
        let Some(address) = parse_address_query(query) else {
            self.status_message = Some(format!("Invalid address: {query}"));
            self.exit_address_jump_mode();
            return;
        };

        let Some(instruction_idx) = self.find_instruction_at_or_after(Address(address)) else {
            self.status_message = Some(format!("Address outside disassembly: {address:#x}"));
            self.exit_address_jump_mode();
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
        self.status_message = Some(format!("Jumped to {address:#x}"));
        self.exit_address_jump_mode();
    }

    pub fn go_back(&mut self) {
        let Some(location) = self.back_stack.pop() else {
            self.status_message = Some("No previous navigation location".to_string());
            return;
        };

        let current = self.current_location();
        self.forward_stack.push(current);
        self.restore_location(location);
    }

    pub fn go_forward(&mut self) {
        let Some(location) = self.forward_stack.pop() else {
            self.status_message = Some("No forward navigation location".to_string());
            return;
        };

        let current = self.current_location();
        self.back_stack.push(current);
        self.restore_location(location);
    }

    pub fn address_context(&self, address: Address) -> AddressContext {
        let block = self.block_containing_address(address);
        let reference_address = block.unwrap_or(address);

        let containing_function = self
            .functions
            .iter()
            .filter(|function| function.entry <= address)
            .max_by_key(|function| function.entry)
            .cloned();

        let nearest_name = self
            .names
            .iter()
            .filter_map(|name| Some((name.address()?, name)))
            .filter(|(name_address, _)| *name_address <= address.0)
            .max_by_key(|(name_address, _)| *name_address)
            .map(|(_, name)| name.clone());

        let incoming_xrefs = self
            .xrefs
            .iter()
            .filter(|xref| xref.to == reference_address)
            .cloned()
            .collect();

        let outgoing_xrefs = self
            .xrefs
            .iter()
            .filter(|xref| xref.from == reference_address)
            .cloned()
            .collect();

        AddressContext {
            address,
            block,
            containing_function,
            nearest_name,
            incoming_xrefs,
            outgoing_xrefs,
        }
    }

    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            self.status_message = Some("No search results".to_string());
            return;
        }

        let next = self
            .selected_search_match
            .map(|idx| (idx + 1) % self.search_matches.len())
            .unwrap_or(0);
        self.activate_search_match(next);
    }

    pub fn previous_search_match(&mut self) {
        if self.search_matches.is_empty() {
            self.status_message = Some("No search results".to_string());
            return;
        }

        let previous = self
            .selected_search_match
            .map(|idx| {
                if idx == 0 {
                    self.search_matches.len() - 1
                } else {
                    idx - 1
                }
            })
            .unwrap_or(0);
        self.activate_search_match(previous);
    }

    fn push_history(&mut self) {
        let current = self.current_location();
        if self.back_stack.last() != Some(&current) {
            self.back_stack.push(current);
        }
        self.forward_stack.clear();
    }

    fn current_location(&self) -> NavigationLocation {
        NavigationLocation {
            tab: self.current_tab.clone(),
            instruction: self.selected_instruction,
            function: self.selected_function,
            name: self.selected_name,
            xref: self.selected_xref,
            graph_block: self.graph_view.selected_block,
        }
    }

    fn restore_location(&mut self, location: NavigationLocation) {
        self.current_tab = location.tab;
        if let Some(instruction_idx) = location.instruction {
            self.select_instruction(instruction_idx);
        }
        self.selected_function = location.function.filter(|idx| *idx < self.functions.len());
        self.function_list_state.select(self.selected_function);
        self.selected_name = location.name.filter(|idx| *idx < self.names.len());
        self.name_list_state.select(self.selected_name);
        self.selected_xref = location.xref.filter(|idx| *idx < self.xrefs.len());
        self.xref_list_state.select(self.selected_xref);
        self.graph_view.selected_block = location.graph_block;
        self.status_message = Some("Restored navigation location".to_string());
    }

    fn select_instruction(&mut self, instruction_idx: usize) {
        if instruction_idx >= self.instructions.len() {
            return;
        }

        self.selected_instruction = Some(instruction_idx);
        if let Some(filtered_idx) = self
            .filtered_instructions
            .iter()
            .position(|idx| *idx == instruction_idx)
        {
            self.instruction_list_state.select(Some(filtered_idx));
        }
    }

    fn activate_search_match(&mut self, match_idx: usize) {
        let Some(search_match) = self.search_matches.get(match_idx).cloned() else {
            return;
        };

        self.push_history();
        self.selected_search_match = Some(match_idx);

        match search_match {
            SearchMatch::Instruction(instruction_idx) => {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
            }
            SearchMatch::Function(function_idx) if function_idx < self.functions.len() => {
                self.selected_function = Some(function_idx);
                self.function_list_state.select(Some(function_idx));
                self.current_tab = Tab::Functions;
            }
            SearchMatch::Name(name_idx) if name_idx < self.names.len() => {
                self.selected_name = Some(name_idx);
                self.name_list_state.select(Some(name_idx));
                self.current_tab = Tab::Names;
            }
            SearchMatch::Xref(xref_idx) if xref_idx < self.xrefs.len() => {
                self.selected_xref = Some(xref_idx);
                self.xref_list_state.select(Some(xref_idx));
                self.current_tab = Tab::Xrefs;
            }
            _ => {}
        }

        self.status_message = Some(format!(
            "Search result {}/{}",
            match_idx + 1,
            self.search_matches.len()
        ));
    }

    fn block_containing_address(&self, address: Address) -> Option<Address> {
        let cfg = self.cfg.as_ref()?;

        cfg.blocks.iter().find_map(|(block_addr, block)| {
            block
                .instructions
                .iter()
                .any(|instruction| instruction.address == address)
                .then_some(*block_addr)
        })
    }

    fn find_instruction_by_address(&self, address: Address) -> Option<usize> {
        self.instructions
            .binary_search_by_key(&address.0, |instruction| instruction.address)
            .ok()
    }

    fn find_instruction_at_or_after(&self, address: Address) -> Option<usize> {
        match self
            .instructions
            .binary_search_by_key(&address.0, |instruction| instruction.address)
        {
            Ok(idx) => Some(idx),
            Err(idx) if idx < self.instructions.len() => Some(idx),
            Err(_) => None,
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        // Reset to show all instructions
        self.filtered_instructions = (0..self.instructions.len()).collect();
        self.search_matches.clear();
        self.selected_search_match = None;
    }

    pub fn update_search(&mut self, query: String) {
        self.search_query = query;
        self.apply_filter();
    }

    pub(crate) fn apply_filter(&mut self) {
        // Only rebuild if search query actually changed
        if self.search_query == self.last_search_query {
            return;
        }

        self.last_search_query = self.search_query.clone();

        if self.search_query.is_empty() {
            self.filtered_instructions = (0..self.instructions.len()).collect();
            self.search_matches.clear();
            self.selected_search_match = None;
        } else {
            // Pre-lowercase search query once
            let search_lower = self.search_query.to_lowercase();

            self.filtered_instructions = (0..self.instructions.len())
                .filter(|&i| {
                    // Use cached display strings instead of formatting
                    self.instruction_display_cache[i]
                        .to_lowercase()
                        .contains(&search_lower)
                })
                .collect();
            self.search_matches = self.collect_search_matches(&search_lower);
            self.selected_search_match = (!self.search_matches.is_empty()).then_some(0);
        }

        // Update selection safely
        if let Some(selected) = self.selected_instruction {
            if !self.filtered_instructions.contains(&selected) {
                if let Some(&first) = self.filtered_instructions.first() {
                    self.selected_instruction = Some(first);
                    self.instruction_list_state.select(Some(0));
                } else {
                    self.selected_instruction = None;
                    self.instruction_list_state.select(None);
                }
            }
        }
    }

    fn collect_search_matches(&self, query: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();

        matches.extend((0..self.instructions.len()).filter_map(|idx| {
            self.instruction_display_cache[idx]
                .to_lowercase()
                .contains(query)
                .then_some(SearchMatch::Instruction(idx))
        }));

        matches.extend(
            self.functions
                .iter()
                .enumerate()
                .filter_map(|(idx, function)| {
                    let text = format!(
                        "{:#x} blocks:{} instructions:{} callers:{}",
                        function.entry.0,
                        function.block_count,
                        function.instruction_count,
                        function.caller_count
                    );
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::Function(idx))
                }),
        );

        matches.extend(self.names.iter().enumerate().filter_map(|(idx, name)| {
            let address = name
                .address()
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "unknown".to_string());
            let text = format!("{} {} {}", name.kind(), address, name.label());
            text.to_lowercase()
                .contains(query)
                .then_some(SearchMatch::Name(idx))
        }));

        matches.extend(self.xrefs.iter().enumerate().filter_map(|(idx, xref)| {
            let text = format!("{} {} {}", xref.kind(), xref.from, xref.to);
            text.to_lowercase()
                .contains(query)
                .then_some(SearchMatch::Xref(idx))
        }));

        matches
    }
}

fn build_name_items(analysis: &BinaryAnalysis) -> Vec<NameItem> {
    let mut names = Vec::with_capacity(
        analysis.imports.len() + analysis.symbols.len() + analysis.strings.len(),
    );

    names.extend(analysis.imports.iter().cloned().map(NameItem::Import));
    names.extend(analysis.symbols.iter().cloned().map(NameItem::Symbol));
    names.extend(analysis.strings.iter().cloned().map(NameItem::String));
    names.sort_by_key(|item| (item.address().unwrap_or(u64::MAX), item.kind()));
    names
}

fn build_xref_items(cfg: &ControlFlowGraph) -> Vec<XrefItem> {
    let mut xrefs: Vec<XrefItem> = cfg
        .edges
        .iter()
        .filter(|edge| edge.edge_type != EdgeType::Return)
        .map(|edge| XrefItem {
            from: edge.from,
            to: edge.to,
            edge_type: edge.edge_type.clone(),
        })
        .collect();

    xrefs.sort_by_key(|xref| (xref.to, xref.from));
    xrefs
}

fn truncate_for_display(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        truncated.push_str("...");
    }
    truncated
}

fn parse_address_query(query: &str) -> Option<u64> {
    let query = query.trim();
    let query = query
        .strip_prefix("0x")
        .or_else(|| query.strip_prefix("0X"))
        .unwrap_or(query);

    if query.is_empty() {
        return None;
    }

    u64::from_str_radix(query, 16)
        .ok()
        .or_else(|| query.parse::<u64>().ok())
}
