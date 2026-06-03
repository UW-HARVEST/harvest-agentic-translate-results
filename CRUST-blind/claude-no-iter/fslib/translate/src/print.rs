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
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", s, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
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
            let sa = lookup_or_num(sst, s);
            let sb = lookup_or_num(sst, arc.state);
            let li = lookup_or_num(ist, arc.ilabel);
            let lo = lookup_or_num(ost, arc.olabel);
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.enqueue(s);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        let sa = lookup_or_num(sst, s);
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
fn lookup_or_num(st: Option<&SymTable>, id: u32) -> String {
    match st {
        Some(t) => match t.get(id as i32) {
            Some(s) => s.to_string(),
            None => id.to_string(),
        },
        None => id.to_string(),
    }
}
