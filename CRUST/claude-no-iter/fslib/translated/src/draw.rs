use std::fs::File;
use std::io::{self, Write};
use crate::fst::Fst;
use crate::symt::SymTable;

const HEADER: &str = "digraph T {\n\trankdir = LR;\n\torientation = Landscape;\n";
const FOOTER: &str = "}\n";

pub fn fst_draw(fst: &Fst, fout: &mut File) -> io::Result<()> {
    fout.write_all(HEADER.as_bytes())?;
    for (s, state) in fst.states.iter().enumerate() {
        if !state.final_state {
            let style = if (s as u32) == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", s, s, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", s, s)?;
        }
        for arc in state.arcs.iter() {
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
    let trn_s = |id: u32| -> String {
        match sst {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };
    let trn_i = |id: u32| -> String {
        match ist {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };
    let trn_o = |id: u32| -> String {
        match ost {
            Some(t) => t.get(id as i32).map(String::from).unwrap_or_else(|| id.to_string()),
            None => id.to_string(),
        }
    };

    fout.write_all(HEADER.as_bytes())?;
    for (s, state) in fst.states.iter().enumerate() {
        let sa = trn_s(s as u32);
        if !state.final_state {
            let style = if (s as u32) == fst.start { "filled" } else { "solid" };
            writeln!(fout, "\t{} [label = \"{}\", shape = circle, style = {} ];", sa, sa, style)?;
        } else {
            writeln!(fout, "\t{} [label = \"{}\", shape = doublecircle, style = filled ];", sa, sa)?;
        }
        for arc in state.arcs.iter() {
            let sb = trn_s(arc.state);
            let li = trn_i(arc.ilabel);
            let lo = trn_o(arc.olabel);
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
    id.to_string()
}

fn trt(st: &mut SymTable, id: usize, _token: &str) -> String {
    st.get(id as i32).map(String::from).unwrap_or_default()
}
