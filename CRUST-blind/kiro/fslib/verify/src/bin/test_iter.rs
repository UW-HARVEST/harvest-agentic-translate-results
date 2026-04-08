use fslib::fst::Fst;
use fslib::iter::FstIter;

#[test]
fn test_iter_traversal() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t1\t1.0\n1\t2\t2\t2\t1.0\n2\t3\t3\t3\t1.0\n3\t0.0");
    let mut iter = FstIter::new(&fst);
    let mut states = Vec::new();
    while let Some(s) = iter.next() {
        states.push(s);
    }
    assert_eq!(states, vec![0, 1, 2, 3]);
}

#[test]
fn test_iter_visited() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t1\t1.0\n1\t2\t2\t2\t1.0\n2\t3\t3\t3\t1.0\n3\t0.0");
    let mut iter = FstIter::new(&fst);
    while iter.next().is_some() {}
    assert_eq!(iter.visited(0), true);
    assert_eq!(iter.visited(3), true);
}

fn main() {}
