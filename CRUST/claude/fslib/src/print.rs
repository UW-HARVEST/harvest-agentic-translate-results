use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for arc in &state.arcs {
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}",
                s as u32, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
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
fn trn_token(_st: Option<&SymTable>, id: usize) -> Option<String> {
    Some(format!("{}", id))
}
fn trt_token(st: Option<&SymTable>, id: usize) -> Option<String> {
    if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
    } else {
        None
    }
}
pub fn fst_print_sym(
    fst: &Fst,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    output: &mut dyn Write,
) -> io::Result<()> {
    let strans = if sst.is_none() { trn_token } else { trt_token };
    let itrans = if ist.is_none() { trn_token } else { trt_token };
    let otrans = if ost.is_none() { trn_token } else { trt_token };
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for arc in &state.arcs {
            let sa = strans(sst, s).unwrap_or_default();
            let sb = strans(sst, arc.state as usize).unwrap_or_default();
            let li = itrans(ist, arc.ilabel as usize).unwrap_or_default();
            let lo = otrans(ost, arc.olabel as usize).unwrap_or_default();
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = strans(sst, s as usize).unwrap_or_default();
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
