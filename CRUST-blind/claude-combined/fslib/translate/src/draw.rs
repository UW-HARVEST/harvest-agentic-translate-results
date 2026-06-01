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
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
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
fn trans_draw(st: Option<&SymTable>, id: u32) -> Option<String> {
    if let Some(t) = st {
        t.get(id as i32).map(|s| s.to_string())
    } else {
        Some(format!("{}", id))
    }
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
        let sa = trans_draw(sst, s).unwrap_or_default();
        if !state.final_state {
            let style = if s == fst.start { "filled" } else { "solid" };
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
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            let sb = trans_draw(sst, arc.state).unwrap_or_default();
            let li = trans_draw(ist, arc.ilabel).unwrap_or_default();
            let lo = trans_draw(ost, arc.olabel).unwrap_or_default();
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
fn _used() {
    let _ = trn;
    let _ = trt;
}
