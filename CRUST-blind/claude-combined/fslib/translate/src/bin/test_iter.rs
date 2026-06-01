use fslib::fst::Fst;
use fslib::iter::FstIter;

#[test]
fn test_iter_visit_order() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 0.0);
    fst.add_arc(1, 2, 2, 2, 0.0);
    fst.add_arc(2, 3, 3, 3, 0.0);
    fst.set_final(3, 0.0);

    let mut iter = FstIter::new(&fst);
    let mut visited = Vec::new();
    while let Some(s) = iter.next() {
        visited.push(s);
    }
    assert_eq!(visited, vec![0, 1, 2, 3]);
}

#[test]
fn test_iter_visited() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 0.0);
    let mut iter = FstIter::new(&fst);
    while iter.next().is_some() {}
    assert_eq!(iter.visited(0), true);
    assert_eq!(iter.visited(1), true);
}

fn main() {}
