use fslib::fst::{Fst, ArcData};
use fslib::queue::Queue;
use fslib::matcher;

#[test]
fn test_match_unsorted() {
    let a = vec![
        ArcData { state: 1, ilabel: 0, olabel: 2, weight: 1.0 },
        ArcData { state: 2, ilabel: 0, olabel: 3, weight: 2.0 },
    ];
    let b = vec![
        ArcData { state: 1, ilabel: 2, olabel: 0, weight: 1.0 },
        ArcData { state: 2, ilabel: 3, olabel: 0, weight: 2.0 },
    ];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    matcher::match_unsorted(&a, &b, &mut q);
    let mut results = Vec::new();
    while let Some((arc_a, arc_b)) = q.dequeue() {
        results.push((arc_a.olabel, arc_b.ilabel));
    }
    // a[0].olabel=2 matches b[0].ilabel=2, a[1].olabel=3 matches b[1].ilabel=3
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], (2, 2));
    assert_eq!(results[1], (3, 3));
}

#[test]
fn test_match_full_sorted() {
    let a = vec![
        ArcData { state: 1, ilabel: 0, olabel: 1, weight: 1.0 },
        ArcData { state: 2, ilabel: 0, olabel: 2, weight: 2.0 },
        ArcData { state: 3, ilabel: 0, olabel: 3, weight: 3.0 },
    ];
    let b = vec![
        ArcData { state: 1, ilabel: 2, olabel: 0, weight: 1.0 },
        ArcData { state: 2, ilabel: 3, olabel: 0, weight: 2.0 },
    ];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    matcher::match_full_sorted(&a, &b, &mut q);
    let mut results = Vec::new();
    while let Some((arc_a, arc_b)) = q.dequeue() {
        results.push((arc_a.olabel, arc_b.ilabel));
    }
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], (2, 2));
    assert_eq!(results[1], (3, 3));
}

fn main() {}
