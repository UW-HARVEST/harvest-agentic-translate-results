use fslib::fst::Fst;
use fslib::trim;

#[test]
fn test_reverse() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t1\t1.0\n1\t2\t2\t2\t2.0\n2\t0.0");
    assert_eq!(fst.start, 0);
    assert_eq!(fst.states[0].final_state, false);
    assert_eq!(fst.states[2].final_state, true);

    fst.reverse();
    assert_eq!(fst.start, 2);
    assert_eq!(fst.states[0].final_state, true);
    assert_eq!(fst.states[2].final_state, false);
    assert_eq!(fst.states[0].n_arcs, 0);
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[2].n_arcs, 1);
    // Reversed arcs
    assert_eq!(fst.states[1].arcs[0].state, 0);
    assert_eq!(fst.states[1].arcs[0].ilabel, 1);
    assert_eq!(fst.states[1].arcs[0].olabel, 1);
    assert_eq!(fst.states[1].arcs[0].weight, 1.0);
    assert_eq!(fst.states[2].arcs[0].state, 1);
    assert_eq!(fst.states[2].arcs[0].ilabel, 2);
    assert_eq!(fst.states[2].arcs[0].olabel, 2);
    assert_eq!(fst.states[2].arcs[0].weight, 2.0);
}

#[test]
fn test_trim_removes_unreachable() {
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2
    fst.add_state(); // 3 - unreachable
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.set_final(2, 0.0);
    assert_eq!(fst.n_states, 4);
    fst.trim();
    assert_eq!(fst.n_states, 3);
}

#[test]
fn test_trim_no_finals() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 1.0);
    assert_eq!(fst.n_states, 2);
    fst.trim();
    assert_eq!(fst.n_states, 0);
}

fn main() {}
