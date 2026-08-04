use crate::sr;
use crate::fst;
use crate::symt;
use std::io::{self, BufRead};
use crate::symt::SymTable;

fn trn(token: &str, _symt: &SymTable) -> usize {
    token.parse::<usize>().unwrap_or(usize::MAX)
}
fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token) {
        Some(id) if id >= 0 => id as usize,
        _ => usize::MAX,
    }
}
fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
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
    // Try tab-separated first, then whitespace
    let parts: Vec<&str> = if buf.contains('\t') {
        buf.trim().split('\t').collect()
    } else {
        buf.trim().split_whitespace().collect()
    };
    match parts.len() {
        5 => {
            let sa: usize = parts[0].parse().unwrap();
            let sb: usize = parts[1].parse().unwrap();
            let li: usize = parts[2].parse().unwrap();
            let lo: usize = parts[3].parse().unwrap();
            let w: f32 = parts[4].parse().unwrap();
            add_arc(fst, sa, sb, li, lo, w);
            0
        }
        4 => {
            let sa: usize = parts[0].parse().unwrap();
            let sb: usize = parts[1].parse().unwrap();
            let li: usize = parts[2].parse().unwrap();
            let lo: usize = parts[3].parse().unwrap();
            add_arc(fst, sa, sb, li, lo, sr.one);
            0
        }
        2 => {
            let sf: usize = parts[0].parse().unwrap();
            let w: f32 = parts[1].parse().unwrap();
            add_final(fst, sf, w);
            0
        }
        1 => {
            if let Ok(sf) = parts[0].parse::<usize>() {
                add_final(fst, sf, sr.one);
                0
            } else {
                -1
            }
        }
        _ => -1,
    }
}
fn parse_line_sym(fst: &mut fst::Fst, buf: &str, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> usize { if sst.n_items > 0 { trt(t, sst) } else { trn(t, sst) } };
    let itrans = |t: &str| -> usize { if ist.n_items > 0 { trt(t, ist) } else { trn(t, ist) } };
    let otrans = |t: &str| -> usize { if ost.n_items > 0 { trt(t, ost) } else { trn(t, ost) } };
    let parts: Vec<&str> = buf.trim().split_whitespace().collect();
    match parts.len() {
        5 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            let w: f32 = parts[4].parse().unwrap();
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX { return -1; }
            add_arc(fst, sa, sb, li, lo, w);
            0
        }
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX { return -1; }
            add_arc(fst, sa, sb, li, lo, sr.one);
            0
        }
        2 => {
            let sf = strans(parts[0]);
            let w: f32 = parts[1].parse().unwrap();
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            0
        }
        1 => {
            let sf = strans(parts[0]);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}
fn parse_line_sym_acc(fst: &mut fst::Fst, buf: &str, ist: &SymTable, _ost: &SymTable, sst: &SymTable) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> usize { if sst.n_items > 0 { trt(t, sst) } else { trn(t, sst) } };
    let itrans = |t: &str| -> usize { if ist.n_items > 0 { trt(t, ist) } else { trn(t, ist) } };
    let parts: Vec<&str> = buf.trim().split_whitespace().collect();
    match parts.len() {
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let w: f32 = parts[3].parse().unwrap();
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX { return -1; }
            add_arc(fst, sa, sb, li, li, w);
            0
        }
        3 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX { return -1; }
            add_arc(fst, sa, sb, li, li, sr.one);
            0
        }
        2 => {
            let sf = strans(parts[0]);
            let w: f32 = parts[1].parse().unwrap();
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            0
        }
        1 => {
            let sf = strans(parts[0]);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}
pub fn fst_compile(fst: &mut fst::Fst, fin: &mut dyn BufRead, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> fst::Fst {
    let mut line_buf = String::new();
    while fin.read_line(&mut line_buf).unwrap_or(0) > 0 {
        let line = line_buf.trim().to_string();
        if !line.is_empty() {
            let res = if !is_acc {
                parse_line_sym(fst, &line, ist, ost, sst)
            } else {
                parse_line_sym_acc(fst, &line, ist, ost, sst)
            };
            if res != 0 {
                eprintln!("Invalid input line: {}", line);
                std::process::exit(1);
            }
        }
        line_buf.clear();
    }
    if sst.n_items > 0 {
        if let Some(start_id) = sst.getr(fst::START_STATE) {
            if start_id >= 0 {
                fst.start = start_id as u32;
            }
        }
    }
    fst.clone()
}
pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    for line in s.split('\n') {
        let line = line.trim();
        if !line.is_empty() {
            if parse_line(fst, line) != 0 {
                eprintln!("Invalid input line: {}", line);
                std::process::exit(1);
            }
        }
    }
    fst.clone()
}
