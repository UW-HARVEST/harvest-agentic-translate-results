use crate::sr::sr_get;
use crate::fst::{Fst, START_STATE};
use crate::symt::SymTable;
use std::io::BufRead;

fn trn_token(token: &str, _symt: &SymTable) -> Option<usize> {
    token.trim().parse::<usize>().ok()
}
fn trt_token(token: &str, symt: &SymTable) -> Option<usize> {
    symt.getr(token).map(|i| i as usize)
}

pub fn add_arc(fst: &mut Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while (sa + 1 > fst.n_states as usize) || (sb + 1 > fst.n_states as usize) {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}

pub fn add_final(fst: &mut Fst, s: usize, w: f32) {
    while s + 1 > fst.n_states as usize {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}

pub fn parse_line(fst: &mut Fst, buf: &str) -> i32 {
    let sr = sr_get(fst.sr_type);
    let trimmed = buf.trim_end_matches(|c: char| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.len() {
        5 => {
            // sa sb li lo w
            let sa = parts[0].parse::<usize>();
            let sb = parts[1].parse::<usize>();
            let li = parts[2].parse::<usize>();
            let lo = parts[3].parse::<usize>();
            let w = parts[4].parse::<f32>();
            match (sa, sb, li, lo, w) {
                (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) => {
                    add_arc(fst, sa, sb, li, lo, w);
                    0
                }
                _ => -1,
            }
        }
        4 => {
            let sa = parts[0].parse::<usize>();
            let sb = parts[1].parse::<usize>();
            let li = parts[2].parse::<usize>();
            let lo = parts[3].parse::<usize>();
            match (sa, sb, li, lo) {
                (Ok(sa), Ok(sb), Ok(li), Ok(lo)) => {
                    add_arc(fst, sa, sb, li, lo, sr.one);
                    0
                }
                _ => -1,
            }
        }
        2 => {
            let sf = parts[0].parse::<usize>();
            let w = parts[1].parse::<f32>();
            match (sf, w) {
                (Ok(sf), Ok(w)) => {
                    add_final(fst, sf, w);
                    0
                }
                _ => -1,
            }
        }
        1 => {
            let sf = parts[0].parse::<usize>();
            match sf {
                Ok(sf) => {
                    add_final(fst, sf, sr.one);
                    0
                }
                _ => -1,
            }
        }
        _ => -1,
    }
}

pub fn parse_line_sym(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr_get(fst.sr_type);
    let trimmed = buf.trim_end_matches(|c: char| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let strans = |t: &str| -> Option<usize> {
        match sst {
            Some(st) => trt_token(t, st),
            None => trn_token(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> Option<usize> {
        match ist {
            Some(st) => trt_token(t, st),
            None => trn_token(t, &SymTable::new()),
        }
    };
    let otrans = |t: &str| -> Option<usize> {
        match ost {
            Some(st) => trt_token(t, st),
            None => trn_token(t, &SymTable::new()),
        }
    };
    match parts.len() {
        5 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            let w = parts[4].parse::<f32>();
            match (sa, sb, li, lo, w) {
                (Some(sa), Some(sb), Some(li), Some(lo), Ok(w)) => {
                    add_arc(fst, sa, sb, li, lo, w);
                    0
                }
                _ => -1,
            }
        }
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            match (sa, sb, li, lo) {
                (Some(sa), Some(sb), Some(li), Some(lo)) => {
                    add_arc(fst, sa, sb, li, lo, sr.one);
                    0
                }
                _ => -1,
            }
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>();
            match (sf, w) {
                (Some(sf), Ok(w)) => {
                    add_final(fst, sf, w);
                    0
                }
                _ => -1,
            }
        }
        1 => {
            let sf = strans(parts[0]);
            match sf {
                Some(sf) => {
                    add_final(fst, sf, sr.one);
                    0
                }
                _ => -1,
            }
        }
        _ => -1,
    }
}

pub fn parse_line_sym_acc(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr_get(fst.sr_type);
    let trimmed = buf.trim_end_matches(|c: char| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let strans = |t: &str| -> Option<usize> {
        match sst {
            Some(st) => trt_token(t, st),
            None => trn_token(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> Option<usize> {
        match ist {
            Some(st) => trt_token(t, st),
            None => trn_token(t, &SymTable::new()),
        }
    };
    match parts.len() {
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let w = parts[3].parse::<f32>();
            match (sa, sb, li, w) {
                (Some(sa), Some(sb), Some(li), Ok(w)) => {
                    add_arc(fst, sa, sb, li, li, w);
                    0
                }
                _ => -1,
            }
        }
        3 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            match (sa, sb, li) {
                (Some(sa), Some(sb), Some(li)) => {
                    add_arc(fst, sa, sb, li, li, sr.one);
                    0
                }
                _ => -1,
            }
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>();
            match (sf, w) {
                (Some(sf), Ok(w)) => {
                    add_final(fst, sf, w);
                    0
                }
                _ => -1,
            }
        }
        1 => {
            let sf = strans(parts[0]);
            match sf {
                Some(sf) => {
                    add_final(fst, sf, sr.one);
                    0
                }
                _ => -1,
            }
        }
        _ => -1,
    }
}

pub fn fst_compile(
    fst: &mut Fst,
    fin: &mut dyn BufRead,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) {
    let mut line_no = 1usize;
    let mut line = String::new();
    loop {
        line.clear();
        let n = fin.read_line(&mut line).unwrap_or(0);
        if n == 0 {
            break;
        }
        line_no += 1;
        let res = if !is_acc {
            parse_line_sym(fst, &line, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, &line, ist, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
            std::process::exit(1);
        }
    }
    if let Some(s) = sst {
        if let Some(start) = s.getr(START_STATE) {
            if start >= 0 {
                fst.start = start as u32;
            }
        }
    }
}

pub fn fst_compile_str(fst: &mut Fst, s: &str) {
    let mut line_no = 1usize;
    for tok in s.split('\n') {
        if tok.is_empty() {
            line_no += 1;
            continue;
        }
        if parse_line(fst, tok) != 0 {
            eprintln!("Invalid input line {}: {}", line_no, tok);
            std::process::exit(1);
        }
        line_no += 1;
    }
}
