use crate::sr::sr_get;
use crate::fst;
use crate::fst::{Fst, START_STATE};
use std::io::BufRead;
use crate::symt::SymTable;

fn trn(token: &str, _symt: &SymTable) -> usize {
    match token.parse::<i64>() {
        Ok(n) => n as usize,
        Err(_) => usize::MAX, // simulating size_t -1
    }
}

fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token) {
        Some(id) if id >= 0 => id as usize,
        _ => usize::MAX,
    }
}

fn add_arc(
    fst: &mut Fst,
    sa: usize,
    sb: usize,
    li: usize,
    lo: usize,
    w: f32,
) {
    while (sa as u32 + 1 > fst.n_states) || (sb as u32 + 1 > fst.n_states) {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}

fn add_final(fst: &mut Fst, s: usize, w: f32) {
    while s as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}

fn parse_line(fst: &mut Fst, buf: &mut str) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    if parts.len() == 5 {
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        let w = parts[4].parse::<f32>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (sa, sb, li, lo, w) {
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (sa, sb, li, lo) {
            add_arc(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
    }
    if parts.len() == 2 {
        let sf = parts[0].parse::<usize>();
        let w = parts[1].parse::<f32>();
        if let (Ok(sf), Ok(w)) = (sf, w) {
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = parts[0].parse::<usize>();
        if let Ok(sf) = sf {
            add_final(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

fn parse_line_sym(fst: &mut Fst, buf: &mut str, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let strans = trt;
    let itrans = trt;
    let otrans = trt;
    if parts.len() == 5 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let lo = otrans(parts[3], ost);
        let w = parts[4].parse::<f32>();
        if let Ok(w) = w {
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let lo = otrans(parts[3], ost);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, lo, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        let sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let Ok(w) = w {
            if sf == usize::MAX {
                return -1;
            }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0], sst);
        if sf == usize::MAX {
            return -1;
        }
        add_final(fst, sf, sr.one);
        return 0;
    }
    -1
}

fn parse_line_sym_acc(fst: &mut Fst, buf: &mut str, ist: &SymTable, _ost: &SymTable, sst: &SymTable) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let strans = trt;
    let itrans = trt;
    if parts.len() == 4 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let w = parts[3].parse::<f32>();
        if let Ok(w) = w {
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, li, w);
            return 0;
        }
    }
    if parts.len() == 3 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, li, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        let sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let Ok(w) = w {
            if sf == usize::MAX {
                return -1;
            }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0], sst);
        if sf == usize::MAX {
            return -1;
        }
        add_final(fst, sf, sr.one);
        return 0;
    }
    -1
}

fn fst_compile(fst: &mut Fst, fin: &mut dyn BufRead, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Fst {
    let mut line_no: usize = 1;
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = match fin.read_line(&mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        let mut tmp: String = buf.trim_end_matches(|c| c == '\n' || c == '\r').to_string();
        let res = if !is_acc {
            parse_line_sym(fst, tmp.as_mut_str(), ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, tmp.as_mut_str(), ist, ost, sst)
        };
        line_no += 1;
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, buf);
        }
    }
    if let Some(start_state) = sst.getr(START_STATE) {
        if start_state >= 0 {
            fst.start = start_state as u32;
        }
    }
    std::mem::replace(fst, Fst::new())
}

fn fst_compile_str(fst: &mut Fst, s: &str) -> Fst {
    let mut line_no: usize = 1;
    for line in s.split('\n') {
        if line.is_empty() {
            continue;
        }
        let mut tmp = line.to_string();
        if parse_line(fst, tmp.as_mut_str()) != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
        }
        line_no += 1;
    }
    std::mem::replace(fst, Fst::new())
}

// Workaround for unused `trn` reference — use it
#[allow(dead_code)]
fn _ensure_trn_used(token: &str, st: &SymTable) -> usize {
    trn(token, st)
}

#[allow(dead_code)]
fn _entry_compile(fst: &mut Fst, fin: &mut dyn BufRead, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Fst {
    fst_compile(fst, fin, ist, ost, sst, is_acc)
}

#[allow(dead_code)]
fn _entry_compile_str(fst: &mut Fst, s: &str) -> Fst {
    fst_compile_str(fst, s)
}

// Touch fst module to avoid unused-import warnings
#[allow(dead_code)]
fn _touch() {
    let _ = fst::EPS;
}
