use crate::fst::Fst;
#[allow(unused_imports)]
use crate::queue::Queue;
use crate::symt::SymTable;
use std::io::{self, Write};

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", s, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", s, format_float(state.weight))?;
    }
    Ok(())
}

fn format_float(f: f32) -> String {
    // Match C %f default of 6 decimals
    format!("{:.6}", f)
}

fn trn_id(id: usize) -> String {
    format!("{}", id)
}

fn trt_sym(st: Option<&SymTable>, id: usize) -> Option<String> {
    if let Some(table) = st {
        match table.get(id as i32) {
            Some(s) => Some(s.to_string()),
            None => None,
        }
    } else {
        Some(trn_id(id))
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
            let sa = trt_sym(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid symbol"))?;
            let sb = trt_sym(sst, arc.state as usize).ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid symbol"))?;
            let li = trt_sym(ist, arc.ilabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid symbol"))?;
            let lo = trt_sym(ost, arc.olabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid symbol"))?;
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = trt_sym(sst, s as usize).ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid symbol"))?;
        writeln!(output, "{}\t{}", sa, format_float(state.weight))?;
    }
    Ok(())
}
