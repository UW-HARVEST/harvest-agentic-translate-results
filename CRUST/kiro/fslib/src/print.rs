use crate::fst::Fst;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for arc in &state.arcs {
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", s, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
        }
        if state.final_state {
            finals.push(s);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", s, state.weight)?;
    }
    Ok(())
}
pub fn fst_print_sym(
    fst: &Fst,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    output: &mut dyn Write,
) -> io::Result<()> {
    let strans = |id: u32| -> String {
        match sst {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    let itrans = |id: u32| -> String {
        match ist {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    let otrans = |id: u32| -> String {
        match ost {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for arc in &state.arcs {
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}",
                strans(s), strans(arc.state), itrans(arc.ilabel), otrans(arc.olabel), arc.weight)?;
        }
        if state.final_state {
            finals.push(s);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", strans(s), state.weight)?;
    }
    Ok(())
}
