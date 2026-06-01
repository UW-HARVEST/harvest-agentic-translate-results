use Megalania::substring_enumerator::SubstringEnumerator;
use std::cell::RefCell;

#[test]
fn test_hello_hello() {
    let data = b"hello hello";
    let expected = [0, 0, 0, 0, 0, 0, 4, 3, 2, 1, 0];
    let em = SubstringEnumerator::new(data, 2, 273);
    for i in 0..data.len() {
        let counter = RefCell::new(0usize);
        em.for_each(i, |_off, _len| {
            *counter.borrow_mut() += 1;
        });
        assert_eq!(*counter.borrow(), expected[i], "pos {} mismatch", i);
    }
}

#[test]
fn test_hello_hello_max3() {
    let data = b"hello hello";
    let expected = [0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 0];
    let em = SubstringEnumerator::new(data, 2, 3);
    for i in 0..data.len() {
        let counter = RefCell::new(0usize);
        em.for_each(i, |_off, _len| {
            *counter.borrow_mut() += 1;
        });
        assert_eq!(*counter.borrow(), expected[i], "pos {} mismatch", i);
    }
}

#[test]
fn test_aa_bb_cc_no_match() {
    let data = b"aa bb cc";
    let em = SubstringEnumerator::new(data, 2, 273);
    for i in 0..data.len() {
        let counter = RefCell::new(0usize);
        em.for_each(i, |_off, _len| {
            *counter.borrow_mut() += 1;
        });
        assert_eq!(*counter.borrow(), 0, "pos {} should have no matches", i);
    }
}

#[test]
fn test_match_callbacks_correct_data() {
    let data = b"hello hello";
    let em = SubstringEnumerator::new(data, 2, 273);
    // At position 6 ('h'), we should match 'hello' from position 0
    // Returns (offset=0, length=2), (0, 3), (0, 4), (0, 5)
    let results: RefCell<Vec<(usize, usize)>> = RefCell::new(Vec::new());
    em.for_each(6, |off, len| {
        results.borrow_mut().push((off, len));
    });
    let r = results.borrow();
    assert_eq!(r.len(), 4);
    assert_eq!(r[0], (0, 2));
    assert_eq!(r[1], (0, 3));
    assert_eq!(r[2], (0, 4));
    assert_eq!(r[3], (0, 5));
}

#[test]
fn test_match_callback_data_consistency() {
    // Verify each (offset, length) callback yields a true substring match
    let data = b"hello hello";
    let em = SubstringEnumerator::new(data, 2, 273);
    for i in 0..data.len() {
        em.for_each(i, |off, len| {
            // Verify offset+len <= data.len()
            assert!(off + len <= data.len());
            // Verify i+len <= data.len()
            assert!(i + len <= data.len());
            // Verify data matches
            for k in 0..len {
                assert_eq!(data[i + k], data[off + k]);
            }
        });
    }
}

#[test]
fn test_pos_zero_no_callbacks() {
    let data = b"hello hello";
    let em = SubstringEnumerator::new(data, 2, 273);
    let counter = RefCell::new(0);
    em.for_each(0, |_, _| {
        *counter.borrow_mut() += 1;
    });
    assert_eq!(*counter.borrow(), 0);
}

#[test]
fn test_last_pos_no_callbacks() {
    let data = b"hello hello";
    let em = SubstringEnumerator::new(data, 2, 273);
    let counter = RefCell::new(0);
    em.for_each(data.len() - 1, |_, _| {
        *counter.borrow_mut() += 1;
    });
    assert_eq!(*counter.borrow(), 0);
}

fn main() {}
