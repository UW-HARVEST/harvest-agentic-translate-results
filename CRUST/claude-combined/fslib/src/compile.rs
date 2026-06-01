use crate::fst;
use std::io::BufRead;
use crate::sr;
use crate::symt::SymTable;
#[allow(dead_code)]
fn trn(token: &str, _symt: &SymTable) -> i64 {
    token.parse::<i64>().unwrap_or(-1)
}
#[allow(dead_code)]
fn trt(token: &str, symt: &SymTable) -> i64 {
    match symt.getr(token) {
        Some(id) => id as i64,
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
    let trimmed = buf.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr_inst = sr::sr_get(fst.sr_type);
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
            -1
        }
        4 => {
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<usize>(),
                parts[2].parse::<usize>(),
                parts[3].parse::<usize>(),
            ) {
                add_arc(fst, sa, sb, li, lo, sr_inst.one);
                return 0;
            }
            -1
        }
        2 => {
            if let (Ok(sf), Ok(w)) = (parts[0].parse::<usize>(), parts[1].parse::<f32>()) {
                add_final(fst, sf, w);
                return 0;
            }
            -1
        }
        1 => {
            if let Ok(sf) = parts[0].parse::<usize>() {
                add_final(fst, sf, sr_inst.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}
pub fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let parts: Vec<&str> = buf.trim().split_whitespace().collect();
    let sr_inst = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> i64 {
        match sst {
            Some(s) => trt(t, s),
            None => trn(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> i64 {
        match ist {
            Some(s) => trt(t, s),
            None => trn(t, &SymTable::new()),
        }
    };
    let otrans = |t: &str| -> i64 {
        match ost {
            Some(s) => trt(t, s),
            None => trn(t, &SymTable::new()),
        }
    };
    match parts.len() {
        5 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            let w = parts[4].parse::<f32>().unwrap_or(0.0);
            if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, w);
            0
        }
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, lo as usize, sr_inst.one);
            0
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>().unwrap_or(0.0);
            if sf < 0 {
                return -1;
            }
            add_final(fst, sf as usize, w);
            0
        }
        1 => {
            let sf = strans(parts[0]);
            if sf < 0 {
                return -1;
            }
            add_final(fst, sf as usize, sr_inst.one);
            0
        }
        _ => -1,
    }
}
pub fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let parts: Vec<&str> = buf.trim().split_whitespace().collect();
    let sr_inst = sr::sr_get(fst.sr_type);
    let strans = |t: &str| -> i64 {
        match sst {
            Some(s) => trt(t, s),
            None => trn(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> i64 {
        match ist {
            Some(s) => trt(t, s),
            None => trn(t, &SymTable::new()),
        }
    };
    match parts.len() {
        4 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let w = parts[3].parse::<f32>().unwrap_or(0.0);
            if sa < 0 || sb < 0 || li < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, w);
            0
        }
        3 => {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            if sa < 0 || sb < 0 || li < 0 {
                return -1;
            }
            add_arc(fst, sa as usize, sb as usize, li as usize, li as usize, sr_inst.one);
            0
        }
        2 => {
            let sf = strans(parts[0]);
            let w = parts[1].parse::<f32>().unwrap_or(0.0);
            if sf < 0 {
                return -1;
            }
            add_final(fst, sf as usize, w);
            0
        }
        1 => {
            let sf = strans(parts[0]);
            if sf < 0 {
                return -1;
            }
            add_final(fst, sf as usize, sr_inst.one);
            0
        }
        _ => -1,
    }
}
#[allow(dead_code)]
pub fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) -> fst::Fst {
    let mut line = String::new();
    let mut line_no: usize = 0;
    loop {
        line.clear();
        let n = fin.read_line(&mut line).unwrap_or(0);
        if n == 0 { break; }
        line_no += 1;
        let trimmed = line.trim_end_matches(|c: char| c == '\n' || c == '\r');
        let res = if !is_acc {
            parse_line_sym(fst, trimmed, Some(ist), Some(ost), Some(sst))
        } else {
            parse_line_sym_acc(fst, trimmed, Some(ist), Some(sst))
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
        }
    }
    if let Some(id) = sst.getr(crate::fst::START_STATE) {
        if id != -1 {
            fst.start = id as u32;
        }
    }
    fst.clone()
}
#[allow(dead_code)]
pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    for line in s.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let _ = parse_line(fst, line);
    }
    fst.clone()
}
