use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for arc in &state.arcs {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                s, arc.state, arc.ilabel, arc.olabel, arc.weight
            )?;
        }
        if state.final_state {
            finals.push(s as u32);
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
    let mut finals: Vec<u32> = Vec::new();
    let trans = |st: Option<&SymTable>, id: u32| -> String {
        match st {
            Some(s) => match s.get(id as i32) {
                Some(t) => t.to_string(),
                None => id.to_string(),
            },
            None => id.to_string(),
        }
    };
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for arc in &state.arcs {
            let sa = trans(sst, s as u32);
            let sb = trans(sst, arc.state);
            let li = trans(ist, arc.ilabel);
            let lo = trans(ost, arc.olabel);
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = trans(sst, s);
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
