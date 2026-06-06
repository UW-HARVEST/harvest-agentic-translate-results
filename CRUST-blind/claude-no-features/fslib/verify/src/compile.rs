use crate::fst;
use crate::sr;
use crate::symt::SymTable;
use std::io::BufRead;
use crate::fst::START_STATE;
fn trn(token: &str, _symt: Option<&SymTable>) -> Option<i32> {
    // try to parse as integer
    token.parse::<i32>().ok()
}
fn trt(token: &str, symt: Option<&SymTable>) -> Option<i32> {
    if let Some(st) = symt {
        st.getr(token)
    } else {
        None
    }
}
fn trans(token: &str, st: Option<&SymTable>) -> Option<i32> {
    if st.is_none() {
        trn(token, st)
    } else {
        trt(token, st)
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

// Try to parse a numeric line (5 fields, 4 fields, 2 fields, 1 field)
fn parse_line(fst: &mut fst::Fst, buf: &str) -> i32 {
    let sr_v = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    match parts.len() {
        5 => {
            // sa sb li lo w
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<usize>(),
                parts[2].parse::<usize>(),
                parts[3].parse::<usize>(),
                parts[4].parse::<f32>(),
            ) {
                add_arc(fst, sa, sb, li, lo, w);
                return 0;
            }
        }
        4 => {
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<usize>(),
                parts[2].parse::<usize>(),
                parts[3].parse::<usize>(),
            ) {
                add_arc(fst, sa, sb, li, lo, sr_v.one);
                return 0;
            }
        }
        2 => {
            if let (Ok(sf), Ok(w)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<f32>(),
            ) {
                add_final(fst, sf, w);
                return 0;
            }
        }
        1 => {
            if let Ok(sf) = parts[0].parse::<usize>() {
                add_final(fst, sf, sr_v.one);
                return 0;
            }
        }
        _ => {}
    }
    -1
}

fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr_v = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    match parts.len() {
        5 => {
            let sa = trans(parts[0], sst);
            let sb = trans(parts[1], sst);
            let li = trans(parts[2], ist);
            let lo = trans(parts[3], ost);
            let w = parts[4].parse::<f32>().ok();
            if let (Some(sa), Some(sb), Some(li), Some(lo), Some(w)) = (sa, sb, li, lo, w) {
                if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                    return -1;
                }
                add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, w);
                return 0;
            }
            return -1;
        }
        4 => {
            let sa = trans(parts[0], sst);
            let sb = trans(parts[1], sst);
            let li = trans(parts[2], ist);
            let lo = trans(parts[3], ost);
            if let (Some(sa), Some(sb), Some(li), Some(lo)) = (sa, sb, li, lo) {
                if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                    return -1;
                }
                add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, sr_v.one);
                return 0;
            }
            return -1;
        }
        2 => {
            let sf = trans(parts[0], sst);
            let w = parts[1].parse::<f32>().ok();
            if let (Some(sf), Some(w)) = (sf, w) {
                if sf < 0 {
                    return -1;
                }
                add_final(fst, sf as usize, w);
                return 0;
            }
            return -1;
        }
        1 => {
            let sf = trans(parts[0], sst);
            if let Some(sf) = sf {
                if sf < 0 {
                    return -1;
                }
                add_final(fst, sf as usize, sr_v.one);
                return 0;
            }
            return -1;
        }
        _ => {}
    }
    -1
}

fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr_v = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    match parts.len() {
        4 => {
            let sa = trans(parts[0], sst);
            let sb = trans(parts[1], sst);
            let li = trans(parts[2], ist);
            let w = parts[3].parse::<f32>().ok();
            if let (Some(sa), Some(sb), Some(li), Some(w)) = (sa, sb, li, w) {
                if sa < 0 || sb < 0 || li < 0 {
                    return -1;
                }
                add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, w);
                return 0;
            }
            return -1;
        }
        3 => {
            let sa = trans(parts[0], sst);
            let sb = trans(parts[1], sst);
            let li = trans(parts[2], ist);
            if let (Some(sa), Some(sb), Some(li)) = (sa, sb, li) {
                if sa < 0 || sb < 0 || li < 0 {
                    return -1;
                }
                add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, sr_v.one);
                return 0;
            }
            return -1;
        }
        2 => {
            let sf = trans(parts[0], sst);
            let w = parts[1].parse::<f32>().ok();
            if let (Some(sf), Some(w)) = (sf, w) {
                if sf < 0 {
                    return -1;
                }
                add_final(fst, sf as usize, w);
                return 0;
            }
            return -1;
        }
        1 => {
            let sf = trans(parts[0], sst);
            if let Some(sf) = sf {
                if sf < 0 {
                    return -1;
                }
                add_final(fst, sf as usize, sr_v.one);
                return 0;
            }
            return -1;
        }
        _ => {}
    }
    -1
}

pub fn fst_compile_into(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) {
    let mut line = String::new();
    let mut line_no = 1usize;
    loop {
        line.clear();
        let n = match fin.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        line_no += 1;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.is_empty() {
            continue;
        }
        let res = if !is_acc {
            parse_line_sym(fst, trimmed, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, trimmed, ist, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
            return;
        }
    }
    if let Some(sst) = sst {
        if let Some(start) = sst.getr(START_STATE) {
            if start >= 0 {
                fst.start = start as u32;
            }
        }
    }
}

pub fn fst_compile_str_into(fst: &mut fst::Fst, s: &str) {
    let mut line_no = 1usize;
    for tok in s.split('\n') {
        let trimmed = tok.trim_end_matches('\r');
        if trimmed.is_empty() {
            line_no += 1;
            continue;
        }
        if parse_line(fst, trimmed) != 0 {
            eprintln!("Invalid input line {}: {}", line_no, trimmed);
            return;
        }
        line_no += 1;
    }
}
