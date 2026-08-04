use fslib::fst::Fst;
use fslib::iter::FstIter;

#[test]
fn test_iter_basic_traverse() {
    let mut f = Fst::new();
    for _ in 0..5 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(0, 2, 2, 2, 0.0);
    f.add_arc(1, 3, 3, 3, 0.0);
    f.add_arc(2, 3, 4, 4, 0.0);
    // 4 unreachable
    let mut iter = FstIter::new(&f);
    let mut visited: Vec<u32> = Vec::new();
    while let Some(s) = iter.next() {
        visited.push(s);
    }
    assert_eq!(visited, vec![0, 1, 2, 3]);
    assert_eq!(iter.visited(0), true);
    assert_eq!(iter.visited(1), true);
    assert_eq!(iter.visited(2), true);
    assert_eq!(iter.visited(3), true);
    assert_eq!(iter.visited(4), false);
}

#[test]
fn test_iter_isolated_start() {
    // start state has no outgoing arcs
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 0;
    let mut iter = FstIter::new(&f);
    let mut visited: Vec<u32> = Vec::new();
    while let Some(s) = iter.next() {
        visited.push(s);
    }
    assert_eq!(visited, vec![0]);
    assert_eq!(iter.visited(0), true);
    assert_eq!(iter.visited(1), false);
    assert_eq!(iter.visited(2), false);
}

#[test]
fn test_iter_state_after_done() {
    let mut f = Fst::new();
    f.add_state();
    let mut iter = FstIter::new(&f);
    let _ = iter.next();
    let result = iter.next();
    assert!(result.is_none());
    assert_eq!(iter.state, u32::MAX);
}

#[test]
fn test_iter_visit_starts_with_start() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 2;
    f.add_arc(2, 0, 1, 1, 0.0);
    let mut iter = FstIter::new(&f);
    let s0 = iter.next();
    assert_eq!(s0, Some(2));
    let s1 = iter.next();
    assert_eq!(s1, Some(0));
}

#[test]
fn test_iter_no_revisit() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 0;
    // Cycle: 0 -> 1 -> 2 -> 0
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(1, 2, 2, 2, 0.0);
    f.add_arc(2, 0, 3, 3, 0.0);
    let mut iter = FstIter::new(&f);
    let mut visited: Vec<u32> = Vec::new();
    while let Some(s) = iter.next() {
        visited.push(s);
    }
    assert_eq!(visited, vec![0, 1, 2]);
}

fn main() {}
