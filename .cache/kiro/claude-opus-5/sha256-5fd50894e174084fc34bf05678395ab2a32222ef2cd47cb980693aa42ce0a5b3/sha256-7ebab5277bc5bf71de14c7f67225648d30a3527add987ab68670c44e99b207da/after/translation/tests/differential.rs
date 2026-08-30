//! Differential tests for `smallestValue`, the sole public entry point of
//! `include/simplestruct.h`.
//!
//! Every call goes through the exported dynamic symbol of the C `.so` and of
//! the Rust `cdylib`, so the `#[no_mangle]` wrapper is under test too.

mod common;

use common::{ListStorage, assert_same, load_pair};

/// Deterministic xorshift PRNG so fuzz cases are reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Value in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[test]
fn both_libraries_export_smallest_value() {
    // Loading succeeds only if both .so files expose the symbol.
    let pair = load_pair();
    assert_eq!(pair.c.name(), "C libSimpleList.so");
    assert_eq!(pair.rust.name(), "Rust libSimpleList.so");
}

#[test]
fn null_head_returns_minus_one() {
    let pair = load_pair();
    let mut empty = ListStorage::new(&[]);
    assert!(empty.head().is_null());
    assert_eq!(assert_same(&pair, &[]), -1);
}

#[test]
fn single_node_returns_its_value() {
    let pair = load_pair();
    for value in [
        0,
        1,
        -1, // collides with the NULL sentinel on purpose
        42,
        -42,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ] {
        assert_eq!(assert_same(&pair, &[value]), value, "single node {value}");
    }
}

#[test]
fn minimum_at_each_position() {
    let pair = load_pair();
    let len = 8;
    for min_index in 0..len {
        let values: Vec<i32> = (0..len)
            .map(|i| if i == min_index { -1000 } else { i as i32 })
            .collect();
        assert_eq!(assert_same(&pair, &values), -1000, "min at {min_index}");
    }
}

#[test]
fn ordered_and_reversed_sequences() {
    let pair = load_pair();
    let ascending: Vec<i32> = (0..64).collect();
    let descending: Vec<i32> = (0..64).rev().collect();
    assert_eq!(assert_same(&pair, &ascending), 0);
    assert_eq!(assert_same(&pair, &descending), 0);

    let neg_ascending: Vec<i32> = (-64..0).collect();
    let neg_descending: Vec<i32> = (-64..0).rev().collect();
    assert_eq!(assert_same(&pair, &neg_ascending), -64);
    assert_eq!(assert_same(&pair, &neg_descending), -64);
}

#[test]
fn duplicates_and_uniform_lists() {
    let pair = load_pair();
    assert_same(&pair, &[7, 7, 7, 7]);
    assert_same(&pair, &[0, 0, 0]);
    assert_same(&pair, &[-5, -5, 3, -5]);
    assert_same(&pair, &[i32::MIN, i32::MIN]);
    assert_same(&pair, &[i32::MAX, i32::MAX, i32::MAX]);
}

#[test]
fn extreme_values_mixed() {
    let pair = load_pair();
    // Cases that would break a naive translation using unsigned comparison
    // or saturating/wrapping arithmetic.
    assert_eq!(assert_same(&pair, &[i32::MAX, i32::MIN]), i32::MIN);
    assert_eq!(assert_same(&pair, &[i32::MIN, i32::MAX]), i32::MIN);
    assert_eq!(assert_same(&pair, &[0, i32::MIN, 0]), i32::MIN);
    assert_eq!(
        assert_same(&pair, &[-1, i32::MIN + 1, i32::MAX]),
        i32::MIN + 1
    );
    assert_eq!(assert_same(&pair, &[i32::MAX, 0, i32::MAX]), 0);
}

#[test]
fn lengths_one_through_sixtyfour() {
    let pair = load_pair();
    let mut rng = Rng::new(0xC0FFEE);
    for len in 1..=64 {
        let values: Vec<i32> = (0..len).map(|_| rng.next_i32() % 1000).collect();
        let expected = *values.iter().min().unwrap();
        assert_eq!(assert_same(&pair, &values), expected, "len {len}");
    }
}

#[test]
fn long_list() {
    let pair = load_pair();
    let mut values: Vec<i32> = (0..100_000).map(|i| 100_000 - i).collect();
    values[73_421] = -999_999;
    assert_eq!(assert_same(&pair, &values), -999_999);
}

#[test]
fn fuzz_full_i32_range() {
    let pair = load_pair();
    let mut rng = Rng::new(0x5EED_1234_ABCD);

    for _ in 0..2_000 {
        let len = rng.below(24) as usize; // includes 0 -> NULL head
        let values: Vec<i32> = (0..len)
            .map(|_| match rng.below(4) {
                // Bias towards boundary values, otherwise full range.
                0 => [i32::MIN, i32::MAX, 0, -1, 1][rng.below(5) as usize],
                _ => rng.next_i32(),
            })
            .collect();

        let expected = values.iter().min().copied().unwrap_or(-1);
        assert_eq!(assert_same(&pair, &values), expected, "fuzz {values:?}");
    }
}

#[test]
fn caller_list_is_not_mutated() {
    // The C function only reads; make sure the Rust version does too.
    let pair = load_pair();
    let values: Vec<i32> = vec![9, -3, 12, 0, 7];

    for imp in [&pair.c, &pair.rust] {
        let mut storage = ListStorage::new(&values);
        let head = storage.head();
        let result = unsafe { imp.smallest_value(head) };
        assert_eq!(result, -3, "{}", imp.name());

        // Walk the chain afterwards and confirm it is unchanged.
        let mut seen = Vec::new();
        let mut cur = head;
        while !cur.is_null() {
            unsafe {
                seen.push((*cur).value);
                cur = (*cur).next;
            }
        }
        assert_eq!(seen, values, "{} mutated the list", imp.name());
    }
}
