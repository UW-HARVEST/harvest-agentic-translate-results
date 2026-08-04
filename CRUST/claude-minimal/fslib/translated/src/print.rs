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
    let strans = |id: u32| -> String {
        match sst {
            Some(t) => t.get(id as i32).unwrap_or("").to_string(),
            None => format!("{}", id),
        }
    };
    let itrans = |id: u32| -> String {
        match ist {
            Some(t) => t.get(id as i32).unwrap_or("").to_string(),
            None => format!("{}", id),
        }
    };
    let otrans = |id: u32| -> String {
        match ost {
            Some(t) => t.get(id as i32).unwrap_or("").to_string(),
            None => format!("{}", id),
        }
    };
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            let sa = strans(s);
            let sb = strans(arc.state);
            let li = itrans(arc.ilabel);
            let lo = otrans(arc.olabel);
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.enqueue(s);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        let sa = strans(s);
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }
    Ok(())
}
