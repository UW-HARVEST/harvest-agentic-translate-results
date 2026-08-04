use crate::fst;
use crate::sr;
use crate::symt::SymTable;
use crate::fst::START_STATE;
use std::io::BufRead;

fn trn(token: &str, _symt: &SymTable) -> Option<usize> {
    token.parse::<usize>().ok()
}
fn trt(token: &str, symt: &SymTable) -> Option<usize> {
    let r = symt.getr(token);
    match r {
        Some(v) if v >= 0 => Some(v as usize),
        _ => None,
    }
}
fn add_arc(
    fst: &mut fst::Fst,
    sa: usize,
    sb: usize,
    li: usize,
    lo: usize,
    w: f32,
) {
    while (sa + 1 > fst.n_states as usize) || (sb + 1 > fst.n_states as usize) {
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
fn parse_line(fst: &mut fst::Fst, buf: &str) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let trimmed = buf.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.len() {
        5 => {
            let sa = parts[0].parse::<usize>();
            let sb = parts[1].parse::<usize>();
            let li = parts[2].parse::<usize>();
            let lo = parts[3].parse::<usize>();
            let w = parts[4].parse::<f32>();
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (sa, sb, li, lo, w) {
                add_arc(fst, sa, sb, li, lo, w);
                return 0;
            }
            -1
        }
        4 => {
            let sa = parts[0].parse::<usize>();
            let sb = parts[1].parse::<usize>();
            let li = parts[2].parse::<usize>();
            let lo = parts[3].parse::<usize>();
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (sa, sb, li, lo) {
                add_arc(fst, sa, sb, li, lo, sr.one);
                return 0;
            }
            -1
        }
        2 => {
            let sf = parts[0].parse::<usize>();
            let w = parts[1].parse::<f32>();
            if let (Ok(sf), Ok(w)) = (sf, w) {
                add_final(fst, sf, w);
                return 0;
            }
            -1
        }
        1 => {
            let sf = parts[0].parse::<usize>();
            if let Ok(sf) = sf {
                add_final(fst, sf, sr.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}
fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let trimmed = buf.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let strans = |t: &str| -> Option<usize> { match sst { Some(s) => trt(t, s), None => trn(t, &SymTable::new()) } };
    let itrans = |t: &str| -> Option<usize> { match ist { Some(s) => trt(t, s), None => trn(t, &SymTable::new()) } };
    let otrans = |t: &str| -> Option<usize> { match ost { Some(s) => trt(t, s), None => trn(t, &SymTable::new()) } };
    match parts.len() {
        5 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            let w = parts[4].parse::<f32>().ok();
            if let (Some(sa), Some(sb), Some(li), Some(lo), Some(w)) = (sa, sb, li, lo, w) {
                add_arc(fst, sa, sb, li, lo, w);
                return 0;
            }
            -1
        }
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            if let (Some(sa), Some(sb), Some(li), Some(lo)) = (sa, sb, li, lo) {
                add_arc(fst, sa, sb, li, lo, sr.one);
                return 0;
            }
            -1
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>().ok();
            if let (Some(sf), Some(w)) = (sf, w) {
                add_final(fst, sf, w);
                return 0;
            }
            -1
        }
        1 => {
            let sf = strans(parts[0]);
            if let Some(sf) = sf {
                add_final(fst, sf, sr.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}
fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let trimmed = buf.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let strans = |t: &str| -> Option<usize> { match sst { Some(s) => trt(t, s), None => trn(t, &SymTable::new()) } };
    let itrans = |t: &str| -> Option<usize> { match ist { Some(s) => trt(t, s), None => trn(t, &SymTable::new()) } };
    match parts.len() {
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let w = parts[3].parse::<f32>().ok();
            if let (Some(sa), Some(sb), Some(li), Some(w)) = (sa, sb, li, w) {
                add_arc(fst, sa, sb, li, li, w);
                return 0;
            }
            -1
        }
        3 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            if let (Some(sa), Some(sb), Some(li)) = (sa, sb, li) {
                add_arc(fst, sa, sb, li, li, sr.one);
                return 0;
            }
            -1
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>().ok();
            if let (Some(sf), Some(w)) = (sf, w) {
                add_final(fst, sf, w);
                return 0;
            }
            -1
        }
        1 => {
            let sf = strans(parts[0]);
            if let Some(sf) = sf {
                add_final(fst, sf, sr.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}
pub fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) {
    let mut line = 1usize;
    let mut buf = String::new();
    while {
        buf.clear();
        match fin.read_line(&mut buf) {
            Ok(0) => false,
            Ok(_) => true,
            Err(_) => false,
        }
    } {
        line += 1;
        let res = if !is_acc {
            parse_line_sym(fst, &buf, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, &buf, ist, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line, buf);
            std::process::exit(1);
        }
    }
    if let Some(sst) = sst {
        if let Some(start_state) = sst.getr(START_STATE) {
            if start_state >= 0 {
                fst.start = start_state as u32;
            }
        }
    }
}
pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) {
    let mut line = 1usize;
    for tok in s.split('\n') {
        if tok.trim().is_empty() {
            line += 1;
            continue;
        }
        if parse_line(fst, tok) != 0 {
            eprintln!("Invalid input line {}: {}", line, tok);
            std::process::exit(1);
        }
        line += 1;
    }
}
