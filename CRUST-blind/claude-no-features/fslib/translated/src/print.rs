use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
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
        writeln!(output, "{}\t{}", s, format_f32(state.weight))?;
    }
    Ok(())
}

fn format_f32(v: f32) -> String {
    // C %f default is 6 digits
    format!("{:.6}", v)
}

fn trn(_st: Option<&SymTable>, id: usize) -> Option<String> {
    Some(format!("{}", id))
}

fn trt(st: Option<&SymTable>, id: usize) -> Option<String> {
    if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
    } else {
        None
    }
}

fn translate(st: Option<&SymTable>, id: usize) -> Option<String> {
    if st.is_none() {
        trn(st, id)
    } else {
        trt(st, id)
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
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            let sa = translate(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let sb = translate(sst, arc.state as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let li = translate(ist, arc.ilabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let lo = translate(ost, arc.olabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = translate(sst, s as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
        writeln!(output, "{}\t{}", sa, format_f32(state.weight))?;
    }
    Ok(())
}
