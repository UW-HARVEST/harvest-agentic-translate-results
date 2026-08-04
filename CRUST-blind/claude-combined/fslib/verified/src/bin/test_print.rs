use fslib::fst::Fst;
use fslib::print::{fst_print, fst_print_sym};

#[test]
fn test_print_basic() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.set_final(1, 0.0);
    let mut buf: Vec<u8> = Vec::new();
    fst_print(&fst, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("0\t1\t1\t2\t0.50000"));
    assert!(s.contains("1\t0.000000"));
}

#[test]
fn test_print_sym_no_symt() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 6, 1.0);
    fst.set_final(1, 0.5);
    let mut buf: Vec<u8> = Vec::new();
    fst_print_sym(&fst, None, None, None, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("0\t1\t5\t6\t1.00000"));
    assert!(s.contains("1\t0.500000"));
}

fn main() {}
