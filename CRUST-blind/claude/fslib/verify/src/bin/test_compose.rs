// Tests for FST composition via fst.compose() (which uses matcher internally)

use fslib::fst::{ArcData, Fst, ISORT, OSORT};

#[test]
fn test_compose_simple() {
    // A: 0 -> 1 (a:b/1.0), 1 final
    // B: 0 -> 1 (b:c/2.0), 1 final
    // Expected: composed FST with state 0 -> state 1 (a:c/3.0), state 1 final
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 2, 1.0);
    a.set_final(1, 0.0);
    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 3, 2.0);
    b.set_final(1, 0.0);
    let mut c = Fst::new();
    a.compose(&b, &mut c);
    assert_eq!(c.n_states, 2);
    assert_eq!(c.start, 0);
    let s0 = &c.states[0];
    assert_eq!(s0.n_arcs, 1);
    let arc: &ArcData = &s0.arcs[0];
    assert_eq!(arc.state, 1);
    assert_eq!(arc.ilabel, 1);
    assert_eq!(arc.olabel, 3);
    assert_eq!(arc.weight, 3.0); // tropical product = a + b = 1 + 2 = 3
    assert_eq!(c.states[1].final_state, true);
    assert_eq!(c.states[1].n_arcs, 0);
}

#[test]
fn test_compose_no_match() {
    // A: 0 -> 1 (a:5), 1 final
    // B: 0 -> 1 (7:8), 1 final - no match: A's olabel=5, B's ilabel=7
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 5, 0.0);
    a.set_final(1, 0.0);
    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 7, 8, 0.0);
    b.set_final(1, 0.0);
    let mut c = Fst::new();
    a.compose(&b, &mut c);
    // Only the start pair is materialized
    assert_eq!(c.n_states, 1);
    assert_eq!(c.start, 0);
    assert_eq!(c.states[0].n_arcs, 0);
    // (0,0) is not final because state 0 in either is not final
    assert_eq!(c.states[0].final_state, false);
}

#[test]
fn test_compose_with_eps() {
    // A: 0 -> 1 (a:eps), 1 -> 2 (b:c), 2 final
    // B: 0 -> 1 (eps:x), 1 -> 2 (c:d), 2 final
    let mut a = Fst::new();
    for _ in 0..3 {
        a.add_state();
    }
    a.add_arc(0, 1, 1, 0, 0.0); // a:eps
    a.add_arc(1, 2, 2, 3, 0.0); // b:c
    a.set_final(2, 0.0);
    let mut b = Fst::new();
    for _ in 0..3 {
        b.add_state();
    }
    b.add_arc(0, 1, 0, 5, 0.0); // eps:x
    b.add_arc(1, 2, 3, 4, 0.0); // c:d
    b.set_final(2, 0.0);
    let mut c = Fst::new();
    a.compose(&b, &mut c);
    // From probe: n_states=5
    assert_eq!(c.n_states, 5);
    // The final state (2,2) should be final
    let mut found_final = false;
    for s in 0..c.n_states {
        if c.states[s as usize].final_state {
            found_final = true;
        }
    }
    assert!(found_final);
}

#[test]
fn test_compose_with_isort_osort() {
    let mut a = Fst::new();
    for _ in 0..3 {
        a.add_state();
    }
    a.add_arc(0, 1, 1, 2, 0.0);
    a.add_arc(0, 1, 1, 3, 0.0);
    a.add_arc(1, 2, 2, 4, 0.0);
    a.set_final(2, 0.0);
    let mut b = Fst::new();
    for _ in 0..3 {
        b.add_state();
    }
    b.add_arc(0, 1, 2, 5, 0.0);
    b.add_arc(0, 1, 3, 6, 0.0);
    b.add_arc(1, 2, 4, 7, 0.0);
    b.set_final(2, 0.0);
    a.arc_sort(1); // OSORT
    b.arc_sort(0); // ISORT
    assert_eq!(a.flags & OSORT, OSORT);
    assert_eq!(b.flags & ISORT, ISORT);
    let mut c = Fst::new();
    a.compose(&b, &mut c);
    // From probe: n_states=3 with arcs: (0,a:5), (0,a:6), (1,b:7)
    assert_eq!(c.n_states, 3);
    assert_eq!(c.start, 0);
    let s0 = &c.states[0];
    assert_eq!(s0.n_arcs, 2);
    let mut s0_olabels: Vec<u32> = s0.arcs.iter().map(|a| a.olabel).collect();
    s0_olabels.sort();
    assert_eq!(s0_olabels, vec![5, 6]);
    for arc in &s0.arcs {
        assert_eq!(arc.ilabel, 1);
        assert_eq!(arc.state, 1);
    }
    // state 1 -> state 2 with (b:z)
    let s1 = &c.states[1];
    assert_eq!(s1.n_arcs, 1);
    let arc = &s1.arcs[0];
    assert_eq!(arc.ilabel, 2);
    assert_eq!(arc.olabel, 7);
    assert_eq!(arc.state, 2);
    assert_eq!(c.states[2].final_state, true);
}

#[test]
fn test_match_arcs_via_compose() {
    // Verify match_arcs is exercised correctly
    use fslib::fst::{match_arcs, Spair};
    use fslib::queue::Queue;
    use fslib::sr::sr_get;
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 2, 0.0);
    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 3, 0.0);
    let sr = sr_get(0); // tropical
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    let pair = Spair { a: 0, b: 0 };
    match_arcs(&a, &b, &pair, &sr, &mut q);
    // expected: epsilon-loop pair (i=0,j=0) skipped, (1,1)->2 ; b's (1,1)->2: olabel(2) == ilabel(2) -> match
    let mut count = 0;
    while let Some(_) = q.dequeue() {
        count += 1;
    }
    assert!(count >= 1);
}

fn main() {}
