use fslib::fst::Fst;
use fslib::print;

#[test]
fn test_fst_print() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t2\t0.5\n1\t2\t3\t4\t1.0\n2\t0.0");
    let mut buf = Vec::new();
    print::fst_print(&fst, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("0\t1\t1\t2\t0.50000"));
    assert!(output.contains("1\t2\t3\t4\t1.00000"));
    assert!(output.contains("2\t0"));
}

#[test]
fn test_fst_print_sym() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t2\t0.5\n1\t0.0");
    let mut buf = Vec::new();
    print::fst_print_sym(&fst, None, None, None, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("0\t1\t1\t2\t0.50000"));
}

fn main() {}
