// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md (C1..C24). Every row calls BOTH the C `.so` and
// the Rust `.so` through their exported `smallestValue` symbol on identical node
// chains and asserts the returned `int`s are bit-identical. Randomized rows use a
// fixed seed so failures reproduce exactly.

mod common;

use common::{assert_same, reference_min, Both, List, Rng};

// ---------------------------------------------------------------- C1: length 0

#[test]
fn c1_length_zero_null_head() {
    let both = Both::load();
    // The only way to express "zero nodes": a NULL head.
    let v = assert_same(&both, &[], "C1 empty/NULL");
    assert_eq!(v, -1, "C1: C returns -1 for NULL head");
}

// ---------------------------------------------------------------- C2/C3: len 1

#[test]
fn c2_length_one_random_full_range() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0002);
    for i in 0..2000 {
        let v = rng.i32_any();
        let got = assert_same(&both, &[v], &format!("C2 iter {i}"));
        assert_eq!(got, v, "C2: single node must return its own value");
    }
}

#[test]
fn c3_length_one_boundary_values() {
    let both = Both::load();
    for &v in &[i32::MIN, -1, 0, 1, i32::MAX] {
        let got = assert_same(&both, &[v], &format!("C3 value {v}"));
        assert_eq!(got, v);
    }
}

// ------------------------------------------------------------------ C4..C7: len 2

#[test]
fn c4_length_two_random() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0004);
    for i in 0..2000 {
        let a = rng.i32_any();
        let b = rng.i32_any();
        let got = assert_same(&both, &[a, b], &format!("C4 iter {i}"));
        assert_eq!(got, reference_min(&[a, b]), "C4 iter {i}");
    }
}

#[test]
fn c5_length_two_min_at_first() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0005);
    for i in 0..500 {
        // Force a < b so the strict `<` never fires.
        let a = rng.i32_in(i32::MIN, i32::MAX - 1);
        let b = rng.i32_in(a.saturating_add(1), i32::MAX);
        let got = assert_same(&both, &[a, b], &format!("C5 iter {i}"));
        assert_eq!(got, a, "C5: seed value wins");
    }
}

#[test]
fn c6_length_two_min_at_second() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0006);
    for i in 0..500 {
        // Force b < a so the strict `<` fires exactly once.
        let a = rng.i32_in(i32::MIN + 1, i32::MAX);
        let b = rng.i32_in(i32::MIN, a - 1);
        let got = assert_same(&both, &[a, b], &format!("C6 iter {i}"));
        assert_eq!(got, b, "C6: second value wins");
    }
}

#[test]
fn c7_length_two_equal_values_tie() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0007);
    for i in 0..500 {
        let a = rng.i32_any();
        // Tie: strict `<` must NOT fire; the earlier value is kept.
        let got = assert_same(&both, &[a, a], &format!("C7 iter {i}"));
        assert_eq!(got, a);
    }
}

// ------------------------------------------------------------------- C8/C9: len 3, 4..16

#[test]
fn c8_length_three_random() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0008);
    for i in 0..2000 {
        let vals = [rng.i32_any(), rng.i32_any(), rng.i32_any()];
        let got = assert_same(&both, &vals, &format!("C8 iter {i}"));
        assert_eq!(got, reference_min(&vals), "C8 iter {i}");
    }
}

#[test]
fn c9_small_lengths_random() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0009);
    for i in 0..3000 {
        let n = rng.usize_in(4, 16);
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        let got = assert_same(&both, &vals, &format!("C9 iter {i}"));
        assert_eq!(got, reference_min(&vals), "C9 iter {i}");
    }
}

// --------------------------------------------- C10..C12: position of the minimum

/// Builds a list of length `n` from `filler` values all `> floor`, then plants
/// `target` (the unique minimum) at index `pos`.
fn with_min_at(rng: &mut Rng, n: usize, pos: usize) -> (Vec<i32>, i32) {
    // Fillers strictly greater than 0; target strictly less than all of them.
    let mut vals: Vec<i32> = (0..n).map(|_| rng.i32_in(1, i32::MAX)).collect();
    let target = rng.i32_in(i32::MIN, -1);
    vals[pos] = target;
    (vals, target)
}

#[test]
fn c10_min_at_first_position() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0010);
    for i in 0..1000 {
        let n = rng.usize_in(4, 16);
        let (vals, target) = with_min_at(&mut rng, n, 0);
        let got = assert_same(&both, &vals, &format!("C10 iter {i}"));
        assert_eq!(got, target, "C10: minimum is the seed");
    }
}

#[test]
fn c11_min_at_middle_position() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0011);
    for i in 0..1000 {
        let n = rng.usize_in(4, 16);
        let pos = rng.usize_in(1, n - 2);
        let (vals, target) = with_min_at(&mut rng, n, pos);
        let got = assert_same(&both, &vals, &format!("C11 iter {i} pos {pos}"));
        assert_eq!(got, target, "C11: minimum mid-chain");
    }
}

#[test]
fn c12_min_at_last_position() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0012);
    for i in 0..1000 {
        let n = rng.usize_in(4, 16);
        let (vals, target) = with_min_at(&mut rng, n, n - 1);
        let got = assert_same(&both, &vals, &format!("C12 iter {i}"));
        assert_eq!(got, target, "C12: `<` fires on the final node");
    }
}

// ------------------------------------------------------- C13/C14: ties and plateaus

#[test]
fn c13_all_values_identical() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0013);
    for i in 0..1000 {
        let n = rng.usize_in(4, 16);
        let v = rng.i32_any();
        let vals = vec![v; n];
        let got = assert_same(&both, &vals, &format!("C13 iter {i}"));
        assert_eq!(got, v, "C13: `<` never fires");
    }
}

#[test]
fn c14_narrow_range_many_ties() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0014);
    for i in 0..2000 {
        let n = rng.usize_in(8, 64);
        // Narrow domain => the minimum repeats many times.
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_in(-3, 3)).collect();
        let got = assert_same(&both, &vals, &format!("C14 iter {i}"));
        assert_eq!(got, reference_min(&vals), "C14 iter {i}");
    }
}

// ------------------------------------------------- C15..C19: value-domain shapes

#[test]
fn c15_all_positive() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0015);
    for i in 0..1000 {
        let n = rng.usize_in(4, 32);
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_in(1, i32::MAX)).collect();
        let got = assert_same(&both, &vals, &format!("C15 iter {i}"));
        assert_eq!(got, reference_min(&vals));
        assert!(got > 0, "C15: minimum of positives stays positive");
    }
}

#[test]
fn c16_all_negative() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0016);
    for i in 0..1000 {
        let n = rng.usize_in(4, 32);
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_in(i32::MIN, -1)).collect();
        let got = assert_same(&both, &vals, &format!("C16 iter {i}"));
        assert_eq!(got, reference_min(&vals));
        assert!(got < 0);
    }
}

#[test]
fn c17_int_min_injected() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0017);
    for i in 0..1000 {
        let n = rng.usize_in(4, 32);
        let mut vals: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        let pos = rng.usize_in(0, n - 1);
        vals[pos] = i32::MIN;
        let got = assert_same(&both, &vals, &format!("C17 iter {i} pos {pos}"));
        assert_eq!(got, i32::MIN, "C17: INT_MIN must win from any position");
    }
}

#[test]
fn c18_all_int_max() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0018);
    for i in 0..500 {
        let n = rng.usize_in(4, 32);
        let vals = vec![i32::MAX; n];
        let got = assert_same(&both, &vals, &format!("C18 iter {i}"));
        assert_eq!(got, i32::MAX);
    }
}

#[test]
fn c19_sign_boundary_values_only() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0019);
    // 0x80000000 as i32 == i32::MIN, 0xFFFFFFFF as i32 == -1. An unsigned
    // comparison would pick a different winner than the C's signed `<`.
    let domain = [
        i32::MIN,
        -1,
        0,
        1,
        i32::MAX,
        0x7FFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
    ];
    for i in 0..2000 {
        let n = rng.usize_in(4, 32);
        let vals: Vec<i32> = (0..n).map(|_| rng.pick(&domain)).collect();
        let got = assert_same(&both, &vals, &format!("C19 iter {i}"));
        assert_eq!(got, reference_min(&vals), "C19 iter {i}: signed compare");
    }
}

// ----------------------------------------------- C20: -1 collides with sentinel

#[test]
fn c20_minus_one_present_collides_with_sentinel() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0020);
    for i in 0..1000 {
        let n = rng.usize_in(4, 32);
        // Fillers >= 0 so that the planted -1 is the true minimum.
        let mut vals: Vec<i32> = (0..n).map(|_| rng.i32_in(0, i32::MAX)).collect();
        let pos = rng.usize_in(0, n - 1);
        vals[pos] = -1;
        let got = assert_same(&both, &vals, &format!("C20 iter {i} pos {pos}"));
        // Same value the C returns for a NULL head — the ambiguity is by design.
        assert_eq!(got, -1, "C20: valid list minimum -1");
    }
}

// ------------------------------------------------------- C21/C22: long chains

#[test]
fn c21_large_length_1000() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0021);
    for i in 0..50 {
        let vals: Vec<i32> = (0..1000).map(|_| rng.i32_any()).collect();
        let got = assert_same(&both, &vals, &format!("C21 iter {i}"));
        assert_eq!(got, reference_min(&vals));
    }
}

#[test]
fn c22_oversized_length_100k() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0022);
    for i in 0..3 {
        let vals: Vec<i32> = (0..100_000).map(|_| rng.i32_any()).collect();
        let got = assert_same(&both, &vals, &format!("C22 iter {i}"));
        assert_eq!(got, reference_min(&vals));
    }
}

// ------------------------------------------- C23: repeated calls / no mutation

#[test]
fn c23_repeated_calls_do_not_mutate_chain() {
    let both = Both::load();
    let mut rng = Rng::new(0xC0FFEE_0023);
    for i in 0..500 {
        let n = rng.usize_in(1, 24);
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        let expected = reference_min(&vals);

        let mut list_c = List::new(&vals);
        let mut list_rust = List::new(&vals);
        let before_c = list_c.snapshot();
        let before_rust = list_rust.snapshot();

        // Call each implementation three times on the SAME chain: the C function
        // advances a local copy of `head`, so results must be stable and the
        // caller's chain untouched.
        for call in 0..3 {
            let got_c = unsafe { (both.c)(list_c.head()) };
            let got_rust = unsafe { (both.rust)(list_rust.head()) };
            assert_eq!(
                got_c, got_rust,
                "C23 iter {i} call {call}: C={got_c} Rust={got_rust}"
            );
            assert_eq!(got_c, expected, "C23 iter {i} call {call}");
        }

        assert_eq!(list_c.snapshot(), before_c, "C23: C mutated the chain");
        assert_eq!(
            list_rust.snapshot(),
            before_rust,
            "C23: Rust mutated the chain"
        );
        // Address-independent comparison across the two distinct chains.
        assert_eq!(
            list_c.value_shape(),
            list_rust.value_shape(),
            "C23: chain shape diverged"
        );
        assert_eq!(list_c.len(), list_rust.len());
    }
}
