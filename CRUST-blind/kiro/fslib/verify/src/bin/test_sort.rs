use fslib::fst::Fst;
use fslib::sort;

#[test]
fn test_isort() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t5\t3\t1.0\n0\t1\t1\t7\t2.0\n0\t1\t3\t1\t3.0\n1\t0.0");
    fst.arc_sort(0); // isort
    assert_eq!(fst.flags & 0x01, 0x01);
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[1].ilabel, 3);
    assert_eq!(fst.states[0].arcs[2].ilabel, 5);
}

#[test]
fn test_osort() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t5\t3\t1.0\n0\t1\t1\t7\t2.0\n0\t1\t3\t1\t3.0\n1\t0.0");
    fst.arc_sort(1); // osort
    assert_eq!(fst.flags & 0x02, 0x02);
    assert_eq!(fst.states[0].arcs[0].olabel, 1);
    assert_eq!(fst.states[0].arcs[1].olabel, 3);
    assert_eq!(fst.states[0].arcs[2].olabel, 7);
}

fn main() {}
