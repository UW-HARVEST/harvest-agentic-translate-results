use fslib::fst::Fst;
use fslib::sr;

#[test]
fn test_new() {
    let fst = Fst::new();
    assert_eq!(fst.n_states, 0);
    assert_eq!(fst.start, 0);
    assert_eq!(fst.flags, 0);
    assert_eq!(fst.sr_type, 0); // SR_TROPICAL
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
    fst.add_arc(0, 1, 1, 2, 0.5);
    assert_eq!(fst.states[0].n_arcs, 1);
    assert_eq!(fst.states[0].arcs[0].state, 1);
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[0].olabel, 2);
    assert_eq!(fst.states[0].arcs[0].weight, 0.5);
}

#[test]
fn test_set_final() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.set_final(2, 2.0);
    assert!(fst.states[2].final_state);
    assert_eq!(fst.states[2].weight, 2.0);
    assert!(!fst.states[0].final_state);
}

#[test]
fn test_get_n_arcs() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(1, 2, 3, 4, 1.5);
    assert_eq!(fst.get_n_arcs(), 2);
}

#[test]
fn test_empty() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.empty();
    assert_eq!(fst.n_states, 0);
    assert_eq!(fst.start, 0);
    assert!(fst.states.is_empty());
}

#[test]
fn test_copy() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(1, 2, 3, 4, 1.5);
    fst.set_final(2, 2.0);

    let mut copy = Fst::new();
    fst.copy(&mut copy);
    assert_eq!(copy.n_states, 3);
    assert_eq!(copy.start, 0);
    assert!(copy.states[2].final_state);
    assert_eq!(copy.get_n_arcs(), 2);
    assert_eq!(copy.states[0].arcs[0].state, 1);
    assert_eq!(copy.states[0].arcs[0].ilabel, 1);
    assert_eq!(copy.states[0].arcs[0].olabel, 2);
    assert_eq!(copy.states[0].arcs[0].weight, 0.5);
}

#[test]
fn test_relabel_input() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 6, 1.0);
    fst.add_arc(0, 1, 7, 5, 2.0);
    fst.relabel(5, 99, 0); // input labels: 5->99
    assert_eq!(fst.states[0].arcs[0].ilabel, 99);
    assert_eq!(fst.states[0].arcs[1].ilabel, 7); // unchanged
    assert_eq!(fst.states[0].arcs[0].olabel, 6); // unchanged
}

#[test]
fn test_relabel_output() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 6, 1.0);
    fst.add_arc(0, 1, 7, 5, 2.0);
    fst.relabel(5, 88, 1); // output labels: 5->88
    assert_eq!(fst.states[0].arcs[0].olabel, 6); // unchanged (was 6)
    assert_eq!(fst.states[0].arcs[1].olabel, 88); // was 5, now 88
}

#[test]
fn test_arc_sort_ilabel() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 3, 1.0);
    fst.add_arc(0, 2, 2, 7, 2.0);
    fst.add_arc(0, 1, 8, 1, 3.0);
    fst.arc_sort(0); // sort by ilabel
    assert_eq!(fst.states[0].arcs[0].ilabel, 2);
    assert_eq!(fst.states[0].arcs[1].ilabel, 5);
    assert_eq!(fst.states[0].arcs[2].ilabel, 8);
    assert_eq!(fst.flags & 0x01, 0x01); // ISORT flag set
}

#[test]
fn test_arc_sort_olabel() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 3, 1.0);
    fst.add_arc(0, 2, 2, 7, 2.0);
    fst.add_arc(0, 1, 8, 1, 3.0);
    fst.arc_sort(1); // sort by olabel
    assert_eq!(fst.states[0].arcs[0].olabel, 1);
    assert_eq!(fst.states[0].arcs[1].olabel, 3);
    assert_eq!(fst.states[0].arcs[2].olabel, 7);
    assert_eq!(fst.flags & 0x02, 0x02); // OSORT flag set
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
    assert_eq!(a.get_n_arcs(), 2);
    // b's arc 0->1 becomes 2->3 after offset
    assert_eq!(a.states[2].arcs[0].state, 3);
    assert_eq!(a.states[2].arcs[0].ilabel, 2);
}

#[test]
fn test_reverse() {
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 2.0);
    fst.set_final(2, 0.0);

    assert_eq!(fst.start, 0);
    fst.reverse();
    // After reverse: start becomes 2 (was final), state 0 becomes final
    assert_eq!(fst.start, 2);
    assert!(fst.states[0].final_state);
    assert!(!fst.states[2].final_state);
    // Arc 0->1 reversed to 1->0
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[1].arcs[0].state, 0);
    // Arc 1->2 reversed to 2->1
    assert_eq!(fst.states[2].n_arcs, 1);
    assert_eq!(fst.states[2].arcs[0].state, 1);
}

#[test]
fn test_compose() {
    // fst_a: 0->1 il=1 ol=2 w=1.0, state 1 final
    let mut fst_a = Fst::new();
    fst_a.add_state();
    fst_a.add_state();
    fst_a.add_arc(0, 1, 1, 2, 1.0);
    fst_a.set_final(1, 0.0);

    // fst_b: 0->1 il=2 ol=3 w=2.0, state 1 final
    let mut fst_b = Fst::new();
    fst_b.add_state();
    fst_b.add_state();
    fst_b.add_arc(0, 1, 2, 3, 2.0);
    fst_b.set_final(1, 0.0);

    let mut fst_c = Fst::new();
    fst_a.compose(&fst_b, &mut fst_c);

    assert_eq!(fst_c.n_states, 2);
    assert_eq!(fst_c.get_n_arcs(), 1);
    // Composed arc: il=1, ol=3, w=3.0 (1.0+2.0 tropical product)
    assert_eq!(fst_c.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst_c.states[0].arcs[0].olabel, 3);
    assert_eq!(fst_c.states[0].arcs[0].weight, 3.0);
    // State 1 should be final
    assert!(fst_c.states[1].final_state);
}

#[test]
fn test_shortest_path() {
    // 0->1 (w=1), 0->2 (w=5), 1->2 (w=1), 2 final
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(0, 2, 2, 2, 5.0);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.set_final(2, 0.0);

    let mut path = Fst::new();
    let result = fst.shortest(&mut path);

    // Shortest path: 0->1->2, total weight 2.0
    assert_eq!(result.n_states, 3);
    assert_eq!(result.get_n_arcs(), 2);
    // Path: state 0 -> state 1 (w=1), state 1 -> state 2 (w=1)
    assert_eq!(result.states[0].arcs[0].state, 1);
    assert_eq!(result.states[0].arcs[0].weight, 1.0);
    assert_eq!(result.states[1].arcs[0].state, 2);
    assert_eq!(result.states[1].arcs[0].weight, 1.0);
    assert!(result.states[2].final_state);
}

#[test]
fn test_trim() {
    // 4 states: 0->1->3 (final), state 2 unreachable
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2 - unreachable
    fst.add_state(); // 3
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 3, 2, 2, 1.0);
    fst.set_final(3, 0.0);

    let result = fst.trim();
    // After trim, unreachable state 2 removed -> 3 states
    assert_eq!(result.n_states, 3);
    assert_eq!(result.get_n_arcs(), 2);
}

fn main() {}
