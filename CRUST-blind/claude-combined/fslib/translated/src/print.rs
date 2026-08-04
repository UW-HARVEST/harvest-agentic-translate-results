use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                s, arc.state, arc.ilabel, arc.olabel, arc.weight
            )?;
        }
        if state.final_state {
            finals.push(s);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{:.6}", s, state.weight)?;
    }
    Ok(())
}
fn trans(st: Option<&SymTable>, id: u32) -> Option<String> {
    if let Some(t) = st {
        t.get(id as i32).map(|s| s.to_string())
    } else {
        Some(format!("{}", id))
    }
}
pub fn fst_print_sym(
    fst: &Fst,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    output: &mut dyn Write,
) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        let sa = trans(sst, s).unwrap_or_default();
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            let sb = trans(sst, arc.state).unwrap_or_default();
            let li = trans(ist, arc.ilabel).unwrap_or_default();
            let lo = trans(ost, arc.olabel).unwrap_or_default();
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                sa, sb, li, lo, arc.weight
            )?;
        }
        if state.final_state {
            finals.push(s);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = trans(sst, s).unwrap_or_default();
        writeln!(output, "{}\t{:.6}", sa, state.weight)?;
    }
    Ok(())
}
