use crate::fst;
use crate::sr;
use std::io::BufRead;
use crate::symt::SymTable;

fn trn(token: &str, _symt: &SymTable) -> usize {
    // Numeric translation
    match token.parse::<i64>() {
        Ok(v) => v as usize,
        Err(_) => usize::MAX,
    }
}

fn trt(token: &str, symt: &SymTable) -> usize {
    match symt.getr(token) {
        Some(-1) | None => usize::MAX,
        Some(v) => v as usize,
    }
}

fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while sa as u32 + 1 > fst.n_states || sb as u32 + 1 > fst.n_states {
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

fn parse_line(fst: &mut fst::Fst, buf: &mut str) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();
    if toks.len() == 5 {
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (
            toks[0].parse::<usize>(),
            toks[1].parse::<usize>(),
            toks[2].parse::<usize>(),
            toks[3].parse::<usize>(),
            toks[4].parse::<f32>(),
        ) {
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if toks.len() == 4 {
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
            toks[0].parse::<usize>(),
            toks[1].parse::<usize>(),
            toks[2].parse::<usize>(),
            toks[3].parse::<usize>(),
        ) {
            add_arc(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
    }
    if toks.len() == 2 {
        if let (Ok(sf), Ok(w)) = (toks[0].parse::<usize>(), toks[1].parse::<f32>()) {
            add_final(fst, sf, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        if let Ok(sf) = toks[0].parse::<usize>() {
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
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();

    if toks.len() == 5 {
        if let Ok(w) = toks[4].parse::<f32>() {
            let sa = trt(toks[0], sst);
            let sb = trt(toks[1], sst);
            let li = trt(toks[2], ist);
            let lo = trt(toks[3], ost);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if toks.len() == 4 {
        let sa = trt(toks[0], sst);
        let sb = trt(toks[1], sst);
        let li = trt(toks[2], ist);
        let lo = trt(toks[3], ost);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX || lo == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, lo, sr.one);
        return 0;
    }
    if toks.len() == 2 {
        if let Ok(w) = toks[1].parse::<f32>() {
            let sf = trt(toks[0], sst);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        let sf = trt(toks[0], sst);
        if sf == usize::MAX { return -1; }
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
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();

    if toks.len() == 4 {
        if let Ok(w) = toks[3].parse::<f32>() {
            let sa = trt(toks[0], sst);
            let sb = trt(toks[1], sst);
            let li = trt(toks[2], ist);
            if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
                return -1;
            }
            add_arc(fst, sa, sb, li, li, w);
            return 0;
        }
    }
    if toks.len() == 3 {
        let sa = trt(toks[0], sst);
        let sb = trt(toks[1], sst);
        let li = trt(toks[2], ist);
        if sa == usize::MAX || sb == usize::MAX || li == usize::MAX {
            return -1;
        }
        add_arc(fst, sa, sb, li, li, sr.one);
        return 0;
    }
    if toks.len() == 2 {
        if let Ok(w) = toks[1].parse::<f32>() {
            let sf = trt(toks[0], sst);
            if sf == usize::MAX { return -1; }
            add_final(fst, sf, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        let sf = trt(toks[0], sst);
        if sf == usize::MAX { return -1; }
        add_final(fst, sf, sr.one);
        return 0;
    }
    -1
}

fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: &SymTable,
    ost: &SymTable,
    sst: &SymTable,
    is_acc: bool,
) -> fst::Fst {
    let mut line_no = 1usize;
    let mut line = String::new();
    loop {
        line.clear();
        match fin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let mut buf = line.clone();
        let res = if !is_acc {
            parse_line_sym(fst, &mut buf, ist, ost, sst)
        } else {
            parse_line_sym_acc(fst, &mut buf, ist, ost, sst)
        };
        if res != 0 {
            eprintln!("Invalid input line {}: {}", line_no, line);
            break;
        }
        line_no += 1;
    }
    if let Some(s) = sst.getr(crate::fst::START_STATE) {
        if s != -1 {
            fst.start = s as u32;
        }
    }
    fst::Fst::new()
}

fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    for (i, line) in s.lines().enumerate() {
        let mut buf = line.to_string();
        if parse_line(fst, &mut buf) != 0 {
            eprintln!("Invalid input line {}: {}", i + 1, line);
            break;
        }
    }
    fst::Fst::new()
}

// Suppress dead-code warnings for unused module-private helpers.
#[allow(dead_code)]
fn _suppress_dead_code() {
    // referencing each helper to keep them
    let _ = trn;
    let _ = fst_compile;
    let _ = fst_compile_str;
}
