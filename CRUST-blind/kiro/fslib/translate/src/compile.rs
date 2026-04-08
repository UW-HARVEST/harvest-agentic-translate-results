use crate::sr;
use crate::fst;
use crate::symt;
use std::io::{self, BufRead};
use crate::symt::SymTable;
fn trn(token: &str, _symt: &SymTable) -> usize {
    token.trim().parse::<usize>().unwrap_or(usize::MAX)
}
fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token.trim()) {
        Some(id) => id as usize,
        None => {
            eprintln!("Unknown token: {}", token);
            usize::MAX
        }
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
fn parse_line(fst: &mut fst::Fst, buf: &mut str) -> i32 {
    let buf = buf.trim();
    let parts: Vec<&str> = buf.split('\t').collect();
    let sr = sr::sr_get(fst.sr_type);
    match parts.len() {
        5 => {
            let sa = parts[0].parse::<usize>().unwrap_or(usize::MAX);
            let sb = parts[1].parse::<usize>().unwrap_or(usize::MAX);
            let li = parts[2].parse::<usize>().unwrap_or(usize::MAX);
            let lo = parts[3].parse::<usize>().unwrap_or(usize::MAX);
            let w = parts[4].parse::<f32>().unwrap_or(0.0);
            add_arc(fst, sa, sb, li, lo, w);
            0
        }
        4 => {
            let sa = parts[0].parse::<usize>().unwrap_or(usize::MAX);
            let sb = parts[1].parse::<usize>().unwrap_or(usize::MAX);
            let li = parts[2].parse::<usize>().unwrap_or(usize::MAX);
            let lo = parts[3].parse::<usize>().unwrap_or(usize::MAX);
            add_arc(fst, sa, sb, li, lo, sr.one);
            0
        }
        2 => {
            let sf = parts[0].parse::<usize>().unwrap_or(usize::MAX);
            let w = parts[1].parse::<f32>().unwrap_or(0.0);
            add_final(fst, sf, w);
            0
        }
        1 => {
            let sf = parts[0].parse::<usize>().unwrap_or(usize::MAX);
            add_final(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}
fn parse_line_sym(fst: &mut fst::Fst, buf: &mut str, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> i32 {
    let buf = buf.trim();
    let parts: Vec<&str> = buf.split('\t').collect();
    let sr = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> usize { if sst.n_items > 0 { trt(t, sst) } else { trn(t, sst) } };
    let itrans = |t: &str| -> usize { if ist.n_items > 0 { trt(t, ist) } else { trn(t, ist) } };
    let otrans = |t: &str| -> usize { if ost.n_items > 0 { trt(t, ost) } else { trn(t, ost) } };
    match parts.len() {
        5 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            let w = parts[4].parse::<f32>().unwrap_or(0.0);
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
            let w = parts[1].parse::<f32>().unwrap_or(0.0);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            0
        }
        1 => {
            if parts[0].is_empty() { return -1; }
            let sf = strans(parts[0]);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}
fn parse_line_sym_acc(fst: &mut fst::Fst, buf: &mut str, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> i32 {
    let buf_trimmed = buf.trim();
    let parts: Vec<&str> = buf_trimmed.split('\t').collect();
    let sr = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> usize { if sst.n_items > 0 { trt(t, sst) } else { trn(t, sst) } };
    let itrans = |t: &str| -> usize { if ist.n_items > 0 { trt(t, ist) } else { trn(t, ist) } };
    match parts.len() {
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let w = parts[3].parse::<f32>().unwrap_or(0.0);
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
            let w = parts[1].parse::<f32>().unwrap_or(0.0);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            0
        }
        1 => {
            if parts[0].is_empty() { return -1; }
            let sf = strans(parts[0]);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}
fn fst_compile(fst: &mut fst::Fst, fin: &mut dyn BufRead, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> fst::Fst {
    let mut line_num = 1usize;
    for line in fin.lines() {
        if let Ok(mut l) = line {
            line_num += 1;
            let res = if !is_acc {
                parse_line_sym(fst, &mut l, ist, ost, sst)
            } else {
                parse_line_sym_acc(fst, &mut l, ist, ost, sst)
            };
            if res != 0 {
                eprintln!("Invalid input line {}: {}", line_num, l);
            }
        }
    }
    if sst.n_items > 0 {
        if let Some(start) = sst.getr(fst::START_STATE) {
            if start >= 0 {
                fst.start = start as u32;
            }
        }
    }
    let mut result = fst::Fst::new();
    std::mem::swap(fst, &mut result);
    result
}
fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    for (i, line) in s.lines().enumerate() {
        let mut l = line.to_string();
        if parse_line(fst, &mut l) != 0 {
            eprintln!("Invalid input line {}: {}", i + 1, line);
        }
    }
    let mut result = fst::Fst::new();
    std::mem::swap(fst, &mut result);
    result
}
