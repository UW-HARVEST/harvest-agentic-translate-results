use crate::fst;
use crate::sr;
use crate::symt::SymTable;
use std::io::{self, BufRead};

#[allow(dead_code)]
const TOKEN_NONE: i64 = -1;

pub fn trn(token: &str, _symt: &SymTable) -> usize {
    // C trn parses to long integer; if not all digits consumed, returns -1
    match token.parse::<i64>() {
        Ok(v) if v >= 0 => v as usize,
        _ => usize::MAX,
    }
}

pub fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token) {
        Some(v) if v >= 0 => v as usize,
        _ => usize::MAX,
    }
}

pub fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while (sa as fst::State + 1 > fst.n_states) || (sb as fst::State + 1 > fst.n_states) {
        fst.add_state();
    }
    fst.add_arc(sa as fst::State, sb as fst::State, li as fst::Label, lo as fst::Label, w);
}

pub fn add_final(fst: &mut fst::Fst, s: usize, w: f32) {
    while s as fst::State + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as fst::State, w);
}

fn parse_line(fst: &mut fst::Fst, buf: &str) -> i32 {
    let sr_inst = sr::sr_get(fst.sr_type);
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

fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr_inst = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();

    let strans = |t: &str| -> usize {
        match sst {
            Some(st) => trt(t, st),
            None => trn(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> usize {
        match ist {
            Some(st) => trt(t, st),
            None => trn(t, &SymTable::new()),
        }
    };
    let otrans = |t: &str| -> usize {
        match ost {
            Some(st) => trt(t, st),
            None => trn(t, &SymTable::new()),
        }
    };

    match parts.len() {
        5 => {
            let _sa = strans(parts[0]);
            let _sb = strans(parts[1]);
            let _li = itrans(parts[2]);
            let _lo = otrans(parts[3]);
            let w = match parts[4].parse::<f32>() {
                Ok(v) => v,
                Err(_) => return -1,
            };
            if _sa == usize::MAX || _sb == usize::MAX || _li == usize::MAX || _lo == usize::MAX {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _lo, w);
            0
        }
        4 => {
            let _sa = strans(parts[0]);
            let _sb = strans(parts[1]);
            let _li = itrans(parts[2]);
            let _lo = otrans(parts[3]);
            if _sa == usize::MAX || _sb == usize::MAX || _li == usize::MAX || _lo == usize::MAX {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _lo, sr_inst.one);
            0
        }
        2 => {
            let _sf = strans(parts[0]);
            let w = match parts[1].parse::<f32>() {
                Ok(v) => v,
                Err(_) => return -1,
            };
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, w);
            0
        }
        1 => {
            let _sf = strans(parts[0]);
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, sr_inst.one);
            0
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
    let sr_inst = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let strans = |t: &str| -> usize {
        match sst {
            Some(st) => trt(t, st),
            None => trn(t, &SymTable::new()),
        }
    };
    let itrans = |t: &str| -> usize {
        match ist {
            Some(st) => trt(t, st),
            None => trn(t, &SymTable::new()),
        }
    };
    match parts.len() {
        4 => {
            let _sa = strans(parts[0]);
            let _sb = strans(parts[1]);
            let _li = itrans(parts[2]);
            let w = match parts[3].parse::<f32>() {
                Ok(v) => v,
                Err(_) => return -1,
            };
            if _sa == usize::MAX || _sb == usize::MAX || _li == usize::MAX {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _li, w);
            0
        }
        3 => {
            let _sa = strans(parts[0]);
            let _sb = strans(parts[1]);
            let _li = itrans(parts[2]);
            if _sa == usize::MAX || _sb == usize::MAX || _li == usize::MAX {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _li, sr_inst.one);
            0
        }
        2 => {
            let _sf = strans(parts[0]);
            let w = match parts[1].parse::<f32>() {
                Ok(v) => v,
                Err(_) => return -1,
            };
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, w);
            0
        }
        1 => {
            let _sf = strans(parts[0]);
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, sr_inst.one);
            0
        }
        _ => -1,
    }
}

pub fn compile_internal(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) -> io::Result<()> {
    let mut buf = String::new();
    let mut line: usize = 1;
    loop {
        buf.clear();
        let n = fin.read_line(&mut buf)?;
        if n == 0 {
            break;
        }
        line += 1;
        let res = if !is_acc {
            parse_line_sym(fst, &buf, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, &buf, ist, sst)
        };
        if res != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid input line {}: {}", line, buf),
            ));
        }
    }
    if let Some(sst) = sst {
        if let Some(start) = sst.getr(fst::START_STATE) {
            if start >= 0 {
                fst.start = start as fst::State;
            }
        }
    }
    Ok(())
}

pub fn compile_str_internal(fst: &mut fst::Fst, s: &str) {
    for line in s.split('\n') {
        if line.is_empty() {
            continue;
        }
        let _ = parse_line(fst, line);
    }
}

#[allow(dead_code)]
fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) -> fst::Fst {
    let _ = compile_internal(fst, fin, Some(ist), Some(ost), Some(sst), is_acc);
    std::mem::replace(fst, fst::Fst::new())
}

#[allow(dead_code)]
fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    compile_str_internal(fst, s);
    std::mem::replace(fst, fst::Fst::new())
}
