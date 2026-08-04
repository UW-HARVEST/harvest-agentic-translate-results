use crate::fst::Fst;
use crate::symt::SymTable;
use std::io::{self, Write};
pub fn fst_print(fst: &Fst, output: &mut dyn Write) -> io::Result<()> {
    let mut finals = Vec::new();
    for (s, state) in fst.states.iter().enumerate() {
        for arc in &state.arcs {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                s,
                arc.state,
                arc.ilabel,
                arc.olabel,
                arc.weight
            )?;
        }
        if state.final_state {
            finals.push(s);
        }
    }

    for s in finals {
        writeln!(output, "{}\t{}", s, fst.states[s].weight)?;
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
    let mut finals = Vec::new();
    for (s, state) in fst.states.iter().enumerate() {
        for arc in &state.arcs {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.5}",
                translate(sst, s as i32)?,
                translate(sst, arc.state as i32)?,
                translate(ist, arc.ilabel as i32)?,
                translate(ost, arc.olabel as i32)?,
                arc.weight
            )?;
        }
        if state.final_state {
            finals.push(s);
        }
    }

    for s in finals {
        writeln!(
            output,
            "{}\t{}",
            translate(sst, s as i32)?,
            fst.states[s].weight
        )?;
    }
    Ok(())
}
fn translate(table: Option<&SymTable>, id: i32) -> io::Result<String> {
    if let Some(table) = table {
        table
            .get(id)
            .map(str::to_string)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))
    } else {
        Ok(id.to_string())
    }
}
