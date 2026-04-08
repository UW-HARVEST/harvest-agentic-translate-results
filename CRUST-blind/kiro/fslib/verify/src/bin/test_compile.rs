use fslib::fst::Fst;
use fslib::compile;

#[test]
fn test_compile_str() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t2\t0.5\n1\t2\t3\t4\t1.0\n2\t0.0");
    assert_eq!(fst.n_states, 3);
    assert_eq!(fst.states[0].n_arcs, 1);
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[2].final_state, true);
    assert_eq!(fst.states[2].weight, 0.0);
    assert_eq!(fst.states[0].arcs[0].state, 1);
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[0].olabel, 2);
    assert_eq!(fst.states[0].arcs[0].weight, 0.5);
    assert_eq!(fst.states[1].arcs[0].state, 2);
    assert_eq!(fst.states[1].arcs[0].ilabel, 3);
    assert_eq!(fst.states[1].arcs[0].olabel, 4);
    assert_eq!(fst.states[1].arcs[0].weight, 1.0);
}

#[test]
fn test_compile_str_no_weight() {
    // Arc without weight should use sr.one (0.0 for tropical)
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t2\n1\t0.0");
    assert_eq!(fst.n_states, 2);
    assert_eq!(fst.states[0].arcs[0].weight, 0.0);
    assert_eq!(fst.states[1].final_state, true);
}

#[test]
fn test_compile_str_final_only() {
    let mut fst = Fst::new();
    fst.compile_str("0");
    assert_eq!(fst.n_states, 1);
    assert_eq!(fst.states[0].final_state, true);
}

fn main() {}
