use crate::fst::Fst;
use crate::symt::SymTable;
use std::fs::File;
use std::io::{self, Write};

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

fn id_str(st: Option<&SymTable>, id: usize) -> String {
    match st {
        Some(s) => match s.get(id as i32) {
            Some(t) => t.to_string(),
            None => format!("{}", id),
        },
        None => format!("{}", id),
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
        let sa = id_str(sst, s as usize);
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
            let sb = id_str(sst, arc.state as usize);
            let li = id_str(ist, arc.ilabel as usize);
            let lo = id_str(ost, arc.olabel as usize);
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
    match st.get(id as i32) {
        Some(t) => t.to_string(),
        None => String::new(),
    }
}

#[allow(dead_code)]
fn _unused(st: &mut SymTable) {
    // Ensure trn/trt are referenced
    let _ = trn(st, 0, "");
    let _ = trt(st, 0, "");
}
