//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `dlopen`/`dlsym` and requires bit-identical results, with
//! many randomized inputs per row (fixed seed).

mod harness;

use harness::*;

/// Number of randomized inputs per property-style row.
const ITERS: usize = 400;

// ---------------------------------------------------------------------------
// C1 -- length 0: head == NULL (branch `if (head)` false)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_empty() {
    let empty: Vec<i32> = Vec::new();
    for _ in 0..64 {
        assert_same("C1/null-head", std::ptr::null_mut(), &empty);
    }
    let list = List::new(&empty);
    assert!(list.head().is_null());
    assert_same_expect("C1/empty-list", &list, -1);
}

// ---------------------------------------------------------------------------
// C2 -- length 1, random full-range value (loop body runs 0 times)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_single_random() {
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..ITERS {
        let v = [rng.next_i32()];
        let list = List::new(&v);
        assert_same_expect("C2/len1", &list, v[0]);
    }
    // Hand-picked landmarks alongside the random sweep.
    for v in [0i32, 1, -1, 2, -2, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        let list = List::new(&[v]);
        assert_same_expect("C2/len1-landmark", &list, v);
    }
}

// ---------------------------------------------------------------------------
// C3 -- length 2, min at head (`if (value < smallest)` false)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c3_len2_min_at_head() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for _ in 0..ITERS {
        let lo = rng.range_i32(i32::MIN, i32::MAX - 1);
        let hi = rng.range_i32(lo + 1, i32::MAX);
        let v = [lo, hi];
        let list = List::new(&v);
        assert_same_expect("C3/len2-min-head", &list, lo);
    }
}

// ---------------------------------------------------------------------------
// C4 -- length 2, min at tail (`if (value < smallest)` true)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_len2_min_at_tail() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for _ in 0..ITERS {
        let lo = rng.range_i32(i32::MIN, i32::MAX - 1);
        let hi = rng.range_i32(lo + 1, i32::MAX);
        let v = [hi, lo];
        let list = List::new(&v);
        assert_same_expect("C4/len2-min-tail", &list, lo);
    }
}

// ---------------------------------------------------------------------------
// C5 -- length 2, equal values (strict `<` never fires)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c5_len2_equal() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for _ in 0..ITERS {
        let x = rng.next_i32();
        let list = List::new(&[x, x]);
        assert_same_expect("C5/len2-equal", &list, x);
    }
    for x in [0i32, -1, 1, i32::MIN, i32::MAX] {
        let list = List::new(&[x, x]);
        assert_same_expect("C5/len2-equal-landmark", &list, x);
    }
}

// ---------------------------------------------------------------------------
// C6 -- length 3, min in the middle
// ---------------------------------------------------------------------------
#[test]
fn cfg_c6_len3_min_middle() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    for _ in 0..ITERS {
        let m = rng.range_i32(i32::MIN, i32::MAX - 1);
        let a = rng.range_i32(m + 1, i32::MAX);
        let b = rng.range_i32(m + 1, i32::MAX);
        let list = List::new(&[a, m, b]);
        assert_same_expect("C6/len3-min-middle", &list, m);
    }
}

// ---------------------------------------------------------------------------
// C7 -- strictly increasing: the update branch NEVER fires
// ---------------------------------------------------------------------------
#[test]
fn cfg_c7_strictly_increasing() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for _ in 0..ITERS {
        let n = rng.len_in(4, 64);
        let mut cur = rng.range_i32(-1_000_000, 1_000_000) as i64;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(cur as i32);
            cur += rng.range_i32(1, 1000) as i64;
        }
        let list = List::new(&v);
        assert_same_expect("C7/increasing", &list, v[0]);
    }
}

// ---------------------------------------------------------------------------
// C8 -- strictly decreasing: the update branch fires EVERY iteration
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_strictly_decreasing() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for _ in 0..ITERS {
        let n = rng.len_in(4, 64);
        let mut cur = rng.range_i32(-1_000_000, 1_000_000) as i64;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(cur as i32);
            cur -= rng.range_i32(1, 1000) as i64;
        }
        let last = *v.last().unwrap();
        let list = List::new(&v);
        assert_same_expect("C8/decreasing", &list, last);
    }
}

// ---------------------------------------------------------------------------
// C9 -- all values identical
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_all_equal() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    for _ in 0..ITERS {
        let n = rng.len_in(4, 64);
        let x = rng.next_i32();
        let v = vec![x; n];
        let list = List::new(&v);
        assert_same_expect("C9/all-equal", &list, x);
    }
}

// ---------------------------------------------------------------------------
// C10 -- uniformly random full-range values, random length
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_random_fullrange() {
    let mut rng = Rng::new(SEED ^ 0xCA);
    for _ in 0..ITERS {
        let n = rng.len_in(1, 64);
        let v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert_same_expect("C10/random", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C11 -- all strictly positive
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_all_positive() {
    let mut rng = Rng::new(SEED ^ 0xCB);
    for _ in 0..ITERS {
        let n = rng.len_in(1, 64);
        let v: Vec<i32> = (0..n).map(|_| rng.range_i32(1, i32::MAX)).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert!(exp > 0);
        assert_same_expect("C11/all-positive", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C12 -- all strictly negative (covers the sentinel-adjacent domain)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_all_negative() {
    let mut rng = Rng::new(SEED ^ 0xCC);
    for _ in 0..ITERS {
        let n = rng.len_in(1, 64);
        let v: Vec<i32> = (0..n).map(|_| rng.range_i32(i32::MIN, -1)).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert!(exp < 0);
        assert_same_expect("C12/all-negative", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C13 -- tiny alphabet {-1,0,1,...}: dense ties and duplicates
// ---------------------------------------------------------------------------
#[test]
fn cfg_c13_small_alphabet_ties() {
    let mut rng = Rng::new(SEED ^ 0xCD);
    const ALPHABET: [i32; 7] = [-2, -1, 0, 1, 2, 3, -3];
    for _ in 0..ITERS {
        let n = rng.len_in(1, 64);
        let v: Vec<i32> = (0..n).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert_same_expect("C13/small-alphabet", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C14 -- minimum at a random interior index
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_min_at_random_index() {
    let mut rng = Rng::new(SEED ^ 0xCE);
    for _ in 0..ITERS {
        let n = rng.len_in(2, 64);
        let m = rng.range_i32(i32::MIN, i32::MAX - 1);
        let mut v: Vec<i32> = (0..n).map(|_| rng.range_i32(m + 1, i32::MAX)).collect();
        let idx = rng.below(n);
        v[idx] = m;
        let list = List::new(&v);
        assert_same_expect("C14/min-at-index", &list, m);
    }
}

// ---------------------------------------------------------------------------
// C15 -- the minimum value duplicated at 2+ random positions
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_duplicated_minimum() {
    let mut rng = Rng::new(SEED ^ 0xCF);
    for _ in 0..ITERS {
        let n = rng.len_in(2, 64);
        let m = rng.range_i32(i32::MIN, i32::MAX - 1);
        let mut v: Vec<i32> = (0..n).map(|_| rng.range_i32(m + 1, i32::MAX)).collect();
        let dups = 2 + rng.below(std::cmp::max(1, n - 1));
        for _ in 0..dups {
            let idx = rng.below(n);
            v[idx] = m;
        }
        let list = List::new(&v);
        assert_same_expect("C15/duplicated-min", &list, m);
    }
}

// ---------------------------------------------------------------------------
// C16 -- minimum pinned at the LAST node (update on the final iteration)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c16_min_at_tail() {
    let mut rng = Rng::new(SEED ^ 0xD0);
    for _ in 0..ITERS {
        let n = rng.len_in(2, 64);
        let m = rng.range_i32(i32::MIN, i32::MAX - 1);
        let mut v: Vec<i32> = (0..n).map(|_| rng.range_i32(m + 1, i32::MAX)).collect();
        *v.last_mut().unwrap() = m;
        let list = List::new(&v);
        assert_same_expect("C16/min-at-tail", &list, m);
    }
}

// ---------------------------------------------------------------------------
// C17 -- i32::MIN and i32::MAX both present (signed compare, not unsigned)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c17_extremes_present() {
    let mut rng = Rng::new(SEED ^ 0xD1);
    for _ in 0..ITERS {
        let n = rng.len_in(2, 64);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let i = rng.below(n);
        let mut j = rng.below(n);
        while j == i {
            j = rng.below(n);
        }
        v[i] = i32::MIN;
        v[j] = i32::MAX;
        let list = List::new(&v);
        assert_same_expect("C17/extremes", &list, i32::MIN);
    }
}

// ---------------------------------------------------------------------------
// C18 -- nodes allocated in shuffled order: `next` order != address order
// ---------------------------------------------------------------------------
#[test]
fn cfg_c18_shuffled_layout() {
    let mut rng = Rng::new(SEED ^ 0xD2);
    for _ in 0..ITERS {
        let n = rng.len_in(2, 64);
        let v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        // Fisher-Yates over the allocation order.
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        let list = List::with_order(&v, &order);
        assert_eq!(list.walk_values(), v, "harness: shuffled list mis-linked");
        let exp = expected(&v);
        assert_same_expect("C18/shuffled-layout", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C19 -- long lists (1_000 and 100_000 nodes)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19_long_lists() {
    let mut rng = Rng::new(SEED ^ 0xD3);
    for &n in &[1000usize, 100_000usize] {
        for _ in 0..3 {
            let v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            let list = List::new(&v);
            let exp = expected(&v);
            assert_same_expect("C19/long", &list, exp);
        }
        // Also the adversarial orderings at this length.
        let mut asc: Vec<i32> = (0..n as i32).collect();
        let list = List::new(&asc);
        assert_same_expect("C19/long-ascending", &list, 0);
        asc.reverse();
        let list = List::new(&asc);
        assert_same_expect("C19/long-descending", &list, 0);
    }
}

// ---------------------------------------------------------------------------
// C20 -- interleave NULL and non-NULL calls: the function must be stateless
// ---------------------------------------------------------------------------
#[test]
fn cfg_c20_interleaved_stateless() {
    let mut rng = Rng::new(SEED ^ 0xD4);
    let empty: Vec<i32> = Vec::new();
    for _ in 0..ITERS {
        assert_same("C20/null-before", std::ptr::null_mut(), &empty);

        let n = rng.len_in(1, 32);
        let v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert_same_expect("C20/list", &list, exp);

        assert_same("C20/null-after", std::ptr::null_mut(), &empty);

        // And the same list again -- results must be reproducible.
        assert_same_expect("C20/list-again", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// C21 -- neither implementation may mutate the caller's nodes
// ---------------------------------------------------------------------------
#[test]
fn cfg_c21_no_mutation() {
    let mut rng = Rng::new(SEED ^ 0xD5);
    let im = impls();
    for _ in 0..ITERS {
        let n = rng.len_in(1, 64);
        let v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let list = List::new(&v);
        let before = list.walk_values();
        assert_eq!(before, v, "harness: list mis-linked");

        let c = im.c.smallest_value(list.head());
        let after_c = list.walk_values();
        assert_eq!(after_c, before, "C mutated the caller's nodes");

        let r = im.rust.smallest_value(list.head());
        let after_r = list.walk_values();
        assert_eq!(after_r, before, "Rust mutated the caller's nodes");

        assert_eq!(c, r, "C21 divergence: C={c} Rust={r} for {v:?}");
        assert_eq!(c, expected(&v));
    }
}
