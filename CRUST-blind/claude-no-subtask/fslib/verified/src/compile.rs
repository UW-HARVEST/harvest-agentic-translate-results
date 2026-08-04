#[allow(unused_imports)]
use crate::sr;
use crate::fst;
#[allow(unused_imports)]
use crate::symt;
use std::io::BufRead;
use crate::symt::SymTable;
use crate::sr::sr_get;
use crate::fst::START_STATE;

fn trn(token: &str, _symt: &SymTable) -> usize {
    match token.parse::<usize>() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Incorrect token: {}", token);
            usize::MAX
        }
    }
}

fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token) {
        Some(v) => v as usize,
        None => {
            eprintln!("Unknown token: {}", token);
            usize::MAX
        }
    }
}

fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while (sa + 1) > fst.n_states as usize || (sb + 1) > fst.n_states as usize {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}

fn add_final(fst: &mut fst::Fst, s: usize, w: f32) {
    while (s + 1) > fst.n_states as usize {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}

fn parse_line(fst: &mut fst::Fst, buf: &mut str) -> i32 {
    let trimmed = buf.trim_end_matches(|c| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);
    if parts.len() == 5 {
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
            add_arc(fst, sa, sb, li, lo, sr.one);
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
            add_final(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &mut str,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
) -> i32 {
    let trimmed = buf.trim_end_matches(|c| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);

    let strans = |t: &str| trt(t, sst);
    let itrans = |t: &str| trt(t, ist);
    let otrans = |t: &str| trt(t, ost);

    if parts.len() == 5 {
        if let Ok(w) = parts[4].parse::<f32>() {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            let lo = otrans(parts[3]);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = strans(parts[0]);
        let sb = strans(parts[1]);
        let li = itrans(parts[2]);
        let lo = otrans(parts[3]);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, lo, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        if let Ok(w) = parts[1].parse::<f32>() {
            let sf = strans(parts[0]);
            if sf == usize::MAX {
                return -1;
            }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0]);
        if sf == usize::MAX {
            return -1;
        }
        add_final(fst, sf, sr.one);
        return 0;
    }
    -1
}

fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &mut str,
    ist: &SymTable,
    _ost: &SymTable,
    sst: &SymTable,
) -> i32 {
    let trimmed = buf.trim_end_matches(|c| c == '\n' || c == '\r');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sr = sr_get(fst.sr_type);

    let strans = |t: &str| trt(t, sst);
    let itrans = |t: &str| trt(t, ist);

    if parts.len() == 4 {
        if let Ok(w) = parts[3].parse::<f32>() {
            let sa = strans(parts[0]);
            let sb = strans(parts[1]);
            let li = itrans(parts[2]);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, li, w);
            return 0;
        }
    }
    if parts.len() == 3 {
        let sa = strans(parts[0]);
        let sb = strans(parts[1]);
        let li = itrans(parts[2]);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, li, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        if let Ok(w) = parts[1].parse::<f32>() {
            let sf = strans(parts[0]);
            if sf == usize::MAX {
                return -1;
            }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0]);
        if sf == usize::MAX {
            return -1;
        }
        add_final(fst, sf, sr.one);
        return 0;
    }
    -1
}

pub fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) -> fst::Fst {
    let mut line = String::new();
    let mut line_no = 1;
    loop {
        line.clear();
        match fin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        line_no += 1;
        let mut owned = line.clone();
        let res = if !is_acc {
            parse_line_sym(fst, owned.as_mut_str(), ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, owned.as_mut_str(), ist, ost, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
            break;
        }
    }
    if let Some(start_state) = sst.getr(START_STATE) {
        fst.start = start_state as u32;
    }
    let mut out = fst::Fst::new();
    fst.copy(&mut out);
    out
}

pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    let mut line_no = 1;
    for line in s.split('\n') {
        let mut owned = line.to_string();
        if owned.is_empty() {
            line_no += 1;
            continue;
        }
        if parse_line(fst, owned.as_mut_str()) != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
            break;
        }
        line_no += 1;
    }
    let mut out = fst::Fst::new();
    fst.copy(&mut out);
    out
}
