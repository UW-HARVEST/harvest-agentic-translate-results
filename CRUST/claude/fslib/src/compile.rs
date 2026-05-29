use crate::fst;
use crate::fst::START_STATE;
use crate::sr::sr_get;
use std::io::BufRead;
use crate::symt::SymTable;
fn trn(token: &str, _symt: Option<&SymTable>) -> Option<i64> {
    match token.parse::<i64>() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}
fn trt(token: &str, symt: Option<&SymTable>) -> Option<i64> {
    if let Some(st) = symt {
        match st.getr(token) {
            Some(-1) => Some(-1),
            Some(v) => Some(v as i64),
            None => Some(-1),
        }
    } else {
        None
    }
}
fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while (sa as u32 + 1 > fst.n_states) || (sb as u32 + 1 > fst.n_states) {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}
fn add_final(fst: &mut fst::Fst, s: usize, w: f32) {
    while s as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}
fn parse_line(fst: &mut fst::Fst, line: &str) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);
    if parts.len() == 5 {
        // arc with weight
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        let w = parts[4].parse::<f32>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (sa, sb, li, lo, w) {
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 4 {
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (sa, sb, li, lo) {
            add_arc(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
        return -1;
    } else if parts.len() == 2 {
        let sf = parts[0].parse::<usize>();
        let w = parts[1].parse::<f32>();
        if let (Ok(sf), Ok(w)) = (sf, w) {
            add_final(fst, sf, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 1 {
        let sf = parts[0].parse::<usize>();
        if let Ok(sf) = sf {
            add_final(fst, sf, sr.one);
            return 0;
        }
        return -1;
    }
    -1
}
fn parse_line_sym(fst: &mut fst::Fst, line: &str, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);
    let strans = if sst.is_none() { trn } else { trt };
    let itrans = if ist.is_none() { trn } else { trt };
    let otrans = if ost.is_none() { trn } else { trt };
    if parts.len() == 5 {
        let _sa = strans(parts[0], sst);
        let _sb = strans(parts[1], sst);
        let _li = itrans(parts[2], ist);
        let _lo = otrans(parts[3], ost);
        let w = parts[4].parse::<f32>().ok();
        if let (Some(sa), Some(sb), Some(li), Some(lo), Some(w)) = (_sa, _sb, _li, _lo, w) {
            if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 4 {
        let _sa = strans(parts[0], sst);
        let _sb = strans(parts[1], sst);
        let _li = itrans(parts[2], ist);
        let _lo = otrans(parts[3], ost);
        if let (Some(sa), Some(sb), Some(li), Some(lo)) = (_sa, _sb, _li, _lo) {
            if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, sr.one);
            return 0;
        }
        return -1;
    } else if parts.len() == 2 {
        let _sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>().ok();
        if let (Some(sf), Some(w)) = (_sf, w) {
            if sf < 0 { return -1; }
            add_final(fst, sf as usize, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 1 {
        let _sf = strans(parts[0], sst);
        if let Some(sf) = _sf {
            if sf < 0 { return -1; }
            add_final(fst, sf as usize, sr.one);
            return 0;
        }
        return -1;
    }
    -1
}
fn parse_line_sym_acc(fst: &mut fst::Fst, line: &str, ist: Option<&SymTable>, sst: Option<&SymTable>) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);
    let strans = if sst.is_none() { trn } else { trt };
    let itrans = if ist.is_none() { trn } else { trt };
    if parts.len() == 4 {
        let _sa = strans(parts[0], sst);
        let _sb = strans(parts[1], sst);
        let _li = itrans(parts[2], ist);
        let w = parts[3].parse::<f32>().ok();
        if let (Some(sa), Some(sb), Some(li), Some(w)) = (_sa, _sb, _li, w) {
            if sa < 0 || sb < 0 || li < 0 { return -1; }
            add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 3 {
        let _sa = strans(parts[0], sst);
        let _sb = strans(parts[1], sst);
        let _li = itrans(parts[2], ist);
        if let (Some(sa), Some(sb), Some(li)) = (_sa, _sb, _li) {
            if sa < 0 || sb < 0 || li < 0 { return -1; }
            add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, sr.one);
            return 0;
        }
        return -1;
    } else if parts.len() == 2 {
        let _sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>().ok();
        if let (Some(sf), Some(w)) = (_sf, w) {
            if sf < 0 { return -1; }
            add_final(fst, sf as usize, w);
            return 0;
        }
        return -1;
    } else if parts.len() == 1 {
        let _sf = strans(parts[0], sst);
        if let Some(sf) = _sf {
            if sf < 0 { return -1; }
            add_final(fst, sf as usize, sr.one);
            return 0;
        }
        return -1;
    }
    -1
}
pub fn fst_compile_pub(fst: &mut fst::Fst, fin: &mut dyn BufRead, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>, is_acc: bool) {
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = fin.read_line(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        let res = if !is_acc {
            parse_line_sym(fst, &buf, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, &buf, ist, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line: {}", buf);
        }
    }
    if let Some(sst) = sst {
        if let Some(s) = sst.getr(START_STATE) {
            if s != -1 {
                fst.start = s as u32;
            }
        }
    }
}
pub fn fst_compile_str_pub(fst: &mut fst::Fst, s: &str) {
    for line in s.lines() {
        if line.is_empty() { continue; }
        if parse_line(fst, line) != 0 {
            eprintln!("Invalid input line: {}", line);
        }
    }
}
