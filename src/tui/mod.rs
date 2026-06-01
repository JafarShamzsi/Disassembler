pub mod app;
mod input;
mod session;
mod views;

pub use app::{App, NameItem, Tab, XrefItem};
pub use session::run_tui;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::Instruction;
    use crate::graph::ControlFlowGraph;
    use crate::graph::{Address, Instruction as CfgInstruction};
    use crate::parser::BinaryAnalysis;
    use crate::parser::{StringSummary, SymbolKind, SymbolSummary};
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

        assert_eq!(app.search_matches.len(), 1);
        app.next_search_match();

        assert_eq!(app.current_tab, Tab::Names);
        assert_eq!(app.selected_name, Some(0));
        assert_eq!(app.name_list_state.selected(), Some(0));
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
    fn rename_prompt_updates_project_and_search() {
        let mut app = App::new(
            vec![instruction(0x1000, "push rbp", 1)],
            None,
            BinaryAnalysis::default(),
        );

        app.begin_rename();
        app.prompt_push_char('e');
        app.prompt_push_char('n');
        app.prompt_push_char('t');
        app.prompt_push_char('r');
        app.prompt_push_char('y');
        app.commit_prompt();

        assert_eq!(app.project.name_for(0x1000), Some("entry"));
        assert_eq!(app.name_for(0x1000), Some("entry"));
        assert!(app.dirty);

        app.update_search("entry".to_string());
        assert_eq!(app.search_matches, vec![app::SearchMatch::Instruction(0)]);
    }

    #[test]
    fn comment_prompt_updates_project() {
        let mut app = App::new(
            vec![instruction(0x1000, "call 0000000000002000h", 5)],
            None,
            BinaryAnalysis::default(),
        );

        app.begin_comment();
        for c in "interesting call".chars() {
            app.prompt_push_char(c);
        }
        app.commit_prompt();

        assert_eq!(app.project.comment_for(0x1000), Some("interesting call"));
        assert_eq!(app.comment_for(0x1000), Some("interesting call"));
        assert!(app.dirty);
    }

    #[test]
    fn bookmark_toggle_updates_project() {
        let mut app = App::new(
            vec![instruction(0x1000, "nop", 1)],
            None,
            BinaryAnalysis::default(),
        );

        app.toggle_bookmark_at_target();
        assert!(app.project.is_bookmarked(0x1000));
        assert!(app.is_bookmarked(0x1000));

        app.toggle_bookmark_at_target();
        assert!(!app.project.is_bookmarked(0x1000));
        assert!(!app.is_bookmarked(0x1000));
    }

    #[test]
    fn bookmark_overlay_jumps_to_selected_bookmark() {
        let mut app = App::new(
            vec![
                instruction(0x1000, "nop", 1),
                instruction(0x2000, "push rbp", 1),
                instruction(0x3000, "ret", 1),
            ],
            None,
            BinaryAnalysis::default(),
        );
        app.project.toggle_bookmark(0x1000, None);
        app.project.toggle_bookmark(0x3000, None);

        app.open_bookmarks_overlay();
        app.next_bookmark();
        app.jump_to_selected_bookmark();

        assert!(!app.bookmarks_overlay);
        assert_eq!(app.current_tab, Tab::Instructions);
        assert_eq!(app.selected_instruction, Some(2));
        assert_eq!(app.instruction_list_state.selected(), Some(2));
        assert_eq!(app.back_stack.len(), 1);
    }

    #[test]
    fn app_project_save_round_trips_annotations() {
        let path = std::env::temp_dir().join(format!(
            "disassembler-tui-project-test-{}.json",
            std::process::id()
        ));
        let project = AnalysisProject::from_binary("sample.exe", b"binary");
        let mut app = App::with_project(
            vec![instruction(0x1000, "ret", 1)],
            None,
            BinaryAnalysis::default(),
            project,
            Some(path.clone()),
        );

        app.begin_rename();
        for c in "entry".chars() {
            app.prompt_push_char(c);
        }
        app.commit_prompt();
        app.save_project().unwrap();

        let loaded = AnalysisProject::load(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.name_for(0x1000), Some("entry"));
        assert!(!app.dirty);
    }
}
