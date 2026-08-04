use crate::fst;
use crate::sr::sr_get;
use crate::symt::SymTable;
use crate::fst::START_STATE;
use std::io::BufRead;

#[allow(dead_code)]
fn trn(token: &str, _symt: Option<&SymTable>) -> Option<usize> {
    token.trim().parse::<usize>().ok()
}

#[allow(dead_code)]
fn trt(token: &str, symt: Option<&SymTable>) -> Option<usize> {
    let st = symt?;
    st.getr(token).map(|v| v as usize)
}

fn translate(token: &str, symt: Option<&SymTable>) -> Option<usize> {
    match symt {
        None => trn(token, None),
        Some(st) => trt(token, Some(st)),
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

fn parse_line(fst: &mut fst::Fst, buf: &str) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
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
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();

    if parts.len() == 5 {
        let sa = translate(parts[0], sst);
        let sb = translate(parts[1], sst);
        let li = translate(parts[2], ist);
        let lo = translate(parts[3], ost);
        let w = parts[4].parse::<f32>();
        if let (Some(sa), Some(sb), Some(li), Some(lo), Ok(w)) = (sa, sb, li, lo, w) {
            add_arc(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = translate(parts[0], sst);
        let sb = translate(parts[1], sst);
        let li = translate(parts[2], ist);
        let lo = translate(parts[3], ost);
        if let (Some(sa), Some(sb), Some(li), Some(lo)) = (sa, sb, li, lo) {
            add_arc(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
    }
    if parts.len() == 2 {
        let sf = translate(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let (Some(sf), Ok(w)) = (sf, w) {
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        if let Some(sf) = translate(parts[0], sst) {
            add_final(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

fn parse_line_sym_acc(
    fst: &mut fst::Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    if parts.len() == 4 {
        let sa = translate(parts[0], sst);
        let sb = translate(parts[1], sst);
        let li = translate(parts[2], ist);
        let w = parts[3].parse::<f32>();
        if let (Some(sa), Some(sb), Some(li), Ok(w)) = (sa, sb, li, w) {
            add_arc(fst, sa, sb, li, li, w);
            return 0;
        }
    }
    if parts.len() == 3 {
        let sa = translate(parts[0], sst);
        let sb = translate(parts[1], sst);
        let li = translate(parts[2], ist);
        if let (Some(sa), Some(sb), Some(li)) = (sa, sb, li) {
            add_arc(fst, sa, sb, li, li, sr.one);
            return 0;
        }
    }
    if parts.len() == 2 {
        let sf = translate(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let (Some(sf), Ok(w)) = (sf, w) {
            add_final(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        if let Some(sf) = translate(parts[0], sst) {
            add_final(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

pub fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) -> std::io::Result<()> {
    let mut line_no: usize = 1;
    let mut line = String::new();
    loop {
        line.clear();
        let n = fin.read_line(&mut line)?;
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
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid input line {}: {}", line_no, line),
            ));
        }
    }
    if let Some(sst) = sst {
        if let Some(start) = sst.getr(START_STATE) {
            fst.start = start as u32;
        }
    }
    Ok(())
}

pub fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> std::io::Result<()> {
    for (line_no, line) in s.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        if parse_line(fst, line) != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid input line {}: {}", line_no + 1, line),
            ));
        }
    }
    Ok(())
}
