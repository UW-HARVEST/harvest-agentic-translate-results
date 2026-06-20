use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";
pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    write!(fout, "{}", HEADER)?;
    for (s, state) in fst.states.iter().enumerate() {
        if !state.final_state {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = circle, style = {} ];",
                s,
                s,
                if s as u32 == fst.start { "filled" } else { "solid" }
            )?;
        } else {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = doublecircle, style = filled ];",
                s, s
            )?;
        }

        for arc in &state.arcs {
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                s, arc.state, arc.ilabel, arc.olabel, arc.weight
            )?;
        }
    }
    write!(fout, "{}", FOOTER)
}
pub fn fst_draw_sym(fst: &Fst, fout: &mut File, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> io::Result<()> {
    write!(fout, "{}", HEADER)?;
    for (s, state) in fst.states.iter().enumerate() {
        let sa = translate(sst, s as i32)?;
        if !state.final_state {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = circle, style = {} ];",
                sa,
                sa,
                if s as u32 == fst.start { "filled" } else { "solid" }
            )?;
        } else {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = doublecircle, style = filled ];",
                sa, sa
            )?;
        }

        for arc in &state.arcs {
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                sa,
                translate(sst, arc.state as i32)?,
                translate(ist, arc.ilabel as i32)?,
                translate(ost, arc.olabel as i32)?,
                arc.weight
            )?;
        }
    }
    write!(fout, "{}", FOOTER)
}
fn trn(st: &mut SymTable, id: usize, token: &str)-> String {
    let _ = st;
    let _ = token;
    id.to_string()
}
fn trt(st: &mut SymTable, id: usize, token: &str)-> String {
    let _ = token;
    st.get(id as i32).unwrap_or("").to_string()
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
