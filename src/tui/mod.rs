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
}
