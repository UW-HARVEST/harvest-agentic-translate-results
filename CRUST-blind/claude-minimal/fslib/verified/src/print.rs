use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for arc in &state.arcs {
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
        writeln!(output, "{}\t{}", s, state.weight)?;
    }
    Ok(())
}

fn trn_id(id: u32) -> String {
    format!("{}", id)
}

fn trt_id(st: &SymTable, id: u32) -> Option<String> {
    st.get(id as i32).map(|s| s.to_string())
}

fn translate(st: Option<&SymTable>, id: u32) -> Option<String> {
    match st {
        None => Some(trn_id(id)),
        Some(t) => trt_id(t, id),
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
        for arc in &state.arcs {
            let sa = translate(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let sb = translate(sst, arc.state).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let li = translate(ist, arc.ilabel).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let lo = translate(ost, arc.olabel).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        let sa = translate(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
