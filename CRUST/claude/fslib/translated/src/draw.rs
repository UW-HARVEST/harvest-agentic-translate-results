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
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", s as u32, s as u32, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", s as u32, s as u32)?;
        }
        for arc in &state.arcs {
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];",
                s as u32, arc.state, arc.ilabel, arc.olabel, arc.weight)?;
        }
    }
    fout.write_all(FOOTER.as_bytes())?;
    Ok(())
}
fn trn_token(_st: Option<&SymTable>, id: usize) -> Option<String> {
    Some(format!("{}", id))
}
fn trt_token(st: Option<&SymTable>, id: usize) -> Option<String> {
    if let Some(st) = st {
        st.get(id as i32).map(|s| s.to_string())
    } else {
        None
    }
}
pub fn fst_draw_sym(fst: &Fst, fout: &mut File, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> io::Result<()> {
    let strans = if sst.is_none() { trn_token } else { trt_token };
    let itrans = if ist.is_none() { trn_token } else { trt_token };
    let otrans = if ost.is_none() { trn_token } else { trt_token };
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        let sa = strans(sst, s).unwrap_or_default();
        if !state.final_state {
            let style = if s as u32 == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", sa, sa, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", sa, sa)?;
        }
        for arc in &state.arcs {
            let sb = strans(sst, arc.state as usize).unwrap_or_default();
            let li = itrans(ist, arc.ilabel as usize).unwrap_or_default();
            let lo = otrans(ost, arc.olabel as usize).unwrap_or_default();
            writeln!(fout, "\t\t{} -> {} [ label = \"{}:{}/{}\" ];", sa, sb, li, lo, arc.weight)?;
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
