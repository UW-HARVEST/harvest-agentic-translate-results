use fslib::fst::Fst;
use fslib::compile::{fst_compile_str, parse_line};

#[test]
fn test_compile_str() {
    let mut fst = Fst::new();
    let s = "0\t1\t1\t2\t0.5\n1\t2\t3\t4\t1.0\n2\t0.0\n";
    fst_compile_str(&mut fst, s);
    assert_eq!(fst.n_states, 3);
    assert_eq!(fst.states[0].n_arcs, 1);
    assert_eq!(fst.states[1].n_arcs, 1);
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.state, 1);
    assert_eq!(a.ilabel, 1);
    assert_eq!(a.olabel, 2);
    assert_eq!(a.weight, 0.5);
    assert_eq!(fst.states[2].final_state, true);
    assert_eq!(fst.states[2].weight, 0.0);
}

#[test]
fn test_parse_line_arc_no_weight() {
    let mut fst = Fst::new();
    let res = parse_line(&mut fst, "0\t1\t5\t6");
    assert_eq!(res, 0);
    assert_eq!(fst.n_states, 2);
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.state, 1);
    assert_eq!(a.ilabel, 5);
    assert_eq!(a.olabel, 6);
    // tropical one is 0.0
    assert_eq!(a.weight, 0.0);
}

#[test]
fn test_parse_line_final_only() {
    let mut fst = Fst::new();
    let res = parse_line(&mut fst, "2");
    assert_eq!(res, 0);
    assert_eq!(fst.n_states, 3);
    assert_eq!(fst.states[2].final_state, true);
}

#[test]
fn test_parse_line_invalid() {
    let mut fst = Fst::new();
    let res = parse_line(&mut fst, "abc");
    assert_eq!(res, -1);
}

fn main() {}
