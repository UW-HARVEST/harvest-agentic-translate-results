use fslib::fst::Fst;

#[test]
fn test_create() {
    let fst = Fst::new();
    assert_eq!(fst.n_states, 0);
    assert_eq!(fst.start, 0);
    assert_eq!(fst.sr_type, 0);
}

#[test]
fn test_add_state() {
    let mut fst = Fst::new();
    let s0 = fst.add_state();
    let s1 = fst.add_state();
    let s2 = fst.add_state();
    assert_eq!(s0, 0);
    assert_eq!(s1, 1);
    assert_eq!(s2, 2);
    assert_eq!(fst.n_states, 3);
}

#[test]
fn test_add_arc() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    let a0 = fst.add_arc(0, 1, 1, 2, 0.5);
    let a1 = fst.add_arc(0, 2, 3, 4, 1.5);
    let a2 = fst.add_arc(1, 2, 5, 6, 2.0);
    assert_eq!(a0, 0);
    assert_eq!(a1, 1);
    assert_eq!(a2, 0);
    assert_eq!(fst.states[0].n_arcs, 2);
    assert_eq!(fst.states[1].n_arcs, 1);
    // Check arc data
    assert_eq!(fst.states[0].arcs[0].state, 1);
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[0].olabel, 2);
    assert_eq!(fst.states[0].arcs[0].weight, 0.5);
    assert_eq!(fst.states[0].arcs[1].state, 2);
    assert_eq!(fst.states[0].arcs[1].ilabel, 3);
    assert_eq!(fst.states[0].arcs[1].olabel, 4);
    assert_eq!(fst.states[0].arcs[1].weight, 1.5);
}

#[test]
fn test_set_final() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.set_final(2, 3.0);
    assert_eq!(fst.states[2].final_state, true);
    assert_eq!(fst.states[2].weight, 3.0);
}

#[test]
fn test_get_n_arcs() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(0, 2, 3, 4, 1.5);
    fst.add_arc(1, 2, 5, 6, 2.0);
    assert_eq!(fst.get_n_arcs(), 3);
}

#[test]
fn test_copy() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    let mut copy = Fst::new();
    fst.copy(&mut copy);
    assert_eq!(copy.n_states, 2);
    assert_eq!(copy.start, 0);
    assert_eq!(copy.states[0].arcs[0].state, 1);
}

#[test]
fn test_relabel_input() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.relabel(1, 100, 0);
    assert_eq!(fst.states[0].arcs[0].ilabel, 100);
    assert_eq!(fst.states[0].arcs[0].olabel, 2);
}

#[test]
fn test_relabel_output() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.relabel(2, 200, 1);
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[0].olabel, 200);
}

#[test]
fn test_empty() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.empty();
    assert_eq!(fst.n_states, 0);
    assert_eq!(fst.states.len(), 0);
    assert_eq!(fst.start, 0);
}

#[test]
fn test_stack() {
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 1, 1.0);
    a.set_final(1, 0.0);

    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 2, 2.0);
    b.set_final(1, 0.0);

    a.stack(&b);
    assert_eq!(a.n_states, 4);
    assert_eq!(a.states[2].arcs[0].state, 3);
    assert_eq!(a.states[3].final_state, true);
}

fn main() {}
