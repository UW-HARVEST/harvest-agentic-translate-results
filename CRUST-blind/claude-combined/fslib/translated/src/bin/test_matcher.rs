use fslib::fst::ArcData;
use fslib::queue::Queue;
use fslib::matcher::{match_unsorted, match_full_sorted, match_half_sorted, match_half_sorted_rev};

fn arc(state: u32, ilabel: u32, olabel: u32) -> ArcData {
    ArcData {
        state,
        weight: 0.0,
        ilabel,
        olabel,
    }
}

#[test]
fn test_match_unsorted() {
    // C semantics: index 0 is the eps loop sentinel, then arcs follow.
    // a olabels: [0(eps), 5, 6]; b ilabels: [0(eps), 5, 6]
    let a = vec![arc(0, 0, 0), arc(2, 1, 5), arc(3, 2, 6)];
    let b = vec![arc(0, 0, 0), arc(5, 5, 7), arc(6, 6, 8)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_unsorted(&a, &b, &mut q);
    assert_eq!(q.len(), 2);
    let m1 = q.dequeue().unwrap();
    assert_eq!(m1.0.olabel, 5);
    assert_eq!(m1.1.ilabel, 5);
    let m2 = q.dequeue().unwrap();
    assert_eq!(m2.0.olabel, 6);
    assert_eq!(m2.1.ilabel, 6);
}

#[test]
fn test_match_full_sorted() {
    let a = vec![arc(0, 0, 0), arc(2, 1, 5), arc(3, 2, 6)];
    let b = vec![arc(0, 0, 0), arc(5, 5, 7), arc(6, 6, 8)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_full_sorted(&a, &b, &mut q);
    assert_eq!(q.len(), 2);
    let m1 = q.dequeue().unwrap();
    assert_eq!(m1.0.olabel, 5);
    assert_eq!(m1.1.ilabel, 5);
}

#[test]
fn test_match_half_sorted() {
    let a = vec![arc(0, 0, 0), arc(2, 1, 5), arc(3, 2, 6)];
    let b = vec![arc(0, 0, 0), arc(5, 5, 7), arc(6, 6, 8)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted(&a, &b, &mut q);
    assert_eq!(q.len(), 2);
}

#[test]
fn test_match_half_sorted_rev() {
    let a = vec![arc(0, 0, 0), arc(2, 1, 5), arc(3, 2, 6)];
    let b = vec![arc(0, 0, 0), arc(5, 5, 7), arc(6, 6, 8)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted_rev(&a, &b, &mut q);
    assert_eq!(q.len(), 2);
}

fn main() {}
