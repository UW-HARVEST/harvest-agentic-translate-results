use fslib::fst::Fst;
use fslib::trim::{fst_trim, fst_rm_states, fst_close_states, fst_get_finals_states, fst_reverse};
use fslib::bitset::BitSet;
use fslib::queue::Queue;

#[test]
fn test_trim_unreachable_and_non_coaccessible() {
    let mut fst = Fst::new();
    for _ in 0..5 {
        fst.add_state();
    }
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.add_arc(0, 3, 3, 3, 1.0); // 3 has no path to a final
    // 4 is unreachable
    fst.set_final(2, 0.0);
    fst_trim(&mut fst);
    assert_eq!(fst.n_states, 3);
    assert_eq!(fst.start, 0);
    assert_eq!(fst.states[0].final_state, false);
    assert_eq!(fst.states[1].final_state, false);
    assert_eq!(fst.states[2].final_state, true);
    assert_eq!(fst.states[0].n_arcs, 1);
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[2].n_arcs, 0);
}

#[test]
fn test_rm_states() {
    let mut fst = Fst::new();
    for _ in 0..5 {
        fst.add_state();
    }
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.add_arc(2, 4, 3, 3, 1.0);
    fst.set_final(4, 0.0);
    let mut mask = BitSet::new(5);
    mask.set(3); // remove state 3
    fst_rm_states(&mut fst, &mask);
    // After removing state 3, original state 4 becomes new state 3
    assert_eq!(fst.n_states, 4);
    assert_eq!(fst.states[3].final_state, true);
    let arc = &fst.states[2].arcs[0];
    assert_eq!(arc.state, 3);
    assert_eq!(arc.ilabel, 3);
}

#[test]
fn test_get_finals_states() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.set_final(1, 0.5);
    fst.set_final(2, 1.0);
    let mut q: Queue<u32> = Queue::new();
    fst_get_finals_states(&fst, &mut q);
    assert_eq!(q.len(), 2);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
}

#[test]
fn test_close_multiple_finals() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.set_final(1, 0.5);
    fst.set_final(2, 1.0);
    let mut q: Queue<u32> = Queue::new();
    fst_get_finals_states(&fst, &mut q);
    fst_close_states(&mut fst, &mut q);
    // a new final state added; previous finals are no longer final
    assert_eq!(fst.n_states, 4);
    assert_eq!(fst.states[1].final_state, false);
    assert_eq!(fst.states[2].final_state, false);
    assert_eq!(fst.states[3].final_state, true);
}

#[test]
fn test_reverse_via_trim_module() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 0.5);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.set_final(2, 0.0);
    fst_reverse(&mut fst);
    assert_eq!(fst.start, 2);
    assert_eq!(fst.states[0].final_state, true);
}

fn main() {}
