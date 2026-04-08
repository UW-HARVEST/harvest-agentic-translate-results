use fslib::fst::Fst;
use fslib::iter::FstIter;

#[test]
fn test_iter_visits_all_reachable() {
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 1.0);

    let mut iter = FstIter::create(&fst);
    let mut visited = vec![];
    loop {
        let s = iter.next_state();
        if s == u32::MAX { break; }
        visited.push(s);
    }
    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&0));
    assert!(visited.contains(&1));
    assert!(visited.contains(&2));
}

#[test]
fn test_iter_unreachable_not_visited() {
    let mut fst = Fst::new();
    fst.add_state(); // 0
    fst.add_state(); // 1
    fst.add_state(); // 2 - unreachable
    fst.add_arc(0, 1, 1, 1, 1.0);

    let mut iter = FstIter::create(&fst);
    loop {
        let s = iter.next_state();
        if s == u32::MAX { break; }
    }
    assert!(iter.is_visited(0));
    assert!(iter.is_visited(1));
    assert!(!iter.is_visited(2));
}

#[test]
fn test_iter_single_state() {
    let mut fst = Fst::new();
    fst.add_state(); // 0

    let mut iter = FstIter::create(&fst);
    let s = iter.next_state();
    assert_eq!(s, 0);
    let s2 = iter.next_state();
    assert_eq!(s2, u32::MAX);
}

fn main() {}
