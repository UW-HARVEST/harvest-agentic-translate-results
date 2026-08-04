use fslib::fst::Fst;
use fslib::print::{fst_print, fst_print_sym};
use fslib::symt::SymTable;

#[test]
fn test_print_basic() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 1, 2, 0.5);
    f.set_final(1, 0.0);
    let mut buf: Vec<u8> = Vec::new();
    fst_print(&f, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    // Output: arc lines, then final lines
    // For arc: "0\t1\t1\t2\t0.50000\n"
    // For final: "1\t0\n"
    assert!(s.contains("0\t1\t1\t2\t0.50000\n"));
    assert!(s.contains("1\t0\n"));
}

#[test]
fn test_print_multiple_arcs() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(1, 2, 2, 2, 1.0);
    f.set_final(2, 0.5);
    let mut buf: Vec<u8> = Vec::new();
    fst_print(&f, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("0\t1\t1\t1\t0.00000\n"));
    assert!(s.contains("1\t2\t2\t2\t1.00000\n"));
    assert!(s.contains("2\t0.5\n"));
}

#[test]
fn test_print_no_final() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 6, 0.0);
    let mut buf: Vec<u8> = Vec::new();
    fst_print(&f, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("0\t1\t5\t6\t0.00000\n"));
    // No final lines, but the function must complete
}

#[test]
fn test_print_sym_with_tables() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 1, 2, 0.0);
    f.set_final(1, 0.0);
    let mut ist = SymTable::new();
    ist.add(0, "<eps>");
    ist.add(1, "a");
    let mut ost = SymTable::new();
    ost.add(0, "<eps>");
    ost.add(1, "x");
    ost.add(2, "y");
    let mut sst = SymTable::new();
    sst.add(0, "S0");
    sst.add(1, "S1");

    let mut buf: Vec<u8> = Vec::new();
    fst_print_sym(&f, Some(&ist), Some(&ost), Some(&sst), &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("S0\tS1\ta\ty\t0.00000\n"));
    assert!(s.contains("S1\t0\n"));
}

#[test]
fn test_print_sym_no_tables_uses_id() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 6, 0.0);
    f.set_final(1, 0.0);
    let mut buf: Vec<u8> = Vec::new();
    fst_print_sym(&f, None, None, None, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("0\t1\t5\t6\t0.00000\n"));
    assert!(s.contains("1\t0\n"));
}

fn main() {}
