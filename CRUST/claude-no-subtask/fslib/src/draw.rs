use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;

const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";

pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for s in 0..fst.n_states as usize {
        if s >= fst.states.len() {
            break;
        }
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
    let strans = |id: u32| -> String {
        if let Some(s) = sst {
            if let Some(t) = s.get(id as i32) {
                return t.to_string();
            }
        }
        format!("{}", id)
    };
    let itrans = |id: u32| -> String {
        if let Some(s) = ist {
            if let Some(t) = s.get(id as i32) {
                return t.to_string();
            }
        }
        format!("{}", id)
    };
    let otrans = |id: u32| -> String {
        if let Some(s) = ost {
            if let Some(t) = s.get(id as i32) {
                return t.to_string();
            }
        }
        format!("{}", id)
    };
    for s in 0..fst.n_states as usize {
        if s >= fst.states.len() {
            break;
        }
        let state = &fst.states[s];
        let sa = strans(s as u32);
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
        for arc in &state.arcs {
            let sb = strans(arc.state);
            let li = itrans(arc.ilabel);
            let lo = otrans(arc.olabel);
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

#[allow(dead_code)]
fn trn(_st: &mut SymTable, id: usize, _token: &str) -> String {
    format!("{}", id)
}
#[allow(dead_code)]
fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    if let Some(t) = st.get(id as i32) {
        t.to_string()
    } else {
        String::new()
    }
}
