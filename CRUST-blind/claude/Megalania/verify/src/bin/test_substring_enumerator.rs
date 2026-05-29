use std::cell::RefCell;
use Megalania::substring_enumerator::SubstringEnumerator;

#[test]
fn test_hello_hello() {
    // From running C: substring_enumerator_for_each on "hello hello"
    // pos 6: (0,2),(0,3),(0,4),(0,5)
    // pos 7: (1,2),(1,3),(1,4)
    // pos 8: (2,2),(2,3)
    // pos 9: (3,2)
    let data: &[u8] = b"hello hello";
    let e = SubstringEnumerator::new(data, 2, 273);

    let collect = |pos: usize| -> Vec<(usize, usize)> {
        let result = RefCell::new(Vec::<(usize, usize)>::new());
        e.for_each(pos, |o, l| result.borrow_mut().push((o, l)));
        result.into_inner()
    };

    for pos in 0..6 {
        assert_eq!(collect(pos), vec![]);
    }
    assert_eq!(collect(6), vec![(0, 2), (0, 3), (0, 4), (0, 5)]);
    assert_eq!(collect(7), vec![(1, 2), (1, 3), (1, 4)]);
    assert_eq!(collect(8), vec![(2, 2), (2, 3)]);
    assert_eq!(collect(9), vec![(3, 2)]);
    assert_eq!(collect(10), vec![]);
}

#[test]
fn test_hello_hello_max_substring_3() {
    // C output: when max_length=3:
    // pos 6: (0,2),(0,3)
    // pos 7: (1,2),(1,3)
    // pos 8: (2,2),(2,3)
    // pos 9: (3,2)
    let data: &[u8] = b"hello hello";
    let e = SubstringEnumerator::new(data, 2, 3);

    let collect = |pos: usize| -> Vec<(usize, usize)> {
        let result = RefCell::new(Vec::<(usize, usize)>::new());
        e.for_each(pos, |o, l| result.borrow_mut().push((o, l)));
        result.into_inner()
    };

    for pos in 0..6 {
        assert_eq!(collect(pos), vec![]);
    }
    assert_eq!(collect(6), vec![(0, 2), (0, 3)]);
    assert_eq!(collect(7), vec![(1, 2), (1, 3)]);
    assert_eq!(collect(8), vec![(2, 2), (2, 3)]);
    assert_eq!(collect(9), vec![(3, 2)]);
    assert_eq!(collect(10), vec![]);
}

#[test]
fn test_no_substrings() {
    // "aa bb cc" - no repeating bigrams
    let data: &[u8] = b"aa bb cc";
    let e = SubstringEnumerator::new(data, 2, 273);
    for pos in 0..data.len() {
        let result = RefCell::new(Vec::<(usize, usize)>::new());
        e.for_each(pos, |o, l| result.borrow_mut().push((o, l)));
        assert!(result.into_inner().is_empty(), "pos={} should have no substrings", pos);
    }
}

#[test]
fn test_memory_usage_grows_linearly() {
    let m0 = SubstringEnumerator::memory_usage(0);
    let m100 = SubstringEnumerator::memory_usage(100);
    let m200 = SubstringEnumerator::memory_usage(200);
    assert_eq!(m100 - m0, 100 * std::mem::size_of::<usize>());
    assert_eq!(m200 - m0, 200 * std::mem::size_of::<usize>());
}

fn main() {}
