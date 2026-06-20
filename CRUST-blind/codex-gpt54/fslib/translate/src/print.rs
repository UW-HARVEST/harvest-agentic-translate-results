use crate::fst::Fst;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::io::{self, Write};

fn render_id(id: u32, table: Option<&SymTable>) -> io::Result<String> {
    if let Some(table) = table {
        if let Some(token) = table.get(id as i32) {
            return Ok(token.to_string());
        }
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"));
    }
    Ok(id.to_string())
}

pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals = Queue::new();
    for (s, state) in fst.states.iter().enumerate() {
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

pub fn fst_print_sym(
    fst: &Fst,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    output: &mut dyn Write,
) -> io::Result<()> {
    let mut finals = Queue::new();
    for (s, state) in fst.states.iter().enumerate() {
        for arc in &state.arcs {
            let sa = render_id(s as u32, sst)?;
            let sb = render_id(arc.state, sst)?;
            let li = render_id(arc.ilabel, ist)?;
            let lo = render_id(arc.olabel, ost)?;
            writeln!(output, "{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight)?;
        }
        if state.final_state {
            finals.enqueue(s as u32);
        }
    }

    while let Some(s) = finals.dequeue() {
        let state = &fst.states[s as usize];
        let sa = render_id(s, sst)?;
        writeln!(output, "{}\t{}", sa, state.weight)?;
    }

    Ok(())
}
