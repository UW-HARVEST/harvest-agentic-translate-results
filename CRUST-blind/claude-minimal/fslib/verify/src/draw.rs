use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";

fn translate(st: Option<&SymTable>, id: u32) -> Option<String> {
    match st {
        None => Some(format!("{}", id)),
        Some(t) => t.get(id as i32).map(|s| s.to_string()),
    }
}

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
        for arc in &state.arcs {
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                s, arc.state, arc.ilabel, arc.olabel, arc.weight
            )?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}

pub fn fst_draw_sym(
    fst: &Fst,
    fout: &mut File,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        let sa = translate(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", sa, sa, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", sa, sa)?;
        }
        for arc in &state.arcs {
            let sb = translate(sst, arc.state).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let li = translate(ist, arc.ilabel).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let lo = translate(ost, arc.olabel).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                sa, sb, li, lo, arc.weight
            )?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}

fn trn(_st: &mut SymTable, id: usize, _token: &str) -> String {
    format!("{}", id)
}
fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    st.get(id as i32).map(|s| s.to_string()).unwrap_or_default()
}

#[allow(dead_code)]
fn _force_use(st: &mut SymTable) {
    let _ = trn(st, 0, "");
    let _ = trt(st, 0, "");
}
