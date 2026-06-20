use std::fs::File;
use std::io::{self, Write};

use crate::fst::Fst;
use crate::symt::SymTable;

const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";

fn render_id(id: u32, table: Option<&SymTable>) -> io::Result<String> {
    if let Some(table) = table {
        if let Some(token) = table.get(id as i32) {
            return Ok(token.to_string());
        }
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"));
    }
    Ok(id.to_string())
}

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

pub fn fst_draw_sym(
    fst: &Fst,
    fout: &mut File,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> io::Result<()> {
    write!(fout, "{}", HEADER)?;
    for (s, state) in fst.states.iter().enumerate() {
        let sa = render_id(s as u32, sst)?;
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
            let sb = render_id(arc.state, sst)?;
            let li = render_id(arc.ilabel, ist)?;
            let lo = render_id(arc.olabel, ost)?;
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", sa, sb, li, lo, arc.weight)?;
        }
    }
    write!(fout, "{}", FOOTER)
}

fn trn(_st: &mut SymTable, id: usize, _token: &str) -> String {
    id.to_string()
}

fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    st.get(id as i32).unwrap_or("").to_string()
}
