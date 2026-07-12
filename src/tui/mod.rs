pub mod app;
mod input;
mod session;
mod views;

pub use app::{App, NameItem, SearchMatch, Tab, XrefItem, XrefKind};
pub use session::{
    run_tui, run_tui_with_project, run_tui_with_project_and_binary,
    run_tui_with_project_binary_and_metadata,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::Instruction;
    use crate::arch::{Architecture, Endianness};
    use crate::graph::ControlFlowGraph;
    use crate::graph::{Address, Instruction as CfgInstruction};
    use crate::graph_view::{GraphScope, NavigationDirection};
    use crate::parser::{BinaryAnalysis, BinaryFormat, BinaryMetadata};
    use crate::parser::{
        DataObjectKind, DataObjectSummary, ExportSummary, ImportSummary, RelocationSummary,
        SectionSummary, StringSummary, SymbolKind, SymbolSummary,
    };
    use crate::project::AnalysisProject;

    fn instruction(address: u64, text: &str, size: usize) -> Instruction {
        Instruction {
            address,
            bytes: vec![0x90; size],
            text: text.to_string(),
        }
    }

    fn cfg_instruction(
        address: u64,
        mnemonic: &str,
        operands: &str,
        size: usize,
    ) -> CfgInstruction {
        CfgInstruction {
            address: Address(address),
            mnemonic: mnemonic.to_string(),
            operands: operands.to_string(),
            bytes: vec![0x90; size],
        }
    }

    #[test]
    fn overview_starts_first_searches_metadata_and_jumps_to_entry() {
        let metadata = BinaryMetadata {
            format: BinaryFormat::PE,
            architecture: Architecture::X64,
            bitness: 64,
            endianness: Endianness::Little,
            entry_point: Some(0x1000),
        };
        let analysis = BinaryAnalysis {
            sections: vec![SectionSummary {
                name: ".text".to_string(),
                address: 0x1000,
                virtual_size: 0x10,
                file_offset: 0,
                file_size: 0x10,
                readable: true,
                writable: false,
                executable: true,
            }],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_metadata_and_project(
            vec![instruction(0x1000, "nop", 1), instruction(0x1001, "ret", 1)],
            vec![0x90, 0xc3],
            Some(metadata),
            None,
            analysis,
            Some(AnalysisProject::from_binary("sample.exe", b"\x90\xc3")),
        );

        assert_eq!(app.current_tab, Tab::Overview);
        assert_eq!(app.current_address(), Some(Address(0x1000)));
        assert!(app.overview_search_text().contains("x86_64"));
        assert_eq!(app.executable_section_count(), 1);

        app.update_search("x86_64".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::Overview)));
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Overview);
        app.jump_to_entry_point();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
        assert_eq!(app.instruction_list_state.selected(), Some(0));
    }
    #[test]
    fn function_jump_selects_entry_instruction() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        let callee_idx = app
            .functions
            .iter()
            .position(|function| function.entry == Address(0x2000))
            .unwrap();

        app.current_tab = Tab::Functions;
        app.selected_function = Some(callee_idx);
        app.function_list_state.select(Some(callee_idx));
        app.jump_to_selected_function();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(2));
        assert_eq!(app.instruction_list_state.selected(), Some(2));
        assert_eq!(app.back_stack.len(), 1);
    }

    #[test]
    fn name_jump_selects_nearest_instruction() {
        let analysis = BinaryAnalysis {
            symbols: vec![SymbolSummary {
                address: Some(0x2001),
                name: "interesting_function".to_string(),
                kind: SymbolKind::Function,
            }],
            strings: vec![StringSummary {
                address: 0x3000,
                section: Some(".rdata".to_string()),
                value: "hello".to_string(),
            }],
            ..BinaryAnalysis::default()
        };

        let mut app = App::new(
            vec![
                instruction(0x1000, "nop", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2004, "mov rbp,rsp", 3),
            ],
            None,
            analysis,
        );

        let symbol_idx = app
            .names
            .iter()
            .position(|item| item.label() == "interesting_function")
            .unwrap();

        app.current_tab = Tab::Names;
        app.selected_name = Some(symbol_idx);
        app.name_list_state.select(Some(symbol_idx));
        app.jump_to_selected_name();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(2));
        assert_eq!(app.instruction_list_state.selected(), Some(2));
    }

    #[test]
    fn xref_jump_selects_source_instruction() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        let xref_idx = app
            .xrefs
            .iter()
            .position(|xref| xref.from == Address(0x1000) && xref.to == Address(0x2000))
            .unwrap();

        app.current_tab = Tab::Xrefs;
        app.selected_xref = Some(xref_idx);
        app.xref_list_state.select(Some(xref_idx));
        app.jump_to_selected_xref();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
        assert_eq!(app.instruction_list_state.selected(), Some(0));
    }

    #[test]
    fn navigation_back_restores_previous_tab_and_selection() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        let callee_idx = app
            .functions
            .iter()
            .position(|function| function.entry == Address(0x2000))
            .unwrap();

        app.current_tab = Tab::Functions;
        app.selected_function = Some(callee_idx);
        app.function_list_state.select(Some(callee_idx));
        app.jump_to_selected_function();
        app.go_back();

        assert_eq!(app.current_tab, Tab::Functions);
        assert_eq!(app.selected_function, Some(callee_idx));
        assert_eq!(app.function_list_state.selected(), Some(callee_idx));
        assert_eq!(app.forward_stack.len(), 1);
    }

    #[test]
    fn address_query_jumps_to_nearest_instruction() {
        let mut app = App::new(
            vec![
                instruction(0x1000, "nop", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2004, "mov rbp,rsp", 3),
            ],
            None,
            BinaryAnalysis::default(),
        );

        app.enter_address_jump_mode();
        app.address_jump_query = "2001".to_string();
        app.jump_to_address_query();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(2));
        assert_eq!(app.instruction_list_state.selected(), Some(2));
        assert!(!app.address_jump_mode);
        assert_eq!(app.back_stack.len(), 1);
    }

    #[test]
    fn address_context_reports_function_name_and_xrefs() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let analysis = BinaryAnalysis {
            symbols: vec![SymbolSummary {
                address: Some(0x2000),
                name: "callee".to_string(),
                kind: SymbolKind::Function,
            }],
            ..BinaryAnalysis::default()
        };

        let app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
            analysis,
        );

        let context = app.address_context(Address(0x2001));

        assert_eq!(context.block, Some(Address(0x2000)));
        assert_eq!(
            context
                .containing_function
                .as_ref()
                .map(|function| function.entry),
            Some(Address(0x2000))
        );
        assert_eq!(
            context.nearest_name.as_ref().map(NameItem::label),
            Some("callee".to_string())
        );
        assert!(context
            .incoming_xrefs
            .iter()
            .any(|xref| xref.from == Address(0x1000) && xref.to == Address(0x2000)));
    }

    #[test]
    fn search_matches_names_and_jumps_results() {
        let analysis = BinaryAnalysis {
            symbols: vec![SymbolSummary {
                address: Some(0x2000),
                name: "callee".to_string(),
                kind: SymbolKind::Function,
            }],
            ..BinaryAnalysis::default()
        };

        let mut app = App::new(
            vec![
                instruction(0x1000, "nop", 1),
                instruction(0x2000, "push rbp", 1),
            ],
            None,
            analysis,
        );

        app.update_search("callee".to_string());

        let name_match_idx = app
            .search_matches
            .iter()
            .position(|search_match| matches!(search_match, SearchMatch::Name(_)))
            .expect("name search result should be present");
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::Symbol(_))));
        app.selected_search_match = Some(if name_match_idx == 0 {
            app.search_matches.len() - 1
        } else {
            name_match_idx - 1
        });
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Names);
        assert_eq!(app.selected_name, Some(0));
        assert_eq!(app.name_list_state.selected(), Some(0));
    }

    #[test]
    fn project_annotations_feed_names_context_and_instruction_text() {
        let mut project = AnalysisProject::from_binary("sample.exe", b"binary");
        project.set_user_name(0x2000, "entry_point");
        project.set_comment(0x2000, "manual note");
        project.toggle_bookmark(0x2000, None);

        let app = App::with_project(
            vec![instruction(0x2000, "push rbp", 1)],
            None,
            BinaryAnalysis::default(),
            Some(project),
        );

        assert!(app
            .names
            .iter()
            .any(|name| name.kind() == "user" && name.label() == "entry_point"));
        assert!(app.instruction_display_text(0).contains("<entry_point>"));
        assert!(app.instruction_display_text(0).contains("[*]"));
        assert!(app.instruction_display_text(0).contains("manual note"));

        let context = app.address_context(Address(0x2000));
        assert_eq!(context.user_name.as_deref(), Some("entry_point"));
        assert_eq!(context.comment.as_deref(), Some("manual note"));
        assert!(context.bookmark.is_some());
    }

    #[test]
    fn bookmark_jump_and_toggle_update_project() {
        let mut project = AnalysisProject::from_binary("sample.exe", b"binary");
        project.toggle_bookmark(0x2000, None);

        let mut app = App::with_project(
            vec![instruction(0x1000, "nop", 1), instruction(0x2000, "ret", 1)],
            None,
            BinaryAnalysis::default(),
            Some(project),
        );

        app.current_tab = Tab::Bookmarks;
        app.selected_bookmark = Some(0);
        app.bookmark_list_state.select(Some(0));
        app.jump_to_selected_bookmark();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(1));
        assert_eq!(app.back_stack.len(), 1);

        app.toggle_bookmark_at_selection();

        assert_eq!(app.bookmark_count(), 0);
        assert!(app.project_dirty);
    }

    #[test]
    fn rename_and_comment_commands_update_project_and_search() {
        let project = AnalysisProject::from_binary("sample.exe", b"binary");
        let mut app = App::with_project(
            vec![instruction(0x1000, "nop", 1)],
            None,
            BinaryAnalysis::default(),
            Some(project),
        );

        app.rename_query = "main_entry".to_string();
        app.apply_rename_query();
        app.comment_query = "start here".to_string();
        app.apply_comment_query();

        let project = app.project.as_ref().unwrap();
        assert_eq!(project.user_name_at(0x1000).unwrap().name, "main_entry");
        assert_eq!(project.comment_at(0x1000).unwrap().text, "start here");
        assert!(app.project_dirty);

        app.update_search("main_entry".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::Name(_))));
    }

    #[test]
    fn app_maps_selected_va_to_file_bytes() {
        let analysis = BinaryAnalysis {
            sections: vec![SectionSummary {
                name: ".text".to_string(),
                address: 0x1000,
                virtual_size: 0x10,
                file_offset: 0x04,
                file_size: 0x10,
                readable: true,
                writable: false,
                executable: true,
            }],
            ..BinaryAnalysis::default()
        };

        let app = App::with_binary_and_project(
            vec![instruction(0x1002, "nop", 1)],
            vec![0, 1, 2, 3, 0xaa, 0xbb, 0xcc, 0xdd],
            None,
            analysis,
            None,
        );

        assert_eq!(app.file_offset_for_address(Address(0x1002)), Some(6));
        assert_eq!(
            app.bytes_at_file_offset(4, 4),
            Some(&[0xaa, 0xbb, 0xcc, 0xdd][..])
        );
        assert_eq!(
            app.section_for_address(Address(0x1002))
                .map(|section| section.name.as_str()),
            Some(".text")
        );
    }

    #[test]
    fn section_tab_tracks_context_search_and_jump() {
        let analysis = BinaryAnalysis {
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x40,
                    file_offset: 0x400,
                    file_size: 0x40,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".rdata".to_string(),
                    address: 0x3000,
                    virtual_size: 0x20,
                    file_offset: 0x800,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::new(
            vec![instruction(0x1000, "nop", 1), instruction(0x1010, "ret", 1)],
            None,
            analysis,
        );

        assert_eq!(app.sections.len(), 2);
        assert_eq!(app.section_list_state.selected(), Some(0));
        assert_eq!(
            app.address_context(Address(0x1010))
                .containing_section
                .as_ref()
                .map(|section| section.name.as_str()),
            Some(".text")
        );

        app.current_tab = Tab::Sections;
        app.selected_section = Some(0);
        app.section_list_state.select(Some(0));
        app.jump_to_selected_section();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
        assert_eq!(app.back_stack.len(), 1);

        app.update_search("rdata".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::Section(_))));
    }

    #[test]
    fn strings_tab_searches_and_opens_mapped_hex_view() {
        let analysis = BinaryAnalysis {
            strings: vec![StringSummary {
                address: 0x3000,
                section: Some(".rdata".to_string()),
                value: "hello data".to_string(),
            }],
            sections: vec![SectionSummary {
                name: ".rdata".to_string(),
                address: 0x3000,
                virtual_size: 0x20,
                file_offset: 0x04,
                file_size: 0x20,
                readable: true,
                writable: false,
                executable: false,
            }],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![instruction(0x1000, "nop", 1)],
            b"xxxxhello data\0padding".to_vec(),
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_string, Some(0));
        assert_eq!(app.string_list_state.selected(), Some(0));

        app.update_search("hello data".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::String(_))));
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Strings);
        assert_eq!(app.selected_string, Some(0));

        app.jump_to_selected_string();

        assert_eq!(app.current_tab, Tab::HexDump);
        assert_eq!(app.current_address(), Some(Address(0x3000)));
        assert_eq!(app.file_offset_for_address(Address(0x3000)), Some(0x04));
    }

    #[test]
    fn data_tab_searches_xrefs_and_follows_executable_targets() {
        let mut binary = vec![0u8; 0x20];
        binary[0x10..0x18].copy_from_slice(&0x1000u64.to_le_bytes());
        let analysis = BinaryAnalysis {
            data_objects: vec![DataObjectSummary {
                address: 0x3000,
                section: Some(".rdata".to_string()),
                kind: DataObjectKind::Pointer,
                size: 8,
                value: 0x1000,
                target: 0x1000,
                target_section: Some(".text".to_string()),
                target_label: Some("entry".to_string()),
            }],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x10,
                    file_offset: 0,
                    file_size: 0x10,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".rdata".to_string(),
                    address: 0x3000,
                    virtual_size: 0x10,
                    file_offset: 0x10,
                    file_size: 0x10,
                    readable: true,
                    writable: false,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![
                instruction(0x1000, "push rbp", 1),
                instruction(0x1001, "ret", 1),
            ],
            binary,
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_data, Some(0));
        assert_eq!(app.data_list_state.selected(), Some(0));
        assert!(app.xrefs.iter().any(|xref| {
            xref.from == Address(0x3000)
                && xref.to == Address(0x1000)
                && xref.kind == XrefKind::DataPointer
                && xref.label.as_deref() == Some("entry")
        }));
        assert!(app
            .address_context(Address(0x1000))
            .incoming_xrefs
            .iter()
            .any(|xref| xref.kind == XrefKind::DataPointer));

        app.update_search("pointer 0x3000".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::Data(_))));
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Data);
        assert_eq!(app.selected_data, Some(0));

        app.jump_to_selected_data_object();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
        assert_eq!(app.instruction_list_state.selected(), Some(0));
    }
    #[test]
    fn exports_tab_searches_and_jumps_to_code_or_hex() {
        let analysis = BinaryAnalysis {
            exports: vec![
                ExportSummary {
                    address: Some(0x1000),
                    name: "DllMain".to_string(),
                    kind: SymbolKind::Export,
                    size: 12,
                    forwarder: None,
                },
                ExportSummary {
                    address: Some(0x3000),
                    name: "ForwardedThing".to_string(),
                    kind: SymbolKind::Export,
                    size: 0,
                    forwarder: Some("OTHERDLL.RealThing".to_string()),
                },
            ],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x20,
                    file_offset: 0,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".edata".to_string(),
                    address: 0x3000,
                    virtual_size: 0x20,
                    file_offset: 0x20,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![
                instruction(0x1000, "push rbp", 1),
                instruction(0x1001, "ret", 1),
            ],
            vec![0; 0x40],
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_export, Some(0));
        assert_eq!(app.export_list_state.selected(), Some(0));

        app.update_search("ForwardedThing".to_string());
        assert!(matches!(
            app.search_matches.first(),
            Some(SearchMatch::Export(_))
        ));
        app.selected_search_match = None;
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Exports);
        assert_eq!(app.selected_export, Some(1));
        app.jump_to_selected_export();

        assert_eq!(app.current_tab, Tab::HexDump);
        assert_eq!(app.current_address(), Some(Address(0x3000)));

        app.current_tab = Tab::Exports;
        app.selected_export = Some(0);
        app.export_list_state.select(Some(0));
        app.jump_to_selected_export();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
    }

    #[test]
    fn relocations_tab_searches_and_jumps_to_code_or_hex() {
        let analysis = BinaryAnalysis {
            relocations: vec![
                RelocationSummary {
                    address: 0x3000,
                    section: Some(".data".to_string()),
                    source: "PE base".to_string(),
                    kind: "DIR64".to_string(),
                    type_id: 10,
                    symbol: None,
                    addend: None,
                },
                RelocationSummary {
                    address: 0x1001,
                    section: Some(".text".to_string()),
                    source: "ELF PLT".to_string(),
                    kind: "X86_64_PC32".to_string(),
                    type_id: 2,
                    symbol: Some("puts".to_string()),
                    addend: Some(-4),
                },
            ],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x20,
                    file_offset: 0,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".data".to_string(),
                    address: 0x3000,
                    virtual_size: 0x20,
                    file_offset: 0x20,
                    file_size: 0x20,
                    readable: true,
                    writable: true,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![
                instruction(0x1000, "mov rax, [rip + 1]", 5),
                instruction(0x1005, "ret", 1),
            ],
            vec![0; 0x40],
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_relocation, Some(0));
        assert_eq!(app.relocation_list_state.selected(), Some(0));

        app.update_search("DIR64".to_string());
        assert!(matches!(
            app.search_matches.first(),
            Some(SearchMatch::Relocation(_))
        ));
        app.selected_search_match = None;
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Relocations);
        assert_eq!(app.selected_relocation, Some(0));
        app.jump_to_selected_relocation();

        assert_eq!(app.current_tab, Tab::HexDump);
        assert_eq!(app.current_address(), Some(Address(0x3000)));

        app.current_tab = Tab::Relocations;
        app.selected_relocation = Some(1);
        app.relocation_list_state.select(Some(1));
        app.jump_to_selected_relocation();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
    }

    #[test]
    fn symbols_tab_searches_and_jumps_to_code_or_hex() {
        let analysis = BinaryAnalysis {
            symbols: vec![
                SymbolSummary {
                    address: Some(0x1000),
                    name: "main_entry".to_string(),
                    kind: SymbolKind::Function,
                },
                SymbolSummary {
                    address: Some(0x3000),
                    name: "global_counter".to_string(),
                    kind: SymbolKind::Object,
                },
            ],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x20,
                    file_offset: 0,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".data".to_string(),
                    address: 0x3000,
                    virtual_size: 0x20,
                    file_offset: 0x20,
                    file_size: 0x20,
                    readable: true,
                    writable: true,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![
                instruction(0x1000, "push rbp", 1),
                instruction(0x1001, "ret", 1),
            ],
            vec![0; 0x40],
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_symbol, Some(0));
        assert_eq!(app.symbol_list_state.selected(), Some(0));

        app.update_search("global_counter".to_string());
        assert!(matches!(
            app.search_matches.first(),
            Some(SearchMatch::Symbol(_))
        ));
        app.selected_search_match = None;
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Symbols);
        assert_eq!(app.selected_symbol, Some(1));
        app.jump_to_selected_symbol();

        assert_eq!(app.current_tab, Tab::HexDump);
        assert_eq!(app.current_address(), Some(Address(0x3000)));

        app.current_tab = Tab::Symbols;
        app.selected_symbol = Some(0);
        app.symbol_list_state.select(Some(0));
        app.jump_to_selected_symbol();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
    }
    #[test]
    fn imports_tab_searches_xrefs_and_jumps_to_first_referrer() {
        let analysis = BinaryAnalysis {
            imports: vec![ImportSummary {
                address: Some(0x3000),
                library: Some("KERNEL32.dll".to_string()),
                name: "ExitProcess".to_string(),
            }],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x20,
                    file_offset: 0,
                    file_size: 0x20,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".idata".to_string(),
                    address: 0x3000,
                    virtual_size: 0x20,
                    file_offset: 0x20,
                    file_size: 0x20,
                    readable: true,
                    writable: true,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let mut app = App::with_binary_and_project(
            vec![
                instruction(0x1000, "call qword [0000000000003000h]", 6),
                instruction(0x1006, "ret", 1),
            ],
            vec![0; 0x40],
            None,
            analysis,
            None,
        );

        assert_eq!(app.selected_import, Some(0));
        assert_eq!(app.import_list_state.selected(), Some(0));
        assert!(app.xrefs.iter().any(|xref| {
            xref.from == Address(0x1000)
                && xref.to == Address(0x3000)
                && xref.kind == XrefKind::Import
                && xref.label.as_deref() == Some("KERNEL32.dll!ExitProcess")
        }));

        app.update_search("ExitProcess".to_string());
        assert!(matches!(
            app.search_matches.first(),
            Some(SearchMatch::Import(_))
        ));
        app.selected_search_match = None;
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Imports);
        assert_eq!(app.selected_import, Some(0));

        app.jump_to_selected_import();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(0));
        assert_eq!(app.instruction_list_state.selected(), Some(0));
    }
    #[test]
    fn xrefs_include_string_import_and_symbol_references() {
        let analysis = BinaryAnalysis {
            imports: vec![ImportSummary {
                address: Some(0x3000),
                library: Some("KERNEL32.dll".to_string()),
                name: "ExitProcess".to_string(),
            }],
            symbols: vec![SymbolSummary {
                address: Some(0x4000),
                name: "global_counter".to_string(),
                kind: SymbolKind::Object,
            }],
            strings: vec![StringSummary {
                address: 0x5000,
                section: Some(".rdata".to_string()),
                value: "hello world".to_string(),
            }],
            ..BinaryAnalysis::default()
        };

        let app = App::new(
            vec![
                instruction(0x1000, "call qword [0000000000003000h]", 6),
                instruction(0x1006, "mov rax,[0000000000004000h]", 7),
                instruction(0x100d, "lea rcx,[0000000000005006h]", 7),
            ],
            None,
            analysis,
        );

        assert!(app.xrefs.iter().any(|xref| {
            xref.from == Address(0x1000)
                && xref.to == Address(0x3000)
                && xref.kind == XrefKind::Import
                && xref.label.as_deref() == Some("KERNEL32.dll!ExitProcess")
        }));
        assert!(app.xrefs.iter().any(|xref| {
            xref.from == Address(0x1006)
                && xref.to == Address(0x4000)
                && xref.kind == XrefKind::Symbol
                && xref.label.as_deref() == Some("global_counter")
        }));
        assert!(app.xrefs.iter().any(|xref| {
            xref.from == Address(0x100d)
                && xref.to == Address(0x5000)
                && xref.kind == XrefKind::String
                && xref.label.as_deref() == Some("hello world")
        }));

        let context = app.address_context(Address(0x100d));
        assert!(context
            .outgoing_xrefs
            .iter()
            .any(|xref| xref.kind == XrefKind::String));
    }

    #[test]
    fn search_matches_xrefs_and_cycles_results() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        app.update_search("0x00002000".to_string());
        assert_eq!(app.search_matches.len(), 1);

        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Xrefs);
        assert_eq!(
            app.selected_xref
                .and_then(|idx| app.xrefs.get(idx))
                .map(|xref| xref.to),
            Some(Address(0x2000))
        );
        assert_eq!(app.selected_search_match, Some(0));
    }
    #[test]
    fn call_graph_tab_tracks_search_and_jump() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "call", "0000000000003000h", 5),
            cfg_instruction(0x100a, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "call", "0000000000003000h", 5),
            cfg_instruction(0x2009, "ret", "", 1),
            cfg_instruction(0x3000, "push", "rbp", 1),
            cfg_instruction(0x3001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x3004, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "call 0000000000003000h", 5),
                instruction(0x100a, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "call 0000000000003000h", 5),
                instruction(0x2009, "ret", 1),
                instruction(0x3000, "push rbp", 1),
                instruction(0x3001, "mov rbp,rsp", 3),
                instruction(0x3004, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        let call_graph = app.call_graph.as_ref().unwrap();
        assert_eq!(call_graph.functions.len(), 3);
        assert_eq!(call_graph.edges.len(), 3);
        assert!(call_graph.edges.iter().any(|edge| {
            edge.caller == Address(0x2000)
                && edge.callee == Address(0x3000)
                && edge.call_sites == vec![Address(0x2004)]
        }));

        app.update_search("incoming:2".to_string());
        assert!(app
            .search_matches
            .iter()
            .any(|search_match| matches!(search_match, SearchMatch::CallGraph(_))));

        let callee_idx = app
            .call_graph
            .as_ref()
            .unwrap()
            .functions
            .iter()
            .position(|function| function.summary.entry == Address(0x3000))
            .unwrap();
        app.current_tab = Tab::CallGraph;
        app.selected_call_graph = Some(callee_idx);
        app.call_graph_list_state.select(Some(callee_idx));
        app.jump_to_selected_call_graph_function();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(7));
        assert_eq!(app.instruction_list_state.selected(), Some(7));
        assert_eq!(app.back_stack.len(), 1);
    }
    #[test]
    fn graph_view_starts_lazy_and_toggles_function_scope() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "je", "0000000000002010h", 2),
            cfg_instruction(0x2006, "ret", "", 1),
            cfg_instruction(0x2010, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "je 0000000000002010h", 2),
                instruction(0x2006, "ret", 1),
                instruction(0x2010, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        assert_eq!(app.graph_view.scope, GraphScope::WholeProgram);
        assert!(!app.graph_view.layout_computed);
        assert!(app.graph_view.blocks.is_empty());

        app.selected_instruction = Some(3);
        app.instruction_list_state.select(Some(3));
        app.toggle_graph_scope();

        assert_eq!(app.graph_view.scope, GraphScope::Function(Address(0x2000)));
        assert!(!app.graph_view.layout_computed);
        assert!(app.graph_view.blocks.is_empty());

        let cfg = app.cfg.as_ref().unwrap();
        app.graph_view.build_from_cfg(cfg);

        assert!(app.graph_view.layout_computed);
        assert_eq!(app.graph_view.blocks.len(), 3);
        assert!(app.graph_view.blocks.contains_key(&Address(0x2000)));
        assert!(app.graph_view.blocks.contains_key(&Address(0x2006)));
        assert!(app.graph_view.blocks.contains_key(&Address(0x2010)));
        assert_eq!(app.graph_view.selected_block, Some(Address(0x2000)));

        app.graph_view
            .move_selection(cfg, NavigationDirection::Down);
        assert_eq!(app.graph_view.selected_block, Some(Address(0x2006)));

        app.toggle_graph_scope();
        assert_eq!(app.graph_view.scope, GraphScope::WholeProgram);
        assert!(!app.graph_view.layout_computed);
        assert!(app.graph_view.blocks.is_empty());
    }
    #[test]
    fn graph_view_syncs_from_instruction_and_jumps_back() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "je", "0000000000002010h", 2),
            cfg_instruction(0x2006, "ret", "", 1),
            cfg_instruction(0x2010, "ret", "", 1),
        ]);

        let mut app = App::new(
            vec![
                instruction(0x1000, "call 0000000000002000h", 5),
                instruction(0x1005, "ret", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x2001, "mov rbp,rsp", 3),
                instruction(0x2004, "je 0000000000002010h", 2),
                instruction(0x2006, "ret", 1),
                instruction(0x2010, "ret", 1),
            ],
            Some(cfg),
            BinaryAnalysis::default(),
        );

        app.selected_instruction = Some(3);
        app.instruction_list_state.select(Some(3));
        app.set_current_tab(Tab::GraphView);

        assert_eq!(app.current_tab, Tab::GraphView);
        assert_eq!(app.graph_view.selected_block, Some(Address(0x2000)));

        let cfg = app.cfg.as_ref().unwrap();
        app.graph_view.build_from_cfg(cfg);
        app.graph_view
            .move_selection(cfg, NavigationDirection::Down);
        assert_eq!(app.graph_view.selected_block, Some(Address(0x2006)));

        app.jump_to_selected_graph_block();

        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(5));
        assert_eq!(app.instruction_list_state.selected(), Some(5));
        assert_eq!(app.back_stack.len(), 1);

        app.go_back();

        assert_eq!(app.current_tab, Tab::GraphView);
        assert_eq!(app.graph_view.scope, GraphScope::WholeProgram);
        assert_eq!(app.graph_view.selected_block, Some(Address(0x2006)));
    }
}
