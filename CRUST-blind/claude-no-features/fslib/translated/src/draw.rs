use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";
pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        if !state.final_state {
            let style = if s as u32 == fst.start { "filled" } else { "solid" };
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = circle, style = {} ];",
                s, s, style
            )?;
        } else {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = doublecircle, style = filled ];",
                s, s
            )?;
        }
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                s, arc.state, arc.ilabel, arc.olabel, format_g(arc.weight)
            )?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}

fn format_g(v: f32) -> String {
    // Print float using `%g` semantics: shorter of fixed and scientific
    if v == 0.0 {
        return "0".to_string();
    }
    let s = format!("{}", v);
    s
}

fn _trn_id(_st: Option<&SymTable>, id: usize) -> Option<String> {
    Some(format!("{}", id))
}

fn _trt_id(st: Option<&SymTable>, id: usize) -> Option<String> {
    if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
    } else {
        None
    }
}

fn translate(st: Option<&SymTable>, id: usize) -> Option<String> {
    if st.is_none() {
        Some(format!("{}", id))
    } else if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
    } else {
        None
    }
}

pub fn fst_draw_sym(fst: &Fst, fout: &mut File, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        let sa = translate(sst, s).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
        if !state.final_state {
            let style = if s as u32 == fst.start { "filled" } else { "solid" };
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = circle, style = {} ];",
                sa, sa, style
            )?;
        } else {
            writeln!(
                fout,
                "\t{} [label = \"{}\", shape = doublecircle, style = filled ];",
                sa, sa
            )?;
        }
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            let sb = translate(sst, arc.state as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let li = translate(ist, arc.ilabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            let lo = translate(ost, arc.olabel as usize).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid symbol"))?;
            writeln!(
                fout,
                "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                sa, sb, li, lo, format_g(arc.weight)
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
    match st.get(id as i32) {
        Some(s) => s.to_string(),
        None => String::new(),
    }
}
