use fslib::fst::Fst;

#[test]
fn test_shortest_path() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t1\t1.0\n0\t2\t2\t2\t5.0\n1\t2\t3\t3\t1.0\n2\t0.0");
    let mut path = Fst::new();
    fst.shortest(&mut path);
    assert_eq!(path.n_states, 3);
    assert_eq!(path.start, 0);
    // State 0: not final, 1 arc
    assert_eq!(path.states[0].final_state, false);
    assert_eq!(path.states[0].n_arcs, 1);
    assert_eq!(path.states[0].arcs[0].state, 1);
    assert_eq!(path.states[0].arcs[0].ilabel, 1);
    assert_eq!(path.states[0].arcs[0].olabel, 1);
    assert_eq!(path.states[0].arcs[0].weight, 1.0);
    // State 1: not final, 1 arc
    assert_eq!(path.states[1].final_state, false);
    assert_eq!(path.states[1].n_arcs, 1);
    assert_eq!(path.states[1].arcs[0].state, 2);
    assert_eq!(path.states[1].arcs[0].ilabel, 3);
    assert_eq!(path.states[1].arcs[0].olabel, 3);
    assert_eq!(path.states[1].arcs[0].weight, 1.0);
    // State 2: final, no arcs
    assert_eq!(path.states[2].final_state, true);
    assert_eq!(path.states[2].n_arcs, 0);
}

fn main() {}
