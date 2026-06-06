use crate::fst;
use std::io::BufRead;
use crate::symt::SymTable;

#[allow(dead_code)]
fn trn(token: &str, _symt: &SymTable) -> usize {
    token.parse::<usize>().unwrap_or(0)
}

#[allow(dead_code)]
fn trt(token: &str, symt: &SymTable) -> usize {
    if let Some(v) = symt.getr(token) {
        if v >= 0 {
            return v as usize;
        }
    }
    usize::MAX
}

#[allow(dead_code)]
fn add_arc(fst: &mut fst::Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while sa as u32 + 1 > fst.n_states || sb as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}

#[allow(dead_code)]
fn add_final(fst: &mut fst::Fst, s: usize, w: f32) {
    while s as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}

#[allow(dead_code)]
fn parse_line(_fst: &mut fst::Fst, _buf: &mut str) -> i32 {
    0
}

#[allow(dead_code)]
fn parse_line_sym(
    _fst: &mut fst::Fst,
    _buf: &mut str,
    _ist: &SymTable,
    _ost: &SymTable,
    _sst: &SymTable,
) -> i32 {
    0
}

#[allow(dead_code)]
fn parse_line_sym_acc(
    _fst: &mut fst::Fst,
    _buf: &mut str,
    _ist: &SymTable,
    _ost: &SymTable,
    _sst: &SymTable,
) -> i32 {
    0
}

#[allow(dead_code)]
fn fst_compile(
    fst: &mut fst::Fst,
    fin: &mut dyn BufRead,
    _ist: &SymTable,
    _ost: &SymTable,
    _sst: &SymTable,
    _is_acc: bool,
) -> fst::Fst {
    let mut s = String::new();
    let _ = fin.read_to_string(&mut s);
    fst.compile_str(&s);
    fst::Fst::new()
}

#[allow(dead_code)]
fn fst_compile_str(fst: &mut fst::Fst, s: &str) -> fst::Fst {
    fst.compile_str(s);
    fst::Fst::new()
}
