use crate::fst::Fst;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::io::{self, Write};

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Queue<u32> = Queue::new();
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
            finals.enqueue(s);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", s, state.weight)?;
    }
    Ok(())
}

fn trans_id(st: Option<&SymTable>, id: usize) -> String {
    match st {
        Some(s) => match s.get(id as i32) {
            Some(t) => t.to_string(),
            None => format!("{}", id),
        },
        None => format!("{}", id),
    }
}

pub fn fst_print_sym(
    fst: &Fst,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    output: &mut dyn Write,
) -> io::Result<()> {
    let mut finals: Queue<u32> = Queue::new();
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            let sa = trans_id(sst, s as usize);
            let sb = trans_id(sst, arc.state as usize);
            let li = trans_id(ist, arc.ilabel as usize);
            let lo = trans_id(ost, arc.olabel as usize);
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.enqueue(s);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        let sa = trans_id(sst, s as usize);
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
