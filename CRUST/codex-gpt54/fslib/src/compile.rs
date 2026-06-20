use crate::sr;
use crate::fst;
use std::io::BufRead;
use crate::symt::SymTable;
fn trn(token: &str, symt: &SymTable)-> usize{
    let _ = symt;
    token.parse::<usize>().unwrap_or(usize::MAX)
}
fn trt(token: &str, symt: &SymTable)-> usize{
    symt.getr(token).unwrap_or(-1) as usize
}
fn add_arc(
    fst: &mut fst::Fst,
    sa: usize,
    sb: usize,
    li: usize,
    lo: usize,
    w: f32,
) {
    while sa + 1 > fst.n_states as usize || sb + 1 > fst.n_states as usize {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}
fn add_final(fst: &mut fst::Fst, s: usize, w: f32) {
    while s + 1 > fst.n_states as usize {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}
fn parse_line(fst: &mut fst::Fst, buf: &mut str)->i32{
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    match parts.as_slice() {
        [sa, sb, li, lo, w] => {
            let (sa, sb, li, lo, w) = (
                sa.parse::<usize>(),
                sb.parse::<usize>(),
                li.parse::<usize>(),
                lo.parse::<usize>(),
                w.parse::<f32>(),
            );
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (sa, sb, li, lo, w) {
                add_arc(fst, sa, sb, li, lo, w);
                0
            } else {
                -1
            }
        }
        [sa, sb, li, lo] => {
            let (sa, sb, li, lo) = (
                sa.parse::<usize>(),
                sb.parse::<usize>(),
                li.parse::<usize>(),
                lo.parse::<usize>(),
            );
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (sa, sb, li, lo) {
                add_arc(fst, sa, sb, li, lo, sr.one());
                0
            } else {
                -1
            }
        }
        [sf, w] => {
            let (sf, w) = (sf.parse::<usize>(), w.parse::<f32>());
            if let (Ok(sf), Ok(w)) = (sf, w) {
                add_final(fst, sf, w);
                0
            } else {
                -1
            }
        }
        [sf] => {
            if let Ok(sf) = sf.parse::<usize>() {
                add_final(fst, sf, sr.one());
                0
            } else {
                -1
            }
        }
        _ => -1,
    }
}
fn parse_line_sym(fst: &mut fst::Fst, buf: &mut str, ist: &SymTable, ost: &SymTable, sst: &SymTable)->i32{
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    let itrans = if ist.n_items == 0 { trn } else { trt };
    let otrans = if ost.n_items == 0 { trn } else { trt };
    let strans = if sst.n_items == 0 { trn } else { trt };

    match parts.as_slice() {
        [sa, sb, li, lo, w] => {
            let (sa, sb, li, lo, w) = (
                strans(sa, sst),
                strans(sb, sst),
                itrans(li, ist),
                otrans(lo, ost),
                w.parse::<f32>().ok(),
            );
            if [sa, sb, li, lo].contains(&usize::MAX) || w.is_none() {
                -1
            } else {
                add_arc(fst, sa, sb, li, lo, w.unwrap());
                0
            }
        }
        [sa, sb, li, lo] => {
            let (sa, sb, li, lo) = (
                strans(sa, sst),
                strans(sb, sst),
                itrans(li, ist),
                otrans(lo, ost),
            );
            if [sa, sb, li, lo].contains(&usize::MAX) {
                -1
            } else {
                add_arc(fst, sa, sb, li, lo, sr.one());
                0
            }
        }
        [sf, w] => {
            let sf = strans(sf, sst);
            let w = w.parse::<f32>().ok();
            if sf == usize::MAX || w.is_none() {
                -1
            } else {
                add_final(fst, sf, w.unwrap());
                0
            }
        }
        [sf] => {
            let sf = strans(sf, sst);
            if sf == usize::MAX {
                -1
            } else {
                add_final(fst, sf, sr.one());
                0
            }
        }
        _ => -1,
    }
}
fn parse_line_sym_acc(fst: &mut fst::Fst, buf: &mut str, ist: &SymTable, ost: &SymTable, sst: &SymTable)->i32{
    let _ = ost;
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    let itrans = if ist.n_items == 0 { trn } else { trt };
    let strans = if sst.n_items == 0 { trn } else { trt };

    match parts.as_slice() {
        [sa, sb, li, w] => {
            let (sa, sb, li, w) = (
                strans(sa, sst),
                strans(sb, sst),
                itrans(li, ist),
                w.parse::<f32>().ok(),
            );
            if [sa, sb, li].contains(&usize::MAX) || w.is_none() {
                -1
            } else {
                add_arc(fst, sa, sb, li, li, w.unwrap());
                0
            }
        }
        [sa, sb, li] => {
            let (sa, sb, li) = (strans(sa, sst), strans(sb, sst), itrans(li, ist));
            if [sa, sb, li].contains(&usize::MAX) {
                -1
            } else {
                add_arc(fst, sa, sb, li, li, sr.one());
                0
            }
        }
        [sf, w] => {
            let sf = strans(sf, sst);
            let w = w.parse::<f32>().ok();
            if sf == usize::MAX || w.is_none() {
                -1
            } else {
                add_final(fst, sf, w.unwrap());
                0
            }
        }
        [sf] => {
            let sf = strans(sf, sst);
            if sf == usize::MAX {
                -1
            } else {
                add_final(fst, sf, sr.one());
                0
            }
        }
        _ => -1,
    }
}
pub(crate) fn fst_compile(fst: &mut fst::Fst, fin: &mut dyn BufRead, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool)-> fst::Fst{
    let mut line = String::new();
    while fin.read_line(&mut line).unwrap_or(0) != 0 {
        let mut current = line.trim_end_matches(['\n', '\r']).to_string();
        let res = if is_acc {
            parse_line_sym_acc(fst, current.as_mut_str(), ist, ost, sst)
        } else {
            parse_line_sym(fst, current.as_mut_str(), ist, ost, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line: {}", current);
            std::process::exit(1);
        }
        line.clear();
    }

    if sst.n_items != 0 {
        if let Some(start_state) = sst.getr(fst::START_STATE) {
            if start_state != -1 {
                fst.start = start_state as u32;
            }
        }
    }

    clone_fst(fst)
}
pub(crate) fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst{
    for line in s.lines() {
        let mut line = line.to_string();
        if parse_line(fst, line.as_mut_str()) != 0 {
            eprintln!("Invalid input line: {}", line);
            std::process::exit(1);
        }
    }
    clone_fst(fst)
}
fn clone_fst(src: &fst::Fst) -> fst::Fst {
    fst::Fst {
        start: src.start,
        n_states: src.n_states,
        n_max: src.n_max,
        sr_type: src.sr_type,
        flags: src.flags,
        states: src
            .states
            .iter()
            .map(|state| fst::StateData {
                n_arcs: state.n_arcs,
                n_max: state.n_max,
                weight: state.weight,
                final_state: state.final_state,
                arcs: state
                    .arcs
                    .iter()
                    .map(|arc| fst::ArcData {
                        state: arc.state,
                        weight: arc.weight,
                        ilabel: arc.ilabel,
                        olabel: arc.olabel,
                    })
                    .collect(),
            })
            .collect(),
    }
}
