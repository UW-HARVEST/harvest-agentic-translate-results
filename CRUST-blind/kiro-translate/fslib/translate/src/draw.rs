use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";
pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    write!(fout, "{}", HEADER)?;
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", s, s, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", s, s)?;
        }
        for arc in &state.arcs {
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", s, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
        }
    }
    write!(fout, "{}", FOOTER)?;
    Ok(())
}
pub fn fst_draw_sym(fst: &Fst, fout: &mut File, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> io::Result<()> {
    let strans = |id: u32| -> String {
        match sst {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    let itrans = |id: u32| -> String {
        match ist {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    let otrans = |id: u32| -> String {
        match ost {
            Some(st) => st.get(id as i32).map(|s| s.to_string()).unwrap_or_else(|| format!("{}", id)),
            None => format!("{}", id),
        }
    };
    write!(fout, "{}", HEADER)?;
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        let sa = strans(s);
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", sa, sa, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", sa, sa)?;
        }
        for arc in &state.arcs {
            let sb = strans(arc.state);
            let li = itrans(arc.ilabel);
            let lo = otrans(arc.olabel);
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", sa, sb, li, lo, arc.weight)?;
        }
    }
    write!(fout, "{}", FOOTER)?;
    Ok(())
}
fn trn(_st: &mut SymTable, id: usize, _token: &str) -> String {
    format!("{}", id)
}
fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    st.get(id as i32).map(|s| s.to_string()).unwrap_or_default()
}
