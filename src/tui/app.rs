use ratatui::widgets::ListState;

use crate::arch::x86::Instruction;
use crate::graph::{
    Address, CallGraph, ControlFlowGraph, EdgeType, ExternalCallTarget, FunctionSummary,
};
use crate::graph_renderer::GraphRenderer;
use crate::graph_view::{GraphScope, GraphView};
use crate::parser::{
    BinaryAnalysis, BinaryMetadata, ImportSummary, SectionSummary, StringSummary, SymbolSummary,
};
use crate::project::{AnalysisProject, Bookmark, UserName};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Overview,
    Instructions,
    Functions,
    CallGraph,
    Imports,
    Exports,
    Symbols,
    Names,
    Strings,
    Data,
    Relocations,
    Sections,
    Xrefs,
    Bookmarks,
    ControlFlow,
    GraphView,
    HexDump,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameItem {
    User(UserName),
    Import(ImportSummary),
    Symbol(SymbolSummary),
    String(StringSummary),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XrefKind {
    ControlFlow(EdgeType),
    Import,
    Symbol,
    String,
    DataPointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefItem {
    pub from: Address,
    pub to: Address,
    pub kind: XrefKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationLocation {
    pub tab: Tab,
    pub instruction: Option<usize>,
    pub function: Option<usize>,
    pub call_graph: Option<usize>,
    pub import: Option<usize>,
    pub export: Option<usize>,
    pub symbol: Option<usize>,
    pub name: Option<usize>,
    pub string: Option<usize>,
    pub data: Option<usize>,
    pub relocation: Option<usize>,
    pub section: Option<usize>,
    pub xref: Option<usize>,
    pub bookmark: Option<usize>,
    pub graph_block: Option<Address>,
    pub graph_scope: GraphScope,
    pub hex_address: Option<Address>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddressContext {
    pub address: Address,
    pub block: Option<Address>,
    pub containing_function: Option<FunctionSummary>,
    pub nearest_name: Option<NameItem>,
    pub containing_section: Option<SectionSummary>,
    pub user_name: Option<String>,
    pub comment: Option<String>,
    pub bookmark: Option<Bookmark>,
    pub incoming_xrefs: Vec<XrefItem>,
    pub outgoing_xrefs: Vec<XrefItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchMatch {
    Overview,
    Instruction(usize),
    Function(usize),
    CallGraph(usize),
    Import(usize),
    Export(usize),
    Symbol(usize),
    Name(usize),
    String(usize),
    Data(usize),
    Relocation(usize),
    Section(usize),
    Xref(usize),
    Bookmark(usize),
}

impl XrefKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            XrefKind::ControlFlow(EdgeType::Call) => "call",
            XrefKind::ControlFlow(EdgeType::ConditionalTrue) => "true",
            XrefKind::ControlFlow(EdgeType::ConditionalFalse) => "false",
            XrefKind::ControlFlow(EdgeType::Unconditional) => "jump",
            XrefKind::ControlFlow(EdgeType::Return) => "return",
            XrefKind::Import => "import",
            XrefKind::Symbol => "symbol",
            XrefKind::String => "string",
            XrefKind::DataPointer => "data",
        }
    }

    fn sort_rank(&self) -> u8 {
        match self {
            XrefKind::ControlFlow(EdgeType::Call) => 0,
            XrefKind::ControlFlow(EdgeType::ConditionalTrue) => 1,
            XrefKind::ControlFlow(EdgeType::ConditionalFalse) => 2,
            XrefKind::ControlFlow(EdgeType::Unconditional) => 3,
            XrefKind::ControlFlow(EdgeType::Return) => 4,
            XrefKind::Import => 5,
            XrefKind::Symbol => 6,
            XrefKind::String => 7,
            XrefKind::DataPointer => 8,
        }
    }
}

impl XrefItem {
    pub(crate) fn kind(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl NameItem {
    pub(crate) fn address(&self) -> Option<u64> {
        match self {
            NameItem::User(user_name) => Some(user_name.address),
            NameItem::Import(import) => import.address,
            NameItem::Symbol(symbol) => symbol.address,
            NameItem::String(string) => Some(string.address),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            NameItem::User(_) => "user",
            NameItem::Import(_) => "import",
            NameItem::Symbol(symbol) => symbol.kind.as_str(),
            NameItem::String(_) => "string",
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            NameItem::User(user_name) => user_name.name.clone(),
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
    pub binary: Vec<u8>,
    pub metadata: Option<BinaryMetadata>,
    pub cfg: Option<ControlFlowGraph>,
    pub call_graph: Option<CallGraph>,
    pub analysis: BinaryAnalysis,
    pub project: Option<AnalysisProject>,
    pub project_dirty: bool,
    pub current_tab: Tab,
    pub instruction_list_state: ListState,
    pub function_list_state: ListState,
    pub call_graph_list_state: ListState,
    pub import_list_state: ListState,
    pub export_list_state: ListState,
    pub symbol_list_state: ListState,
    pub name_list_state: ListState,
    pub string_list_state: ListState,
    pub data_list_state: ListState,
    pub relocation_list_state: ListState,
    pub section_list_state: ListState,
    pub xref_list_state: ListState,
    pub bookmark_list_state: ListState,
    pub selected_instruction: Option<usize>,
    pub selected_function: Option<usize>,
    pub selected_call_graph: Option<usize>,
    pub selected_import: Option<usize>,
    pub selected_export: Option<usize>,
    pub selected_symbol: Option<usize>,
    pub selected_name: Option<usize>,
    pub selected_string: Option<usize>,
    pub selected_data: Option<usize>,
    pub selected_relocation: Option<usize>,
    pub selected_section: Option<usize>,
    pub selected_xref: Option<usize>,
    pub selected_bookmark: Option<usize>,
    pub selected_hex_address: Option<Address>,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub address_jump_mode: bool,
    pub address_jump_query: String,
    pub rename_mode: bool,
    pub rename_query: String,
    pub comment_mode: bool,
    pub comment_query: String,
    pub status_message: Option<String>,
    pub filtered_instructions: Vec<usize>,
    pub search_matches: Vec<SearchMatch>,
    pub selected_search_match: Option<usize>,
    pub instruction_display_cache: Vec<String>,
    pub last_search_query: String,
    pub functions: Vec<FunctionSummary>,
    pub names: Vec<NameItem>,
    pub sections: Vec<SectionSummary>,
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
        Self::with_project(instructions, cfg, analysis, None)
    }

    pub fn with_project(
        instructions: Vec<Instruction>,
        cfg: Option<ControlFlowGraph>,
        analysis: BinaryAnalysis,
        project: Option<AnalysisProject>,
    ) -> Self {
        Self::with_binary_and_project(instructions, Vec::new(), cfg, analysis, project)
    }

    pub fn with_binary_and_project(
        instructions: Vec<Instruction>,
        binary: Vec<u8>,
        cfg: Option<ControlFlowGraph>,
        analysis: BinaryAnalysis,
        project: Option<AnalysisProject>,
    ) -> Self {
        Self::with_binary_metadata_and_project(instructions, binary, None, cfg, analysis, project)
    }

    pub fn with_binary_metadata_and_project(
        instructions: Vec<Instruction>,
        binary: Vec<u8>,
        metadata: Option<BinaryMetadata>,
        cfg: Option<ControlFlowGraph>,
        analysis: BinaryAnalysis,
        project: Option<AnalysisProject>,
    ) -> Self {
        let instruction_display_cache: Vec<String> = instructions
            .iter()
            .map(|instr| format!("{:#08x}: {}", instr.address, instr.text))
            .collect();

        let graph_view = GraphView::new();
        let functions = cfg
            .as_ref()
            .map(ControlFlowGraph::function_summaries)
            .unwrap_or_default();
        let call_graph = cfg.as_ref().map(|cfg| {
            cfg.call_graph_with_external_targets(build_external_call_targets(&analysis))
        });
        let names = build_name_items(&analysis, project.as_ref());
        let sections = analysis.sections.clone();
        let xrefs = build_xref_items(cfg.as_ref(), &instructions, &analysis);

        let mut app = Self {
            instructions,
            binary,
            metadata,
            cfg,
            call_graph,
            analysis,
            project,
            project_dirty: false,
            current_tab: Tab::Overview,
            instruction_list_state: ListState::default(),
            function_list_state: ListState::default(),
            call_graph_list_state: ListState::default(),
            import_list_state: ListState::default(),
            export_list_state: ListState::default(),
            symbol_list_state: ListState::default(),
            name_list_state: ListState::default(),
            string_list_state: ListState::default(),
            data_list_state: ListState::default(),
            relocation_list_state: ListState::default(),
            section_list_state: ListState::default(),
            xref_list_state: ListState::default(),
            bookmark_list_state: ListState::default(),
            selected_instruction: None,
            selected_function: None,
            selected_call_graph: None,
            selected_import: None,
            selected_export: None,
            selected_symbol: None,
            selected_name: None,
            selected_string: None,
            selected_data: None,
            selected_relocation: None,
            selected_section: None,
            selected_xref: None,
            selected_bookmark: None,
            selected_hex_address: None,
            scroll_offset: 0,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            address_jump_mode: false,
            address_jump_query: String::new(),
            rename_mode: false,
            rename_query: String::new(),
            comment_mode: false,
            comment_query: String::new(),
            status_message: None,
            filtered_instructions: Vec::new(),
            search_matches: Vec::new(),
            selected_search_match: None,
            instruction_display_cache,
            last_search_query: String::new(),
            functions,
            names,
            sections,
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

        if app.call_graph_function_count() > 0 {
            app.call_graph_list_state.select(Some(0));
            app.selected_call_graph = Some(0);
        }

        if !app.analysis.imports.is_empty() {
            app.import_list_state.select(Some(0));
            app.selected_import = Some(0);
        }

        if !app.analysis.exports.is_empty() {
            app.export_list_state.select(Some(0));
            app.selected_export = Some(0);
        }

        if !app.analysis.symbols.is_empty() {
            app.symbol_list_state.select(Some(0));
            app.selected_symbol = Some(0);
        }

        if !app.names.is_empty() {
            app.name_list_state.select(Some(0));
            app.selected_name = Some(0);
        }

        if !app.analysis.strings.is_empty() {
            app.string_list_state.select(Some(0));
            app.selected_string = Some(0);
        }

        if !app.analysis.data_objects.is_empty() {
            app.data_list_state.select(Some(0));
            app.selected_data = Some(0);
        }

        if !app.analysis.relocations.is_empty() {
            app.relocation_list_state.select(Some(0));
            app.selected_relocation = Some(0);
        }

        if !app.sections.is_empty() {
            app.section_list_state.select(Some(0));
            app.selected_section = Some(0);
        }

        if !app.xrefs.is_empty() {
            app.xref_list_state.select(Some(0));
            app.selected_xref = Some(0);
        }

        if app.bookmark_count() > 0 {
            app.bookmark_list_state.select(Some(0));
            app.selected_bookmark = Some(0);
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

    pub fn set_current_tab(&mut self, tab: Tab) {
        let address = self.current_address();
        self.current_tab = tab;
        if self.current_tab == Tab::HexDump {
            if let Some(address) = address {
                self.selected_hex_address = Some(address);
            }
        }
        if self.current_tab == Tab::GraphView {
            if let Some(address) = address {
                self.sync_graph_selection_to_address(address);
            }
        }
    }

    pub fn next_tab(&mut self) {
        let next = match self.current_tab {
            Tab::Overview => Tab::Instructions,
            Tab::Instructions => Tab::Functions,
            Tab::Functions => Tab::CallGraph,
            Tab::CallGraph => Tab::Imports,
            Tab::Imports => Tab::Exports,
            Tab::Exports => Tab::Symbols,
            Tab::Symbols => Tab::Names,
            Tab::Names => Tab::Strings,
            Tab::Strings => Tab::Data,
            Tab::Data => Tab::Relocations,
            Tab::Relocations => Tab::Sections,
            Tab::Sections => Tab::Xrefs,
            Tab::Xrefs => Tab::Bookmarks,
            Tab::Bookmarks => Tab::ControlFlow,
            Tab::ControlFlow => Tab::GraphView,
            Tab::GraphView => Tab::HexDump,
            Tab::HexDump => Tab::Overview,
        };
        self.set_current_tab(next);
    }

    pub fn previous_tab(&mut self) {
        let previous = match self.current_tab {
            Tab::Overview => Tab::HexDump,
            Tab::Instructions => Tab::Overview,
            Tab::Functions => Tab::Instructions,
            Tab::CallGraph => Tab::Functions,
            Tab::Imports => Tab::CallGraph,
            Tab::Exports => Tab::Imports,
            Tab::Symbols => Tab::Exports,
            Tab::Names => Tab::Symbols,
            Tab::Strings => Tab::Names,
            Tab::Data => Tab::Strings,
            Tab::Relocations => Tab::Data,
            Tab::Sections => Tab::Relocations,
            Tab::Xrefs => Tab::Sections,
            Tab::Bookmarks => Tab::Xrefs,
            Tab::ControlFlow => Tab::Bookmarks,
            Tab::GraphView => Tab::ControlFlow,
            Tab::HexDump => Tab::GraphView,
        };
        self.set_current_tab(previous);
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

    pub fn next_call_graph_function(&mut self) {
        let count = self.call_graph_function_count();
        if count == 0 {
            return;
        }

        if let Some(current) = self.call_graph_list_state.selected() {
            let next = (current + 1).min(count.saturating_sub(1));
            self.call_graph_list_state.select(Some(next));
            self.selected_call_graph = Some(next);
        }
    }

    pub fn previous_call_graph_function(&mut self) {
        if self.call_graph_function_count() == 0 {
            return;
        }

        if let Some(current) = self.call_graph_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.call_graph_list_state.select(Some(previous));
            self.selected_call_graph = Some(previous);
        }
    }

    pub fn next_import(&mut self) {
        if self.analysis.imports.is_empty() {
            return;
        }

        if let Some(current) = self.import_list_state.selected() {
            let next = (current + 1).min(self.analysis.imports.len().saturating_sub(1));
            self.import_list_state.select(Some(next));
            self.selected_import = Some(next);
        }
    }

    pub fn previous_import(&mut self) {
        if self.analysis.imports.is_empty() {
            return;
        }

        if let Some(current) = self.import_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.import_list_state.select(Some(previous));
            self.selected_import = Some(previous);
        }
    }

    pub fn next_export(&mut self) {
        if self.analysis.exports.is_empty() {
            return;
        }

        if let Some(current) = self.export_list_state.selected() {
            let next = (current + 1).min(self.analysis.exports.len().saturating_sub(1));
            self.export_list_state.select(Some(next));
            self.selected_export = Some(next);
        }
    }

    pub fn previous_export(&mut self) {
        if self.analysis.exports.is_empty() {
            return;
        }

        if let Some(current) = self.export_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.export_list_state.select(Some(previous));
            self.selected_export = Some(previous);
        }
    }

    pub fn next_symbol(&mut self) {
        if self.analysis.symbols.is_empty() {
            return;
        }

        if let Some(current) = self.symbol_list_state.selected() {
            let next = (current + 1).min(self.analysis.symbols.len().saturating_sub(1));
            self.symbol_list_state.select(Some(next));
            self.selected_symbol = Some(next);
        }
    }

    pub fn previous_symbol(&mut self) {
        if self.analysis.symbols.is_empty() {
            return;
        }

        if let Some(current) = self.symbol_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.symbol_list_state.select(Some(previous));
            self.selected_symbol = Some(previous);
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

    pub fn next_string(&mut self) {
        if self.analysis.strings.is_empty() {
            return;
        }

        if let Some(current) = self.string_list_state.selected() {
            let next = (current + 1).min(self.analysis.strings.len().saturating_sub(1));
            self.string_list_state.select(Some(next));
            self.selected_string = Some(next);
        }
    }

    pub fn previous_string(&mut self) {
        if self.analysis.strings.is_empty() {
            return;
        }

        if let Some(current) = self.string_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.string_list_state.select(Some(previous));
            self.selected_string = Some(previous);
        }
    }

    pub fn next_data_object(&mut self) {
        if self.analysis.data_objects.is_empty() {
            return;
        }

        if let Some(current) = self.data_list_state.selected() {
            let next = (current + 1).min(self.analysis.data_objects.len().saturating_sub(1));
            self.data_list_state.select(Some(next));
            self.selected_data = Some(next);
        }
    }

    pub fn previous_data_object(&mut self) {
        if self.analysis.data_objects.is_empty() {
            return;
        }

        if let Some(current) = self.data_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.data_list_state.select(Some(previous));
            self.selected_data = Some(previous);
        }
    }

    pub fn next_relocation(&mut self) {
        if self.analysis.relocations.is_empty() {
            return;
        }

        if let Some(current) = self.relocation_list_state.selected() {
            let next = (current + 1).min(self.analysis.relocations.len().saturating_sub(1));
            self.relocation_list_state.select(Some(next));
            self.selected_relocation = Some(next);
        }
    }

    pub fn previous_relocation(&mut self) {
        if self.analysis.relocations.is_empty() {
            return;
        }

        if let Some(current) = self.relocation_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.relocation_list_state.select(Some(previous));
            self.selected_relocation = Some(previous);
        }
    }

    pub fn next_section(&mut self) {
        if self.sections.is_empty() {
            return;
        }

        if let Some(current) = self.section_list_state.selected() {
            let next = (current + 1).min(self.sections.len().saturating_sub(1));
            self.section_list_state.select(Some(next));
            self.selected_section = Some(next);
        }
    }

    pub fn previous_section(&mut self) {
        if self.sections.is_empty() {
            return;
        }

        if let Some(current) = self.section_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.section_list_state.select(Some(previous));
            self.selected_section = Some(previous);
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

    pub fn next_bookmark(&mut self) {
        let bookmark_count = self.bookmark_count();
        if bookmark_count == 0 {
            return;
        }

        if let Some(current) = self.bookmark_list_state.selected() {
            let next = (current + 1).min(bookmark_count.saturating_sub(1));
            self.bookmark_list_state.select(Some(next));
            self.selected_bookmark = Some(next);
        }
    }

    pub fn previous_bookmark(&mut self) {
        if self.bookmark_count() == 0 {
            return;
        }

        if let Some(current) = self.bookmark_list_state.selected() {
            let previous = current.saturating_sub(1);
            self.bookmark_list_state.select(Some(previous));
            self.selected_bookmark = Some(previous);
        }
    }

    pub fn jump_to_entry_point(&mut self) {
        let entry = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.entry_point)
            .map(Address)
            .or_else(|| {
                self.instructions
                    .first()
                    .map(|instruction| Address(instruction.address))
            });
        let Some(entry) = entry else {
            self.status_message = Some("No entry point available".to_string());
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(entry) else {
            self.status_message = Some(format!("Entry point outside disassembly: {:#x}", entry.0));
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
        self.status_message = Some(format!("Jumped to entry point {:#x}", entry.0));
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

    pub fn jump_to_selected_call_graph_function(&mut self) {
        let Some(function_idx) = self.selected_call_graph else {
            return;
        };
        let Some(function) = self
            .call_graph
            .as_ref()
            .and_then(|graph| graph.functions.get(function_idx))
        else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_by_address(function.summary.entry) else {
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
    }

    pub fn jump_to_selected_import(&mut self) {
        let Some(import_idx) = self.selected_import else {
            return;
        };
        let Some(import) = self.analysis.imports.get(import_idx) else {
            return;
        };
        let import_name = import.name.clone();
        let Some(address) = import.address.map(Address) else {
            self.status_message = Some(format!("Import {import_name} has no address"));
            return;
        };

        self.push_history();
        if let Some(instruction_idx) = self
            .xrefs
            .iter()
            .find(|xref| xref.kind == XrefKind::Import && xref.to == address)
            .and_then(|xref| self.find_instruction_at_or_after(xref.from))
        {
            self.select_instruction(instruction_idx);
            self.current_tab = Tab::Instructions;
            self.status_message = Some(format!("Jumped to import xref for {import_name}"));
            return;
        }

        if self.file_offset_for_address(address).is_some() {
            self.selected_hex_address = Some(address);
            self.current_tab = Tab::HexDump;
            self.status_message = Some(format!("Hex view at import slot {:#x}", address.0));
        } else {
            self.status_message = Some(format!(
                "Import address {:#x} is not file-backed",
                address.0
            ));
        }
    }
    pub fn jump_to_selected_export(&mut self) {
        let Some(export_idx) = self.selected_export else {
            return;
        };
        let Some(export) = self.analysis.exports.get(export_idx) else {
            return;
        };
        let export_name = export.name.clone();
        let Some(address) = export.address.map(Address) else {
            self.status_message = Some(format!("Export {export_name} has no address"));
            return;
        };

        self.push_history();
        if self
            .section_for_address(address)
            .is_some_and(|section| section.executable)
        {
            if let Some(instruction_idx) = self.find_instruction_at_or_after(address) {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
                self.status_message = Some(format!("Jumped to export {export_name}"));
                return;
            }
        }

        if self.file_offset_for_address(address).is_some() {
            self.selected_hex_address = Some(address);
            self.current_tab = Tab::HexDump;
            self.status_message = Some(format!("Hex view at export {export_name}"));
        } else {
            self.status_message = Some(format!(
                "Export address {:#x} is not file-backed",
                address.0
            ));
        }
    }

    pub fn jump_to_selected_symbol(&mut self) {
        let Some(symbol_idx) = self.selected_symbol else {
            return;
        };
        let Some(symbol) = self.analysis.symbols.get(symbol_idx) else {
            return;
        };
        let symbol_name = symbol.name.clone();
        let Some(address) = symbol.address.map(Address) else {
            self.status_message = Some(format!("Symbol {symbol_name} has no address"));
            return;
        };

        self.push_history();
        if self
            .section_for_address(address)
            .is_some_and(|section| section.executable)
        {
            if let Some(instruction_idx) = self.find_instruction_at_or_after(address) {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
                self.status_message = Some(format!("Jumped to symbol {symbol_name}"));
                return;
            }
        }

        if self.file_offset_for_address(address).is_some() {
            self.selected_hex_address = Some(address);
            self.current_tab = Tab::HexDump;
            self.status_message = Some(format!("Hex view at symbol {symbol_name}"));
        } else {
            self.status_message = Some(format!(
                "Symbol address {:#x} is not file-backed",
                address.0
            ));
        }
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

    pub fn jump_to_selected_string(&mut self) {
        let Some(string_idx) = self.selected_string else {
            return;
        };
        let Some(string_address) = self
            .analysis
            .strings
            .get(string_idx)
            .map(|string| string.address)
        else {
            return;
        };

        let address = Address(string_address);
        self.push_history();
        self.selected_hex_address = Some(address);
        self.current_tab = Tab::HexDump;
        self.status_message = Some(format!("Hex view at string {string_address:#x}"));
    }

    pub fn jump_to_selected_data_object(&mut self) {
        let Some(data_idx) = self.selected_data else {
            return;
        };
        let Some(object) = self.analysis.data_objects.get(data_idx) else {
            return;
        };
        let target = Address(object.target);
        let object_address = Address(object.address);

        self.push_history();
        if self
            .section_for_address(target)
            .is_some_and(|section| section.executable)
        {
            if let Some(instruction_idx) = self.find_instruction_at_or_after(target) {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
                self.status_message = Some(format!("Followed data pointer to {:#x}", target.0));
                return;
            }
        }

        self.selected_hex_address = Some(
            self.file_offset_for_address(target)
                .map(|_| target)
                .unwrap_or(object_address),
        );
        self.current_tab = Tab::HexDump;
        self.status_message = Some(format!("Hex view at data pointer target {:#x}", target.0));
    }

    pub fn jump_to_selected_relocation(&mut self) {
        let Some(relocation_idx) = self.selected_relocation else {
            return;
        };
        let Some(relocation) = self.analysis.relocations.get(relocation_idx) else {
            return;
        };
        let address = Address(relocation.address);
        let kind = relocation.kind.clone();

        self.push_history();
        if self
            .section_for_address(address)
            .is_some_and(|section| section.executable)
        {
            if let Some(instruction_idx) = self
                .find_instruction_containing_address(address)
                .or_else(|| self.find_instruction_at_or_after(address))
            {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
                self.status_message = Some(format!("Jumped to relocation {kind}"));
                return;
            }
        }

        if self.file_offset_for_address(address).is_some() {
            self.selected_hex_address = Some(address);
            self.current_tab = Tab::HexDump;
            self.status_message = Some(format!("Hex view at relocation {kind}"));
        } else {
            self.status_message = Some(format!(
                "Relocation address {:#x} is not file-backed",
                address.0
            ));
        }
    }

    pub fn jump_to_selected_section(&mut self) {
        let Some(section_idx) = self.selected_section else {
            return;
        };
        let Some(section) = self.sections.get(section_idx) else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(Address(section.address))
        else {
            self.status_message = Some(format!("Section {} has no disassembly", section.name));
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

    pub fn jump_to_selected_bookmark(&mut self) {
        let Some(bookmark_idx) = self.selected_bookmark else {
            return;
        };
        let Some(address) = self
            .project
            .as_ref()
            .and_then(|project| project.bookmarks.get(bookmark_idx))
            .map(|bookmark| bookmark.address)
        else {
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(Address(address)) else {
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
    }

    pub fn jump_to_selected_graph_block(&mut self) {
        let Some(block_address) = self.graph_view.selected_block else {
            self.status_message = Some("No graph block selected".to_string());
            return;
        };
        let Some(instruction_idx) = self.find_instruction_at_or_after(block_address) else {
            self.status_message = Some(format!("Graph block {} has no instruction", block_address));
            return;
        };

        self.push_history();
        self.select_instruction(instruction_idx);
        self.current_tab = Tab::Instructions;
        self.status_message = Some(format!("Jumped to graph block {}", block_address));
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

    pub fn enter_rename_mode(&mut self) {
        let Some(address) = self.current_address() else {
            self.status_message = Some("No address selected for rename".to_string());
            return;
        };
        if self.project.is_none() {
            self.status_message = Some("Project state is unavailable".to_string());
            return;
        }

        self.rename_mode = true;
        self.rename_query = self
            .project_user_name_at(address.0)
            .map(str::to_string)
            .unwrap_or_default();
        self.status_message = Some(format!("Rename {:#x}; Enter saves, ESC cancels", address.0));
    }

    pub fn exit_rename_mode(&mut self) {
        self.rename_mode = false;
        self.rename_query.clear();
    }

    pub fn apply_rename_query(&mut self) {
        let Some(address) = self.current_address() else {
            self.status_message = Some("No address selected for rename".to_string());
            self.exit_rename_mode();
            return;
        };
        let name = self.rename_query.trim().to_string();

        if let Some(project) = &mut self.project {
            if name.is_empty() {
                project.remove_user_name(address.0);
                self.status_message = Some(format!("Removed user name at {:#x}", address.0));
            } else {
                project.set_user_name(address.0, name.clone());
                self.status_message = Some(format!("Renamed {:#x} to {}", address.0, name));
            }
            self.project_dirty = true;
            self.refresh_project_views();
        } else {
            self.status_message = Some("Project state is unavailable".to_string());
        }

        self.exit_rename_mode();
    }

    pub fn enter_comment_mode(&mut self) {
        let Some(address) = self.current_address() else {
            self.status_message = Some("No address selected for comment".to_string());
            return;
        };
        if self.project.is_none() {
            self.status_message = Some("Project state is unavailable".to_string());
            return;
        }

        self.comment_mode = true;
        self.comment_query = self
            .project_comment_at(address.0)
            .map(str::to_string)
            .unwrap_or_default();
        self.status_message = Some(format!(
            "Comment {:#x}; Enter saves, ESC cancels",
            address.0
        ));
    }

    pub fn exit_comment_mode(&mut self) {
        self.comment_mode = false;
        self.comment_query.clear();
    }

    pub fn apply_comment_query(&mut self) {
        let Some(address) = self.current_address() else {
            self.status_message = Some("No address selected for comment".to_string());
            self.exit_comment_mode();
            return;
        };
        let comment = self.comment_query.trim().to_string();

        if let Some(project) = &mut self.project {
            if comment.is_empty() {
                project.remove_comment(address.0);
                self.status_message = Some(format!("Removed comment at {:#x}", address.0));
            } else {
                project.set_comment(address.0, comment);
                self.status_message = Some(format!("Saved comment at {:#x}", address.0));
            }
            self.project_dirty = true;
            self.refresh_project_views();
        } else {
            self.status_message = Some("Project state is unavailable".to_string());
        }

        self.exit_comment_mode();
    }

    pub fn toggle_bookmark_at_selection(&mut self) {
        let Some(address) = self.current_address() else {
            self.status_message = Some("No address selected for bookmark".to_string());
            return;
        };

        if let Some(project) = &mut self.project {
            let was_bookmarked = project.is_bookmarked(address.0);
            project.toggle_bookmark(address.0, None);
            self.project_dirty = true;
            self.refresh_project_views();
            self.status_message = Some(if was_bookmarked {
                format!("Removed bookmark at {:#x}", address.0)
            } else {
                format!("Bookmarked {:#x}", address.0)
            });
        } else {
            self.status_message = Some("Project state is unavailable".to_string());
        }
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

    fn sync_graph_selection_to_address(&mut self, address: Address) {
        let Some(cfg) = self.cfg.as_ref() else {
            return;
        };
        let Some(block) = self
            .block_containing_address(address)
            .or_else(|| cfg.blocks.contains_key(&address).then_some(address))
        else {
            return;
        };

        if let Some(function_entry) = cfg.function_entry_containing_address(address) {
            if matches!(self.graph_view.scope, GraphScope::Function(current) if current != function_entry)
            {
                self.graph_view
                    .set_scope(GraphScope::Function(function_entry));
            }
        }

        self.graph_view.selected_block = Some(block);
    }

    pub fn toggle_graph_scope(&mut self) {
        if self.cfg.is_none() {
            self.status_message = Some("No control flow graph available".to_string());
            return;
        }

        match self.graph_view.scope {
            GraphScope::WholeProgram => {
                let Some(entry) = self.current_function_entry_for_graph() else {
                    self.status_message =
                        Some("No containing function for graph scope".to_string());
                    return;
                };
                let block_count = self
                    .cfg
                    .as_ref()
                    .map(|cfg| cfg.function_block_addresses(entry).len())
                    .unwrap_or_default();
                if block_count == 0 {
                    self.status_message = Some(format!("Function {} has no graph blocks", entry));
                    return;
                }

                self.graph_view.selected_block = Some(entry);
                self.graph_view.set_scope(GraphScope::Function(entry));
                self.status_message = Some(format!(
                    "Graph scope: function {} ({} blocks)",
                    entry, block_count
                ));
            }
            GraphScope::Function(_) => {
                self.graph_view.set_scope(GraphScope::WholeProgram);
                self.status_message = Some("Graph scope: whole program".to_string());
            }
        }
    }

    fn current_function_entry_for_graph(&self) -> Option<Address> {
        match self.current_tab {
            Tab::Functions => self
                .selected_function
                .and_then(|idx| self.functions.get(idx))
                .map(|function| function.entry),
            Tab::CallGraph => self
                .selected_call_graph
                .and_then(|idx| self.call_graph.as_ref()?.functions.get(idx))
                .map(|function| function.summary.entry),
            _ => self.current_address().and_then(|address| {
                self.cfg
                    .as_ref()
                    .and_then(|cfg| cfg.function_entry_containing_address(address))
            }),
        }
    }

    pub fn address_context(&self, address: Address) -> AddressContext {
        let block = self.block_containing_address(address);
        let reference_address = block.unwrap_or(address);

        let containing_function = self
            .cfg
            .as_ref()
            .and_then(|cfg| cfg.function_entry_containing_address(address))
            .and_then(|entry| {
                self.functions
                    .iter()
                    .find(|function| function.entry == entry)
            })
            .cloned()
            .or_else(|| {
                self.functions
                    .iter()
                    .filter(|function| function.entry <= address)
                    .max_by_key(|function| function.entry)
                    .cloned()
            });

        let nearest_name = self
            .names
            .iter()
            .filter_map(|name| Some((name.address()?, name)))
            .filter(|(name_address, _)| *name_address <= address.0)
            .max_by_key(|(name_address, _)| *name_address)
            .map(|(_, name)| name.clone());

        let containing_section = self.analysis.section_containing_address(address.0).cloned();

        let incoming_xrefs = self
            .xrefs
            .iter()
            .filter(|xref| xref.to == reference_address || xref.to == address)
            .cloned()
            .collect();

        let outgoing_xrefs = self
            .xrefs
            .iter()
            .filter(|xref| xref.from == reference_address || xref.from == address)
            .cloned()
            .collect();

        AddressContext {
            address,
            block,
            containing_function,
            nearest_name,
            containing_section,
            user_name: self.project_user_name_at(address.0).map(str::to_string),
            comment: self.project_comment_at(address.0).map(str::to_string),
            bookmark: self.project_bookmark_at(address.0).cloned(),
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
            call_graph: self.selected_call_graph,
            import: self.selected_import,
            export: self.selected_export,
            symbol: self.selected_symbol,
            name: self.selected_name,
            string: self.selected_string,
            data: self.selected_data,
            relocation: self.selected_relocation,
            section: self.selected_section,
            xref: self.selected_xref,
            bookmark: self.selected_bookmark,
            graph_block: self.graph_view.selected_block,
            graph_scope: self.graph_view.scope,
            hex_address: self.selected_hex_address,
        }
    }

    fn restore_location(&mut self, location: NavigationLocation) {
        self.current_tab = location.tab;
        if let Some(instruction_idx) = location.instruction {
            self.select_instruction(instruction_idx);
        }
        self.selected_function = location.function.filter(|idx| *idx < self.functions.len());
        self.function_list_state.select(self.selected_function);
        self.selected_call_graph = location
            .call_graph
            .filter(|idx| *idx < self.call_graph_function_count());
        self.call_graph_list_state.select(self.selected_call_graph);
        self.selected_import = location
            .import
            .filter(|idx| *idx < self.analysis.imports.len());
        self.import_list_state.select(self.selected_import);
        self.selected_export = location
            .export
            .filter(|idx| *idx < self.analysis.exports.len());
        self.export_list_state.select(self.selected_export);
        self.selected_symbol = location
            .symbol
            .filter(|idx| *idx < self.analysis.symbols.len());
        self.symbol_list_state.select(self.selected_symbol);
        self.selected_name = location.name.filter(|idx| *idx < self.names.len());
        self.name_list_state.select(self.selected_name);
        self.selected_string = location
            .string
            .filter(|idx| *idx < self.analysis.strings.len());
        self.string_list_state.select(self.selected_string);
        self.selected_data = location
            .data
            .filter(|idx| *idx < self.analysis.data_objects.len());
        self.data_list_state.select(self.selected_data);
        self.selected_relocation = location
            .relocation
            .filter(|idx| *idx < self.analysis.relocations.len());
        self.relocation_list_state.select(self.selected_relocation);
        self.selected_section = location.section.filter(|idx| *idx < self.sections.len());
        self.section_list_state.select(self.selected_section);
        self.selected_xref = location.xref.filter(|idx| *idx < self.xrefs.len());
        self.xref_list_state.select(self.selected_xref);
        self.selected_bookmark = location.bookmark.filter(|idx| *idx < self.bookmark_count());
        self.bookmark_list_state.select(self.selected_bookmark);
        self.graph_view.set_scope(location.graph_scope);
        self.graph_view.selected_block = location.graph_block;
        self.selected_hex_address = location.hex_address;
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
        } else {
            self.filtered_instructions = (0..self.instructions.len()).collect();
            self.instruction_list_state.select(Some(instruction_idx));
        }
    }

    fn activate_search_match(&mut self, match_idx: usize) {
        let Some(search_match) = self.search_matches.get(match_idx).cloned() else {
            return;
        };

        self.push_history();
        self.selected_search_match = Some(match_idx);

        match search_match {
            SearchMatch::Overview => {
                self.current_tab = Tab::Overview;
            }
            SearchMatch::Instruction(instruction_idx) => {
                self.select_instruction(instruction_idx);
                self.current_tab = Tab::Instructions;
            }
            SearchMatch::Function(function_idx) if function_idx < self.functions.len() => {
                self.selected_function = Some(function_idx);
                self.function_list_state.select(Some(function_idx));
                self.current_tab = Tab::Functions;
            }
            SearchMatch::CallGraph(function_idx)
                if function_idx < self.call_graph_function_count() =>
            {
                self.selected_call_graph = Some(function_idx);
                self.call_graph_list_state.select(Some(function_idx));
                self.current_tab = Tab::CallGraph;
            }
            SearchMatch::Import(import_idx) if import_idx < self.analysis.imports.len() => {
                self.selected_import = Some(import_idx);
                self.import_list_state.select(Some(import_idx));
                self.current_tab = Tab::Imports;
            }
            SearchMatch::Export(export_idx) if export_idx < self.analysis.exports.len() => {
                self.selected_export = Some(export_idx);
                self.export_list_state.select(Some(export_idx));
                self.current_tab = Tab::Exports;
            }
            SearchMatch::Symbol(symbol_idx) if symbol_idx < self.analysis.symbols.len() => {
                self.selected_symbol = Some(symbol_idx);
                self.symbol_list_state.select(Some(symbol_idx));
                self.current_tab = Tab::Symbols;
            }
            SearchMatch::Name(name_idx) if name_idx < self.names.len() => {
                self.selected_name = Some(name_idx);
                self.name_list_state.select(Some(name_idx));
                self.current_tab = Tab::Names;
            }
            SearchMatch::String(string_idx) if string_idx < self.analysis.strings.len() => {
                self.selected_string = Some(string_idx);
                self.string_list_state.select(Some(string_idx));
                self.current_tab = Tab::Strings;
            }
            SearchMatch::Data(data_idx) if data_idx < self.analysis.data_objects.len() => {
                self.selected_data = Some(data_idx);
                self.data_list_state.select(Some(data_idx));
                self.current_tab = Tab::Data;
            }
            SearchMatch::Relocation(relocation_idx)
                if relocation_idx < self.analysis.relocations.len() =>
            {
                self.selected_relocation = Some(relocation_idx);
                self.relocation_list_state.select(Some(relocation_idx));
                self.current_tab = Tab::Relocations;
            }
            SearchMatch::Section(section_idx) if section_idx < self.sections.len() => {
                self.selected_section = Some(section_idx);
                self.section_list_state.select(Some(section_idx));
                self.current_tab = Tab::Sections;
            }
            SearchMatch::Xref(xref_idx) if xref_idx < self.xrefs.len() => {
                self.selected_xref = Some(xref_idx);
                self.xref_list_state.select(Some(xref_idx));
                self.current_tab = Tab::Xrefs;
            }
            SearchMatch::Bookmark(bookmark_idx) if bookmark_idx < self.bookmark_count() => {
                self.selected_bookmark = Some(bookmark_idx);
                self.bookmark_list_state.select(Some(bookmark_idx));
                self.current_tab = Tab::Bookmarks;
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

    fn find_instruction_containing_address(&self, address: Address) -> Option<usize> {
        self.instructions.iter().position(|instruction| {
            let end = instruction
                .address
                .saturating_add(instruction.bytes.len() as u64);
            address.0 >= instruction.address && address.0 < end
        })
    }

    pub fn bookmark_count(&self) -> usize {
        self.project
            .as_ref()
            .map_or(0, |project| project.bookmarks.len())
    }

    pub fn call_graph_function_count(&self) -> usize {
        self.call_graph
            .as_ref()
            .map_or(0, |graph| graph.functions.len())
    }

    pub fn current_address(&self) -> Option<Address> {
        match self.current_tab {
            Tab::Overview => self
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.entry_point)
                .map(Address)
                .or_else(|| {
                    self.selected_instruction
                        .and_then(|idx| self.instructions.get(idx))
                        .map(|instruction| Address(instruction.address))
                }),
            Tab::Instructions | Tab::ControlFlow => self
                .selected_instruction
                .and_then(|idx| self.instructions.get(idx))
                .map(|instruction| Address(instruction.address)),
            Tab::Functions => self
                .selected_function
                .and_then(|idx| self.functions.get(idx))
                .map(|function| function.entry),
            Tab::CallGraph => self
                .selected_call_graph
                .and_then(|idx| self.call_graph.as_ref()?.functions.get(idx))
                .map(|function| function.summary.entry),
            Tab::Imports => self
                .selected_import
                .and_then(|idx| self.analysis.imports.get(idx))
                .and_then(|import| import.address)
                .map(Address),
            Tab::Exports => self
                .selected_export
                .and_then(|idx| self.analysis.exports.get(idx))
                .and_then(|export| export.address)
                .map(Address),
            Tab::Symbols => self
                .selected_symbol
                .and_then(|idx| self.analysis.symbols.get(idx))
                .and_then(|symbol| symbol.address)
                .map(Address),
            Tab::Names => self
                .selected_name
                .and_then(|idx| self.names.get(idx))
                .and_then(NameItem::address)
                .map(Address),
            Tab::Strings => self
                .selected_string
                .and_then(|idx| self.analysis.strings.get(idx))
                .map(|string| Address(string.address)),
            Tab::Data => self
                .selected_data
                .and_then(|idx| self.analysis.data_objects.get(idx))
                .map(|object| Address(object.address)),
            Tab::Relocations => self
                .selected_relocation
                .and_then(|idx| self.analysis.relocations.get(idx))
                .map(|relocation| Address(relocation.address)),
            Tab::HexDump => self.selected_hex_address.or_else(|| {
                self.selected_instruction
                    .and_then(|idx| self.instructions.get(idx))
                    .map(|instruction| Address(instruction.address))
            }),
            Tab::Sections => self
                .selected_section
                .and_then(|idx| self.sections.get(idx))
                .map(|section| Address(section.address)),
            Tab::Xrefs => self
                .selected_xref
                .and_then(|idx| self.xrefs.get(idx))
                .map(|xref| xref.from),
            Tab::Bookmarks => self
                .selected_bookmark
                .and_then(|idx| self.project.as_ref()?.bookmarks.get(idx))
                .map(|bookmark| Address(bookmark.address)),
            Tab::GraphView => self.graph_view.selected_block,
        }
    }

    pub(crate) fn binary_path_display(&self) -> String {
        self.project
            .as_ref()
            .map(|project| project.binary.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub(crate) fn binary_size(&self) -> u64 {
        self.project
            .as_ref()
            .map(|project| project.binary.size)
            .unwrap_or(self.binary.len() as u64)
    }

    pub(crate) fn binary_fingerprint(&self) -> Option<&str> {
        self.project
            .as_ref()
            .map(|project| project.binary.fingerprint.as_str())
    }

    pub(crate) fn executable_section_count(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.executable)
            .count()
    }

    pub(crate) fn writable_section_count(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.writable)
            .count()
    }

    pub(crate) fn data_section_count(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.is_string_candidate())
            .count()
    }

    pub(crate) fn overview_search_text(&self) -> String {
        let metadata = self
            .metadata
            .as_ref()
            .map(|metadata| {
                format!(
                    "{} {} {} {} entry:{:#x}",
                    metadata.format,
                    metadata.architecture,
                    metadata.bitness,
                    metadata.endianness,
                    metadata.entry_point.unwrap_or_default()
                )
            })
            .unwrap_or_else(|| "metadata unknown".to_string());
        format!(
            "overview {} path:{} fingerprint:{} size:{} instructions:{} functions:{} imports:{} noreturn:{} symbols:{} strings:{} data:{} relocations:{} unwind:{} sections:{} xrefs:{} bookmarks:{}",
            metadata,
            self.binary_path_display(),
            self.binary_fingerprint().unwrap_or("unknown"),
            self.binary_size(),
            self.instructions.len(),
            self.call_graph
                .as_ref()
                .map_or(self.functions.len(), |graph| graph.functions.len()),
            self.analysis.imports.len(),
            self.cfg
                .as_ref()
                .map_or(0, ControlFlowGraph::noreturn_call_target_count),
            self.analysis.symbols.len(),
            self.analysis.strings.len(),
            self.analysis.data_objects.len(),
            self.analysis.relocations.len(),
            self.analysis.function_ranges.len(),
            self.sections.len(),
            self.xrefs.len(),
            self.bookmark_count()
        )
    }
    pub fn file_offset_for_address(&self, address: Address) -> Option<usize> {
        let offset = self.analysis.file_offset_for_va(address.0)?;
        let offset = usize::try_from(offset).ok()?;
        (offset < self.binary.len()).then_some(offset)
    }

    pub fn bytes_at_file_offset(&self, file_offset: usize, len: usize) -> Option<&[u8]> {
        let end = file_offset.saturating_add(len).min(self.binary.len());
        (file_offset < end).then_some(&self.binary[file_offset..end])
    }

    pub fn section_for_address(&self, address: Address) -> Option<&SectionSummary> {
        self.analysis.section_containing_address(address.0)
    }

    pub fn project_user_name_at(&self, address: u64) -> Option<&str> {
        self.project
            .as_ref()
            .and_then(|project| project.user_name_at(address))
            .map(|user_name| user_name.name.as_str())
    }

    pub fn project_comment_at(&self, address: u64) -> Option<&str> {
        self.project
            .as_ref()
            .and_then(|project| project.comment_at(address))
            .map(|comment| comment.text.as_str())
    }

    pub fn project_bookmark_at(&self, address: u64) -> Option<&Bookmark> {
        self.project
            .as_ref()
            .and_then(|project| project.bookmark_at(address))
    }

    pub fn instruction_display_text(&self, instruction_idx: usize) -> String {
        let Some(instruction) = self.instructions.get(instruction_idx) else {
            return String::new();
        };

        let mut text = self
            .instruction_display_cache
            .get(instruction_idx)
            .cloned()
            .unwrap_or_else(|| format!("{:#08x}: {}", instruction.address, instruction.text));

        if let Some(name) = self.project_user_name_at(instruction.address) {
            text.push_str("  <");
            text.push_str(name);
            text.push('>');
        }

        if self.project_bookmark_at(instruction.address).is_some() {
            text.push_str("  [*]");
        }

        if let Some(comment) = self.project_comment_at(instruction.address) {
            text.push_str("  ; ");
            text.push_str(comment);
        }

        text
    }

    fn refresh_project_views(&mut self) {
        self.names = build_name_items(&self.analysis, self.project.as_ref());
        if self.names.is_empty() {
            self.selected_name = None;
            self.name_list_state.select(None);
        } else {
            let selected = self
                .selected_name
                .unwrap_or(0)
                .min(self.names.len().saturating_sub(1));
            self.selected_name = Some(selected);
            self.name_list_state.select(Some(selected));
        }

        self.refresh_bookmark_selection();

        if !self.search_query.is_empty() {
            self.last_search_query.clear();
            self.apply_filter();
        }
    }

    fn refresh_bookmark_selection(&mut self) {
        let bookmark_count = self.bookmark_count();
        if bookmark_count == 0 {
            self.selected_bookmark = None;
            self.bookmark_list_state.select(None);
        } else {
            let selected = self
                .selected_bookmark
                .unwrap_or(0)
                .min(bookmark_count.saturating_sub(1));
            self.selected_bookmark = Some(selected);
            self.bookmark_list_state.select(Some(selected));
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
                    self.instruction_display_text(i)
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

        if self.overview_search_text().to_lowercase().contains(query) {
            matches.push(SearchMatch::Overview);
        }

        matches.extend((0..self.instructions.len()).filter_map(|idx| {
            self.instruction_display_text(idx)
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

        if let Some(call_graph) = &self.call_graph {
            matches.extend(call_graph.functions.iter().enumerate().filter_map(
                |(idx, function)| {
                    let entry = function.summary.entry.0;
                    let name = self.project_user_name_at(entry).unwrap_or("");
                    let import_thunk = function
                        .import_thunk
                        .as_ref()
                        .map(|target| target.label.as_str())
                        .unwrap_or("");
                    let text = format!(
                        "call graph {:#x} {} {} incoming:{} outgoing:{} callers:{} calls:{}",
                        entry,
                        name,
                        import_thunk,
                        function.incoming_call_count,
                        function.outgoing_call_count,
                        function.incoming_call_count,
                        function.outgoing_call_count
                    );
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::CallGraph(idx))
                },
            ));
        }

        matches.extend(
            self.analysis
                .imports
                .iter()
                .enumerate()
                .filter_map(|(idx, import)| {
                    let address = import
                        .address
                        .map(|address| format!("{address:#x}"))
                        .unwrap_or_else(|| "unknown".to_string());
                    let library = import.library.as_deref().unwrap_or("unknown");
                    let text = format!("import {} {} {}", address, library, import.name);
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::Import(idx))
                }),
        );
        matches.extend(
            self.analysis
                .exports
                .iter()
                .enumerate()
                .filter_map(|(idx, export)| {
                    let address = export
                        .address
                        .map(|address| format!("{address:#x}"))
                        .unwrap_or_else(|| "unknown".to_string());
                    let forwarder = export.forwarder.as_deref().unwrap_or("");
                    let text = format!(
                        "export {} {} {} {}",
                        export.kind.as_str(),
                        address,
                        export.name,
                        forwarder
                    );
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::Export(idx))
                }),
        );
        matches.extend(
            self.analysis
                .symbols
                .iter()
                .enumerate()
                .filter_map(|(idx, symbol)| {
                    let address = symbol
                        .address
                        .map(|address| format!("{address:#x}"))
                        .unwrap_or_else(|| "unknown".to_string());
                    let text = format!(
                        "symbol {} {} {}",
                        symbol.kind.as_str(),
                        address,
                        symbol.name
                    );
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::Symbol(idx))
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

        matches.extend(
            self.analysis
                .strings
                .iter()
                .enumerate()
                .filter_map(|(idx, string)| {
                    let section = string.section.as_deref().unwrap_or("unknown");
                    let text = format!("string {:#x} {} {}", string.address, section, string.value);
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::String(idx))
                }),
        );

        matches.extend(self.analysis.data_objects.iter().enumerate().filter_map(
            |(idx, object)| {
                let section = object.section.as_deref().unwrap_or("unknown");
                let label = object
                    .target_label
                    .as_deref()
                    .or(object.target_section.as_deref())
                    .unwrap_or("unknown");
                let text = format!(
                    "data {} {:#x} {:#x} {} {}",
                    object.kind.as_str(),
                    object.address,
                    object.target,
                    section,
                    label
                );
                text.to_lowercase()
                    .contains(query)
                    .then_some(SearchMatch::Data(idx))
            },
        ));

        matches.extend(self.analysis.relocations.iter().enumerate().filter_map(
            |(idx, relocation)| {
                let section = relocation.section.as_deref().unwrap_or("unknown");
                let symbol = relocation.symbol.as_deref().unwrap_or("");
                let addend = relocation
                    .addend
                    .map(|addend| format!("{addend:#x}"))
                    .unwrap_or_default();
                let text = format!(
                    "relocation reloc {:#x} {} {} type:{} source:{} symbol:{} addend:{}",
                    relocation.address,
                    section,
                    relocation.kind,
                    relocation.type_id,
                    relocation.source,
                    symbol,
                    addend
                );
                text.to_lowercase()
                    .contains(query)
                    .then_some(SearchMatch::Relocation(idx))
            },
        ));

        matches.extend(
            self.sections
                .iter()
                .enumerate()
                .filter_map(|(idx, section)| {
                    let text = format!(
                        "section {} {:#x} {:#x} {}",
                        section.name,
                        section.address,
                        section.end_address(),
                        section.permissions()
                    );
                    text.to_lowercase()
                        .contains(query)
                        .then_some(SearchMatch::Section(idx))
                }),
        );

        matches.extend(self.xrefs.iter().enumerate().filter_map(|(idx, xref)| {
            let text = format!(
                "{} {} {} {}",
                xref.kind(),
                xref.from,
                xref.to,
                xref.label.as_deref().unwrap_or("")
            );
            text.to_lowercase()
                .contains(query)
                .then_some(SearchMatch::Xref(idx))
        }));

        if let Some(project) = &self.project {
            matches.extend(project.comments.iter().filter_map(|comment| {
                let text = format!("comment {:#x} {}", comment.address, comment.text);
                if !text.to_lowercase().contains(query) {
                    return None;
                }
                self.find_instruction_at_or_after(Address(comment.address))
                    .map(SearchMatch::Instruction)
            }));

            matches.extend(
                project
                    .bookmarks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, bookmark)| {
                        let user_name = self.project_user_name_at(bookmark.address).unwrap_or("");
                        let comment = self.project_comment_at(bookmark.address).unwrap_or("");
                        let label = bookmark.label.as_deref().unwrap_or("");
                        let text = format!(
                            "bookmark {:#x} {} {} {}",
                            bookmark.address, label, user_name, comment
                        );
                        text.to_lowercase()
                            .contains(query)
                            .then_some(SearchMatch::Bookmark(idx))
                    }),
            );
        }

        matches
    }
}

fn build_external_call_targets(analysis: &BinaryAnalysis) -> Vec<ExternalCallTarget> {
    analysis
        .imports
        .iter()
        .filter_map(|import| {
            let address = import.address?;
            let library = import.library.as_deref().unwrap_or("unknown");
            Some(ExternalCallTarget {
                address: Address(address),
                label: format!("{library}!{}", import.name),
            })
        })
        .collect()
}

fn build_name_items(analysis: &BinaryAnalysis, project: Option<&AnalysisProject>) -> Vec<NameItem> {
    let user_name_count = project.map_or(0, |project| project.user_names.len());
    let mut names =
        Vec::with_capacity(user_name_count + analysis.imports.len() + analysis.symbols.len());

    if let Some(project) = project {
        names.extend(project.user_names.iter().cloned().map(NameItem::User));
    }
    names.extend(analysis.imports.iter().cloned().map(NameItem::Import));
    names.extend(analysis.symbols.iter().cloned().map(NameItem::Symbol));
    names.sort_by_key(|item| (item.address().unwrap_or(u64::MAX), item.kind()));
    names
}

fn build_xref_items(
    cfg: Option<&ControlFlowGraph>,
    instructions: &[Instruction],
    analysis: &BinaryAnalysis,
) -> Vec<XrefItem> {
    let mut xrefs = Vec::new();

    if let Some(cfg) = cfg {
        xrefs.extend(
            cfg.edges
                .iter()
                .filter(|edge| edge.edge_type != EdgeType::Return)
                .map(|edge| XrefItem {
                    from: edge.from,
                    to: edge.to,
                    kind: XrefKind::ControlFlow(edge.edge_type.clone()),
                    label: None,
                }),
        );
    }

    xrefs.extend(analysis.data_objects.iter().map(|object| {
        XrefItem {
            from: Address(object.address),
            to: Address(object.target),
            kind: XrefKind::DataPointer,
            label: object
                .target_label
                .clone()
                .or_else(|| object.target_section.clone()),
        }
    }));

    let targets = build_named_xref_targets(analysis);
    for instruction in instructions {
        for candidate in extract_address_candidates(&instruction.text) {
            for target in targets.iter().filter(|target| target.contains(candidate)) {
                let xref = XrefItem {
                    from: Address(instruction.address),
                    to: Address(target.address),
                    kind: target.kind.clone(),
                    label: Some(target.label.clone()),
                };

                if !xrefs.iter().any(|existing| {
                    existing.from == xref.from
                        && existing.to == xref.to
                        && existing.kind == xref.kind
                        && existing.label == xref.label
                }) {
                    xrefs.push(xref);
                }
            }
        }
    }

    xrefs.sort_by(|left, right| {
        (
            left.to,
            left.from,
            left.kind.sort_rank(),
            left.label.as_deref().unwrap_or(""),
        )
            .cmp(&(
                right.to,
                right.from,
                right.kind.sort_rank(),
                right.label.as_deref().unwrap_or(""),
            ))
    });
    xrefs
}

#[derive(Debug, Clone)]
struct NamedXrefTarget {
    address: u64,
    end: u64,
    kind: XrefKind,
    label: String,
}

impl NamedXrefTarget {
    fn contains(&self, address: u64) -> bool {
        address >= self.address && address < self.end
    }
}

fn build_named_xref_targets(analysis: &BinaryAnalysis) -> Vec<NamedXrefTarget> {
    let mut targets = Vec::new();

    targets.extend(analysis.imports.iter().filter_map(|import| {
        let address = import.address?;
        let library = import.library.as_deref().unwrap_or("unknown");
        Some(NamedXrefTarget {
            address,
            end: address.saturating_add(1),
            kind: XrefKind::Import,
            label: format!("{library}!{}", import.name),
        })
    }));

    targets.extend(analysis.symbols.iter().filter_map(|symbol| {
        let address = symbol.address?;
        Some(NamedXrefTarget {
            address,
            end: address.saturating_add(1),
            kind: XrefKind::Symbol,
            label: symbol.name.clone(),
        })
    }));

    targets.extend(analysis.strings.iter().map(|string| {
        NamedXrefTarget {
            address: string.address,
            end: string
                .address
                .saturating_add((string.value.len() as u64).max(1)),
            kind: XrefKind::String,
            label: truncate_for_display(&string.value, 72),
        }
    }));

    targets.sort_by_key(|target| {
        (
            target.address,
            target.kind.sort_rank(),
            target.label.clone(),
        )
    });
    targets
}

fn extract_address_candidates(text: &str) -> Vec<u64> {
    let mut candidates = Vec::new();

    for token in
        text.split(|ch: char| !(ch.is_ascii_hexdigit() || matches!(ch, 'x' | 'X' | 'h' | 'H')))
    {
        let Some(address) = parse_hex_address_token(token) else {
            continue;
        };
        if address < 0x1000 {
            continue;
        }
        if !candidates.contains(&address) {
            candidates.push(address);
        }
    }

    candidates
}

fn parse_hex_address_token(token: &str) -> Option<u64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        if hex.is_empty() || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return None;
        }
        return u64::from_str_radix(hex, 16).ok();
    }

    let hex = token
        .strip_suffix('h')
        .or_else(|| token.strip_suffix('H'))?;
    if hex.is_empty() || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    u64::from_str_radix(hex, 16).ok()
}

pub(crate) fn truncate_for_display(value: &str, max_chars: usize) -> String {
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
