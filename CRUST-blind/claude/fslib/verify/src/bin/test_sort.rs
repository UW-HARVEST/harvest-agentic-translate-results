use fslib::fst::{Fst, ISORT, OSORT};
use fslib::sort::fst_arc_sort;

#[test]
fn test_sort_input() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 1, 0.0);
    f.add_arc(0, 1, 3, 2, 0.0);
    f.add_arc(0, 1, 1, 3, 0.0);
    f.add_arc(0, 1, 2, 4, 0.0);
    fst_arc_sort(&mut f, false); // input sort
    assert_eq!(f.flags, ISORT);
    let arcs = &f.states[0].arcs;
    assert_eq!(arcs.len(), 4);
    assert_eq!(arcs[0].ilabel, 1);
    assert_eq!(arcs[0].olabel, 3);
    assert_eq!(arcs[1].ilabel, 2);
    assert_eq!(arcs[1].olabel, 4);
    assert_eq!(arcs[2].ilabel, 3);
    assert_eq!(arcs[2].olabel, 2);
    assert_eq!(arcs[3].ilabel, 5);
    assert_eq!(arcs[3].olabel, 1);
}

#[test]
fn test_sort_output() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 1, 0.0);
    f.add_arc(0, 1, 3, 2, 0.0);
    f.add_arc(0, 1, 1, 3, 0.0);
    f.add_arc(0, 1, 2, 4, 0.0);
    fst_arc_sort(&mut f, true); // output sort
    assert_eq!(f.flags, OSORT);
    let arcs = &f.states[0].arcs;
    assert_eq!(arcs[0].olabel, 1);
    assert_eq!(arcs[1].olabel, 2);
    assert_eq!(arcs[2].olabel, 3);
    assert_eq!(arcs[3].olabel, 4);
    assert_eq!(arcs[0].ilabel, 5);
    assert_eq!(arcs[1].ilabel, 3);
    assert_eq!(arcs[2].ilabel, 1);
    assert_eq!(arcs[3].ilabel, 2);
}

#[test]
fn test_sort_both_flags_set() {
    let mut f = Fst::new();
    f.add_state();
    f.add_arc(0, 0, 5, 1, 0.0);
    fst_arc_sort(&mut f, false);
    assert_eq!(f.flags, ISORT);
    fst_arc_sort(&mut f, true);
    assert_eq!(f.flags, ISORT | OSORT);
}

#[test]
fn test_sort_multiple_states() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.add_arc(0, 1, 3, 0, 0.0);
    f.add_arc(0, 1, 1, 0, 0.0);
    f.add_arc(0, 1, 2, 0, 0.0);
    f.add_arc(1, 2, 5, 0, 0.0);
    f.add_arc(1, 2, 4, 0, 0.0);
    fst_arc_sort(&mut f, false);
    let s0 = &f.states[0].arcs;
    assert_eq!(s0[0].ilabel, 1);
    assert_eq!(s0[1].ilabel, 2);
    assert_eq!(s0[2].ilabel, 3);
    let s1 = &f.states[1].arcs;
    assert_eq!(s1[0].ilabel, 4);
    assert_eq!(s1[1].ilabel, 5);
}

fn main() {}
