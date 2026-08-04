use crate::fst::Fst;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::io::{self, Write};

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals: Queue<u32> = Queue::new();
    for s in 0..fst.n_states as usize {
        if s >= fst.states.len() {
            break;
        }
        let state = &fst.states[s];
        for arc in &state.arcs {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                s, arc.state, arc.ilabel, arc.olabel, arc.weight
            )?;
        }
        if state.final_state {
            finals.enqueue(s as u32);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        writeln!(output, "{}\t{}", s, state.weight)?;
    }
    Ok(())
}
fn trans_id(_st: Option<&SymTable>, id: u32) -> Option<String> {
    Some(format!("{}", id))
}
fn trans_token(st: Option<&SymTable>, id: u32) -> Option<String> {
    if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
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
    let strans = if sst.is_some() { trans_token } else { trans_id };
    let itrans = if ist.is_some() { trans_token } else { trans_id };
    let otrans = if ost.is_some() { trans_token } else { trans_id };
    let mut finals: Queue<u32> = Queue::new();
    for s in 0..fst.n_states as usize {
        if s >= fst.states.len() {
            break;
        }
        let state = &fst.states[s];
        for arc in &state.arcs {
            let sa = strans(sst, s as u32);
            let sb = strans(sst, arc.state);
            let li = itrans(ist, arc.ilabel);
            let lo = otrans(ost, arc.olabel);
            match (sa, sb, li, lo) {
                (Some(sa), Some(sb), Some(li), Some(lo)) => {
                    writeln!(
                        output,
                        "{}\t{}\t{}\t{}\t{:.5}",
                        sa, sb, li, lo, arc.weight
                    )?;
                }
                _ => {
                    eprintln!("Invalid symbol");
                }
            }
        }
        if state.final_state {
            finals.enqueue(s as u32);
        }
    }
    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        let sa = strans(sst, s);
        if let Some(sa) = sa {
            writeln!(output, "{}\t{}", sa, state.weight)?;
        }
    }
    Ok(())
}
