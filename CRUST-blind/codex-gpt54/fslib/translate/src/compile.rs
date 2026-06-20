use crate::fst;
use crate::symt::SymTable;
use std::io::BufRead;

fn trn(token: &str, _symt: &SymTable) -> usize {
    if let Ok(value) = token.trim().parse::<usize>() {
        value
    } else {
        eprintln!("Incorrect token: {}", token);
        usize::MAX
    }
}

fn trt(token: &str, symt: &SymTable) -> usize {
    if symt.n_items == 0 {
        return trn(token, symt);
    }
    if let Some(value) = symt.getr(token) {
        value as usize
    } else {
        eprintln!("Unknown token: {}", token);
        usize::MAX
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

fn parse_line(fst: &mut fst::Fst, buf: &mut str) -> i32 {
    let sr = crate::sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.trim().split('\t').collect();
    match parts.as_slice() {
        [sa, sb, li, lo, w] => {
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (
                sa.parse::<usize>(),
                sb.parse::<usize>(),
                li.parse::<usize>(),
                lo.parse::<usize>(),
                w.parse::<f32>(),
            ) {
                add_arc(fst, sa, sb, li, lo, w);
                0
            } else {
                -1
            }
        }
        [sa, sb, li, lo] => {
            if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
                sa.parse::<usize>(),
                sb.parse::<usize>(),
                li.parse::<usize>(),
                lo.parse::<usize>(),
            ) {
                add_arc(fst, sa, sb, li, lo, sr.one());
                0
            } else {
                -1
            }
        }
        [sf, w] => {
            if let (Ok(sf), Ok(w)) = (sf.parse::<usize>(), w.parse::<f32>()) {
                add_final(fst, sf, w);
                0
            } else {
                -1
            }
        }
        [sf] => {
            if let Ok(sf) = sf.parse::<usize>() {
                add_final(fst, sf, sr.one());
                0
            } else {
                -1
            }
        }
        _ => -1,
    }
}

fn parse_line_sym(
    fst: &mut fst::Fst,
    buf: &mut str,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
) -> i32 {
    let sr = crate::sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.trim().split('\t').collect();
    match parts.as_slice() {
        [sa, sb, li, lo, w] => {
            let _sa = trt(sa, sst);
            let _sb = trt(sb, sst);
            let _li = trt(li, ist);
            let _lo = trt(lo, ost);
            if [_sa, _sb, _li, _lo].contains(&usize::MAX) {
                return -1;
            }
            if let Ok(w) = w.parse::<f32>() {
                add_arc(fst, _sa, _sb, _li, _lo, w);
                0
            } else {
                -1
            }
        }
        [sa, sb, li, lo] => {
            let _sa = trt(sa, sst);
            let _sb = trt(sb, sst);
            let _li = trt(li, ist);
            let _lo = trt(lo, ost);
            if [_sa, _sb, _li, _lo].contains(&usize::MAX) {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _lo, sr.one());
            0
        }
        [sf, w] => {
            let _sf = trt(sf, sst);
            if _sf == usize::MAX {
                return -1;
            }
            if let Ok(w) = w.parse::<f32>() {
                add_final(fst, _sf, w);
                0
            } else {
                -1
            }
        }
        [sf] => {
            let _sf = trt(sf, sst);
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, sr.one());
            0
        }
        _ => -1,
    }
}

fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &mut str,
    ist: &SymTable,
    _ost: &SymTable,
    sst: &SymTable,
) -> i32 {
    let sr = crate::sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.trim().split('\t').collect();
    match parts.as_slice() {
        [sa, sb, li, w] => {
            let _sa = trt(sa, sst);
            let _sb = trt(sb, sst);
            let _li = trt(li, ist);
            if [_sa, _sb, _li].contains(&usize::MAX) {
                return -1;
            }
            if let Ok(w) = w.parse::<f32>() {
                add_arc(fst, _sa, _sb, _li, _li, w);
                0
            } else {
                -1
            }
        }
        [sa, sb, li] => {
            let _sa = trt(sa, sst);
            let _sb = trt(sb, sst);
            let _li = trt(li, ist);
            if [_sa, _sb, _li].contains(&usize::MAX) {
                return -1;
            }
            add_arc(fst, _sa, _sb, _li, _li, sr.one());
            0
        }
        [sf, w] => {
            let _sf = trt(sf, sst);
            if _sf == usize::MAX {
                return -1;
            }
            if let Ok(w) = w.parse::<f32>() {
                add_final(fst, _sf, w);
                0
            } else {
                -1
            }
        }
        [sf] => {
            let _sf = trt(sf, sst);
            if _sf == usize::MAX {
                return -1;
            }
            add_final(fst, _sf, sr.one());
            0
        }
        _ => -1,
    }
}

fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) -> fst::Fst {
    let mut buf = String::new();
    let mut line = 1usize;
    loop {
        buf.clear();
        match fin.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                line += 1;
                let res = if !is_acc {
                    parse_line_sym(fst, buf.as_mut_str(), ist, ost, sst)
                } else {
                    parse_line_sym_acc(fst, buf.as_mut_str(), ist, ost, sst)
                };
                if res != 0 {
                    eprintln!("Invalid input line {}: {}", line, buf.trim_end());
                    std::process::exit(1);
                }
            }
            Err(_) => break,
        }
    }

    if let Some(start_state) = sst.getr(fst::START_STATE) {
        fst.start = start_state as u32;
    }

    fst.clone()
}

fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    for (line, entry) in s.lines().enumerate() {
        let mut owned = entry.to_string();
        if parse_line(fst, owned.as_mut_str()) != 0 {
            eprintln!("Invalid input line {}: {}", line + 1, entry);
            std::process::exit(1);
        }
    }
    fst.clone()
}
