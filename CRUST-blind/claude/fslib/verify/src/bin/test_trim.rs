use fslib::bitset::BitSet;
use fslib::fst::Fst;
use fslib::trim::{fst_reverse, fst_rm_states, fst_trim};

#[test]
fn test_reverse_basic() {
    // 0 -> 1 -> 2, 2 final
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 1.0);
    f.add_arc(1, 2, 2, 2, 2.0);
    f.set_final(2, 0.0);
    fst_reverse(&mut f);
    // Per probe:
    //   n_states=3 start=2
    //   state 0: final=1
    //   state 1: arc -> 0 il=1 ol=1 w=1
    //   state 2: arc -> 1 il=2 ol=2 w=2
    assert_eq!(f.n_states, 3);
    assert_eq!(f.start, 2);
    let s0 = &f.states[0];
    assert_eq!(s0.final_state, true);
    assert_eq!(s0.n_arcs, 0);
    let s1 = &f.states[1];
    assert_eq!(s1.n_arcs, 1);
    assert_eq!(s1.arcs[0].state, 0);
    assert_eq!(s1.arcs[0].ilabel, 1);
    assert_eq!(s1.arcs[0].olabel, 1);
    assert_eq!(s1.arcs[0].weight, 1.0);
    let s2 = &f.states[2];
    assert_eq!(s2.n_arcs, 1);
    assert_eq!(s2.arcs[0].state, 1);
    assert_eq!(s2.arcs[0].ilabel, 2);
    assert_eq!(s2.arcs[0].olabel, 2);
    assert_eq!(s2.arcs[0].weight, 2.0);
    assert_eq!(s2.final_state, false);
}

#[test]
fn test_trim_basic() {
    // 0 -> 1 -> 2 (final), 0 -> 4 (reachable but no path to final), 3 unreachable
    let mut f = Fst::new();
    for _ in 0..5 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 1.0);
    f.add_arc(1, 2, 2, 2, 1.0);
    f.add_arc(0, 4, 5, 5, 1.0);
    f.set_final(2, 0.0);
    fst_trim(&mut f);
    // Per probe: n_states=3 start=0
    // state 0: arc -> 1 il=1 ol=1 w=1
    // state 1: arc -> 2 il=2 ol=2 w=1
    // state 2: final
    assert_eq!(f.n_states, 3);
    assert_eq!(f.start, 0);
    let s0 = &f.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 1);
    let s1 = &f.states[1];
    assert_eq!(s1.n_arcs, 1);
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 2);
    assert_eq!(s1.arcs[0].olabel, 2);
    let s2 = &f.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.n_arcs, 0);
}

#[test]
fn test_trim_no_finals_makes_empty() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(1, 2, 2, 2, 0.0);
    fst_trim(&mut f);
    assert_eq!(f.n_states, 0);
}

#[test]
fn test_trim_multiple_finals() {
    // Per C/probe: trim with multiple finals creates a new "single final" state
    // and connects the original finals to it via eps arcs.
    let mut f = Fst::new();
    for _ in 0..4 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(0, 2, 2, 2, 0.0);
    f.add_arc(0, 3, 3, 3, 0.0);
    f.set_final(1, 0.5);
    f.set_final(2, 0.7);
    f.set_final(3, 0.9);
    fst_trim(&mut f);
    // From C probe: n_states=5 start=0
    //   state 0: arcs to 1,2,3 with il=1,2,3
    //   state 1,2,3: -> 4 with eps:eps and respective weights 0.5, 0.7, 0.9
    //   state 4: final
    assert_eq!(f.n_states, 5);
    assert_eq!(f.start, 0);
    let s0 = &f.states[0];
    assert_eq!(s0.n_arcs, 3);
    let s1 = &f.states[1];
    assert_eq!(s1.n_arcs, 1);
    assert_eq!(s1.arcs[0].state, 4);
    assert_eq!(s1.arcs[0].ilabel, 0);
    assert_eq!(s1.arcs[0].olabel, 0);
    assert_eq!(s1.arcs[0].weight, 0.5);
    assert_eq!(s1.final_state, false);
    let s2 = &f.states[2];
    assert_eq!(s2.arcs[0].state, 4);
    assert_eq!(s2.arcs[0].weight, 0.7);
    let s3 = &f.states[3];
    assert_eq!(s3.arcs[0].state, 4);
    assert_eq!(s3.arcs[0].weight, 0.9);
    let s4 = &f.states[4];
    assert_eq!(s4.final_state, true);
    assert_eq!(s4.n_arcs, 0);
}

#[test]
fn test_rm_states_basic() {
    // Remove state 2 from a 4-state FST; mask: 0=keep, 1=remove
    let mut f = Fst::new();
    for _ in 0..4 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(1, 2, 2, 2, 0.0);
    f.add_arc(0, 3, 3, 3, 0.0);
    f.set_final(3, 0.0);
    let mut mask = BitSet::new(4);
    mask.set(2); // remove state 2
    fst_rm_states(&mut f, &mask);
    // Now state 2 is removed; remaining states 0, 1, 3 (renumbered to 0, 1, 2)
    assert_eq!(f.n_states, 3);
    let s0 = &f.states[0];
    // arcs from state 0:
    //   - to state 1 (kept): valid arc
    //   - to state 3 (-> 2 after renumber)
    assert_eq!(s0.n_arcs, 2);
    let s1 = &f.states[1];
    // arc 1->2 was to removed state 2 -> dropped
    assert_eq!(s1.n_arcs, 0);
    let s2 = &f.states[2]; // renumbered from old state 3
    assert_eq!(s2.final_state, true);
}

fn main() {}
