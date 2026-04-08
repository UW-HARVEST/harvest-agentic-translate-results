use fslib::fst::Fst;

#[test]
fn test_compose() {
    let mut a = Fst::new();
    a.compile_str("0\t1\t1\t2\t1.0\n1\t0.0");
    let mut b = Fst::new();
    b.compile_str("0\t1\t2\t3\t2.0\n1\t0.0");
    let mut c = Fst::new();
    a.compose(&b, &mut c);
    assert_eq!(c.n_states, 2);
    assert_eq!(c.start, 0);
    // State 0: not final, 1 arc
    assert_eq!(c.states[0].final_state, false);
    assert_eq!(c.states[0].n_arcs, 1);
    assert_eq!(c.states[0].arcs[0].state, 1);
    assert_eq!(c.states[0].arcs[0].ilabel, 1);
    assert_eq!(c.states[0].arcs[0].olabel, 3);
    assert_eq!(c.states[0].arcs[0].weight, 3.0);
    // State 1: final, no arcs
    assert_eq!(c.states[1].final_state, true);
    assert_eq!(c.states[1].n_arcs, 0);
}

fn main() {}
