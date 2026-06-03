use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";
pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", s, s, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", s, s)?;
        }
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", s, arc.state, arc.ilabel, arc.olabel, format_g(arc.weight))?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}
pub fn fst_draw_sym(fst: &Fst, fout: &mut File, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        let sa = lookup_or_num(sst, s);
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", sa, sa, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", sa, sa)?;
        }
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            let sb = lookup_or_num(sst, arc.state);
            let li = lookup_or_num(ist, arc.ilabel);
            let lo = lookup_or_num(ost, arc.olabel);
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", sa, sb, li, lo, format_g(arc.weight))?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}
fn trn(_st: &mut SymTable, id: usize, _token: &str) -> String {
    id.to_string()
}
fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    match st.get(id as i32) {
        Some(t) => t.to_string(),
        None => String::new(),
    }
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
fn format_g(v: f32) -> String {
    // %g formatter approximation
    if v == 0.0 {
        return "0".to_string();
    }
    let s = format!("{}", v);
    s
}
