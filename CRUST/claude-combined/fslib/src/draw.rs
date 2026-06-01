use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;
const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";
pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    write!(fout, "{}", HEADER)?;
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
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
    write!(fout, "{}", FOOTER)?;
    Ok(())
}
fn trn(_st: Option<&SymTable>, id: usize, _token: &str) -> String {
    id.to_string()
}
fn trt(st: Option<&SymTable>, id: usize, _token: &str) -> String {
    match st {
        Some(s) => match s.get(id as i32) {
            Some(t) => t.to_string(),
            None => id.to_string(),
        },
        None => id.to_string(),
    }
}
pub fn fst_draw_sym(
    fst: &Fst,
    fout: &mut File,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> io::Result<()> {
    let strans = |id: u32| -> String {
        if sst.is_none() {
            trn(None, id as usize, "")
        } else {
            trt(sst, id as usize, "")
        }
    };
    let itrans = |id: u32| -> String {
        if ist.is_none() {
            trn(None, id as usize, "")
        } else {
            trt(ist, id as usize, "")
        }
    };
    let otrans = |id: u32| -> String {
        if ost.is_none() {
            trn(None, id as usize, "")
        } else {
            trt(ost, id as usize, "")
        }
    };
    write!(fout, "{}", HEADER)?;
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        let sa = strans(s as u32);
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
    write!(fout, "{}", FOOTER)?;
    Ok(())
}
