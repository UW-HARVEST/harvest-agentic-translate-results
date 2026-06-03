use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Vec<u32> = Vec::new();
    for (s, state) in fst.states.iter().enumerate() {
        for arc in state.arcs.iter() {
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
    let trn_s = |id: u32| -> String {
        match sst {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };
    let trn_i = |id: u32| -> String {
        match ist {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };
    let trn_o = |id: u32| -> String {
        match ost {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };

    let mut finals: Vec<u32> = Vec::new();
    for (s, state) in fst.states.iter().enumerate() {
        for arc in state.arcs.iter() {
            let sa = trn_s(s as u32);
            let sb = trn_s(arc.state);
            let li = trn_i(arc.ilabel);
            let lo = trn_o(arc.olabel);
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.push(s as u32);
        }
    }
    for s in finals {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", trn_s(s), state.weight)?;
    }
    Ok(())
}
