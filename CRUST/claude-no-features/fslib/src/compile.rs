use crate::fst;
use crate::fst::START_STATE;
use crate::sr;
use crate::symt::SymTable;
use std::io::BufRead;

#[allow(dead_code)]
fn trn(token: &str, _symt: &SymTable) -> i64 {
    match token.parse::<i64>() {
        Ok(v) => v,
        Err(_) => -1,
    }
}

#[allow(dead_code)]
fn trt(token: &str, symt: &SymTable) -> i64 {
    match symt.getr(token) {
        Some(v) => v as i64,
        None => -1,
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

pub fn parse_line(fst: &mut fst::Fst, buf: &str) -> i32 {
    let sr_struct = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    if parts.len() == 5 {
        // src dst il ol weight
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
    if parts.len() == 4 {
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
            parts[0].parse::<usize>(),
            parts[1].parse::<usize>(),
            parts[2].parse::<usize>(),
            parts[3].parse::<usize>(),
        ) {
            add_arc(fst, sa, sb, li, lo, sr_struct.one);
            return 0;
        }
    }
    if parts.len() == 2 {
        if let (Ok(sf), Ok(w)) = (parts[0].parse::<usize>(), parts[1].parse::<f32>()) {
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        if let Ok(sf) = parts[0].parse::<usize>() {
            add_final(fst, sf, sr_struct.one);
            return 0;
        }
    }
    -1
}

fn translate(token: &str, symt: Option<&SymTable>) -> i64 {
    match symt {
        Some(t) => match t.getr(token) {
            Some(v) => v as i64,
            None => -1,
        },
        None => match token.parse::<i64>() {
            Ok(v) => v,
            Err(_) => -1,
        },
    }
}

pub fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr_struct = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    if parts.len() == 5 {
        let _sa = translate(parts[0], sst);
        let _sb = translate(parts[1], sst);
        let _li = translate(parts[2], ist);
        let _lo = translate(parts[3], ost);
        let w = parts[4].parse::<f32>();
        if let Ok(w) = w {
            if _sa < 0 || _sb < 0 || _li < 0 || _lo < 0 {
                return -1;
            }
            add_arc(fst, _sa as usize, _sb as usize, _li as usize, _lo as usize, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let _sa = translate(parts[0], sst);
        let _sb = translate(parts[1], sst);
        let _li = translate(parts[2], ist);
        let _lo = translate(parts[3], ost);
        if _sa < 0 || _sb < 0 || _li < 0 || _lo < 0 {
            return -1;
        }
        add_arc(fst, _sa as usize, _sb as usize, _li as usize, _lo as usize, sr_struct.one);
        return 0;
    }
    if parts.len() == 2 {
        let _sf = translate(parts[0], sst);
        if let Ok(w) = parts[1].parse::<f32>() {
            if _sf < 0 {
                return -1;
            }
            add_final(fst, _sf as usize, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let _sf = translate(parts[0], sst);
        if _sf < 0 {
            return -1;
        }
        add_final(fst, _sf as usize, sr_struct.one);
        return 0;
    }
    -1
}

pub fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr_struct = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    if parts.len() == 4 {
        let _sa = translate(parts[0], sst);
        let _sb = translate(parts[1], sst);
        let _li = translate(parts[2], ist);
        if let Ok(w) = parts[3].parse::<f32>() {
            if _sa < 0 || _sb < 0 || _li < 0 {
                return -1;
            }
            add_arc(fst, _sa as usize, _sb as usize, _li as usize, _li as usize, w);
            return 0;
        }
    }
    if parts.len() == 3 {
        let _sa = translate(parts[0], sst);
        let _sb = translate(parts[1], sst);
        let _li = translate(parts[2], ist);
        if _sa < 0 || _sb < 0 || _li < 0 {
            return -1;
        }
        add_arc(fst, _sa as usize, _sb as usize, _li as usize, _li as usize, sr_struct.one);
        return 0;
    }
    if parts.len() == 2 {
        let _sf = translate(parts[0], sst);
        if let Ok(w) = parts[1].parse::<f32>() {
            if _sf < 0 {
                return -1;
            }
            add_final(fst, _sf as usize, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let _sf = translate(parts[0], sst);
        if _sf < 0 {
            return -1;
        }
        add_final(fst, _sf as usize, sr_struct.one);
        return 0;
    }
    -1
}

pub fn parse_line_sym_dispatch(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) -> i32 {
    if is_acc {
        parse_line_sym_acc(fst, buf, ist, sst)
    } else {
        parse_line_sym(fst, buf, ist, ost, sst)
    }
}

pub fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) {
    let mut line = String::new();
    loop {
        line.clear();
        let n = match fin.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        parse_line_sym_dispatch(fst, &line, Some(ist), Some(ost), Some(sst), is_acc);
    }
    if let Some(s) = sst.getr(START_STATE) {
        if s >= 0 {
            fst.start = s as u32;
        }
    }
}

pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) {
    for line in s.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        parse_line(fst, line);
    }
}
