//! Phase C -- error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid
//! input/condition, calls BOTH `.so`s, and asserts they return the SAME
//! sentinel/value -- never merely "both failed somehow".

mod harness;

use harness::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// E1 -- the library's ONLY rejection branch: `if (head)` false => `return -1`
// ---------------------------------------------------------------------------
#[test]
fn err_e1_null_head() {
    let empty: Vec<i32> = Vec::new();
    let got = assert_same("E1/null-head", std::ptr::null_mut(), &empty);
    assert_eq!(got, -1, "E1: the C sentinel for a NULL head is exactly -1, got {got}");
    assert_eq!(got.to_ne_bytes(), (-1i32).to_ne_bytes());
}

// ---------------------------------------------------------------------------
// G1 -- NULL head is rejected identically even after successful calls
//       (no cached/stale state in either implementation)
// ---------------------------------------------------------------------------
#[test]
fn err_g1_null_head_repeated() {
    let mut rng = Rng::new(SEED ^ 0x61);
    let empty: Vec<i32> = Vec::new();
    for _ in 0..200 {
        // A successful call in between, with a value that is NOT -1 so a stale
        // cached result would be visible.
        let mut v = rng.next_i32();
        if v == -1 {
            v = 7;
        }
        let list = List::new(&[v]);
        assert_same_expect("G1/success-between", &list, v);

        let got = assert_same("G1/null-head", std::ptr::null_mut(), &empty);
        assert_eq!(got, -1, "G1: NULL head must still yield -1, got {got}");
    }
}

// ---------------------------------------------------------------------------
// G2 -- sentinel aliasing: a list whose genuine minimum is -1 returns -1,
//       indistinguishable from the NULL-head error. Preserved C quirk.
// ---------------------------------------------------------------------------
#[test]
fn err_g2_sentinel_aliasing() {
    let empty: Vec<i32> = Vec::new();
    let null_result = assert_same("G2/null", std::ptr::null_mut(), &empty);

    // Single node holding -1.
    let one = List::new(&[-1]);
    let one_result = assert_same_expect("G2/single-minus-one", &one, -1);
    assert_eq!(
        null_result, one_result,
        "G2: the C API cannot distinguish these; both must be -1"
    );

    // -1 as the minimum at several positions / list shapes.
    let mut rng = Rng::new(SEED ^ 0x62);
    for _ in 0..200 {
        let n = rng.len_in(1, 32);
        let mut v: Vec<i32> = (0..n).map(|_| rng.range_i32(0, i32::MAX)).collect();
        v[rng.below(n)] = -1;
        let list = List::new(&v);
        let got = assert_same_expect("G2/min-is-minus-one", &list, -1);
        assert_eq!(got, null_result);
    }
}

// ---------------------------------------------------------------------------
// G3 -- INT_MIN: one step past the negative end of the value range
// ---------------------------------------------------------------------------
#[test]
fn err_g3_int_min() {
    let list = List::new(&[i32::MIN]);
    let got = assert_same_expect("G3/int-min-single", &list, i32::MIN);
    assert_eq!(got, -2147483648i32);

    // INT_MIN reachable only through the update branch (not via the head init).
    let list = List::new(&[0, i32::MIN]);
    assert_same_expect("G3/int-min-tail", &list, i32::MIN);
    let list = List::new(&[i32::MIN, 0]);
    assert_same_expect("G3/int-min-head", &list, i32::MIN);

    let mut rng = Rng::new(SEED ^ 0x63);
    for _ in 0..200 {
        let n = rng.len_in(1, 32);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        v[rng.below(n)] = i32::MIN;
        let list = List::new(&v);
        assert_same_expect("G3/int-min-random", &list, i32::MIN);
    }
}

// ---------------------------------------------------------------------------
// G4 -- INT_MAX: one step past the positive end of the value range
// ---------------------------------------------------------------------------
#[test]
fn err_g4_int_max() {
    let list = List::new(&[i32::MAX]);
    let got = assert_same_expect("G4/int-max-single", &list, i32::MAX);
    assert_eq!(got, 2147483647i32);

    // All-INT_MAX list: the update branch must never fire.
    let list = List::new(&[i32::MAX; 16]);
    assert_same_expect("G4/int-max-all", &list, i32::MAX);

    let list = List::new(&[i32::MAX, i32::MAX - 1]);
    assert_same_expect("G4/int-max-then-less", &list, i32::MAX - 1);
}

// ---------------------------------------------------------------------------
// G5 -- INT_MIN together with INT_MAX: `<` must be a SIGNED comparison
// ---------------------------------------------------------------------------
#[test]
fn err_g5_int_min_and_max() {
    for v in [
        vec![i32::MIN, i32::MAX],
        vec![i32::MAX, i32::MIN],
        vec![i32::MAX, 0, i32::MIN],
        vec![i32::MIN, 0, i32::MAX],
        vec![0, i32::MAX, i32::MIN, -1, 1],
        vec![i32::MAX, i32::MAX, i32::MIN, i32::MIN],
    ] {
        let list = List::new(&v);
        assert_same_expect("G5/min-and-max", &list, i32::MIN);
    }

    // -1 vs INT_MAX: an unsigned compare would wrongly pick INT_MAX.
    let list = List::new(&[i32::MAX, -1]);
    assert_same_expect("G5/max-then-neg1", &list, -1);
    let list = List::new(&[-1, i32::MAX]);
    assert_same_expect("G5/neg1-then-max", &list, -1);
}

// ---------------------------------------------------------------------------
// G6 -- the empty/one boundary: zero length (NULL) vs length 1
// ---------------------------------------------------------------------------
#[test]
fn err_g6_empty_vs_one() {
    let empty: Vec<i32> = Vec::new();
    let zero = assert_same("G6/zero-length", std::ptr::null_mut(), &empty);
    assert_eq!(zero, -1);

    let mut rng = Rng::new(SEED ^ 0x66);
    for _ in 0..200 {
        let v = rng.next_i32();
        let list = List::new(&[v]);
        let one = assert_same_expect("G6/one-length", &list, v);
        if v != -1 {
            assert_ne!(
                one, zero,
                "G6: a length-1 list holding {v} must not report the empty sentinel"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// G7 -- oversized length: a 100_000-node chain (iterative C, so no stack limit)
// ---------------------------------------------------------------------------
#[test]
fn err_g7_oversized_length() {
    const N: usize = 100_000;
    let mut rng = Rng::new(SEED ^ 0x67);

    // Minimum at the very last node: the deepest possible update.
    let mut v: Vec<i32> = (0..N).map(|_| rng.range_i32(0, i32::MAX)).collect();
    v[N - 1] = i32::MIN;
    let list = List::new(&v);
    assert_eq!(list.len(), N);
    assert_same_expect("G7/oversized-min-last", &list, i32::MIN);

    // Minimum at the very first node: no update over 100k iterations.
    let mut v: Vec<i32> = (0..N).map(|_| rng.range_i32(0, i32::MAX)).collect();
    v[0] = i32::MIN;
    let list = List::new(&v);
    assert_same_expect("G7/oversized-min-first", &list, i32::MIN);
}

// ---------------------------------------------------------------------------
// G8 -- arbitrary uninterpreted bit patterns crossing the FFI boundary.
//
// The C API declares no `enum`, so there is no "invalid variant" to feed it;
// the analogous hostile input is an arbitrary 32-bit pattern in `value`, since
// a C `int` accepts any of the 2^32 patterns. Sweeping them proves the Rust
// reinterprets the payload identically (no sign/zero-extension mismatch).
// ---------------------------------------------------------------------------
#[test]
fn err_g8_arbitrary_bit_patterns() {
    // Structured landmark patterns first.
    let patterns: [u32; 14] = [
        0x0000_0000, 0xFFFF_FFFF, 0x8000_0000, 0x7FFF_FFFF, 0x8000_0001, 0x7FFF_FFFE,
        0xFFFF_FFFE, 0x0000_0001, 0xDEAD_BEEF, 0xCAFE_BABE, 0xAAAA_AAAA, 0x5555_5555,
        0xFFFF_0000, 0x0000_FFFF,
    ];
    for &p in &patterns {
        let v = p as i32;
        let list = List::new(&[v]);
        let got = assert_same_expect("G8/pattern-single", &list, v);
        assert_eq!(
            got as u32, p,
            "G8: bit pattern 0x{p:08x} did not round-trip (got 0x{:08x})",
            got as u32
        );
    }
    // Every pairing of landmark patterns.
    for &a in &patterns {
        for &b in &patterns {
            let v = [a as i32, b as i32];
            let list = List::new(&v);
            let exp: c_int = std::cmp::min(a as i32, b as i32);
            assert_same_expect("G8/pattern-pair", &list, exp);
        }
    }
    // Random full-space sweep.
    let mut rng = Rng::new(SEED ^ 0x68);
    for _ in 0..400 {
        let n = rng.len_in(1, 40);
        let v: Vec<i32> = (0..n).map(|_| rng.next_u32() as i32).collect();
        let list = List::new(&v);
        let exp = expected(&v);
        assert_same_expect("G8/pattern-random", &list, exp);
    }
}

// ---------------------------------------------------------------------------
// G9 -- a node physically past the NULL terminator must never be read.
//
// The extra node lives in the SAME contiguous allocation immediately after the
// terminating node and holds INT_MIN, so an implementation that walks memory
// instead of honouring `next == NULL` would return INT_MIN.
// ---------------------------------------------------------------------------
#[test]
fn err_g9_node_past_terminator() {
    let mut rng = Rng::new(SEED ^ 0x69);
    for _ in 0..200 {
        let n = rng.len_in(1, 32);
        // n live nodes + 1 unreachable trap node, contiguous.
        let mut arena: Vec<CListNode> = Vec::with_capacity(n + 1);
        let live: Vec<i32> = (0..n).map(|_| rng.range_i32(0, i32::MAX)).collect();
        for &val in &live {
            arena.push(CListNode { value: val, next: std::ptr::null_mut() });
        }
        // The trap.
        arena.push(CListNode { value: i32::MIN, next: std::ptr::null_mut() });

        let mut arena: Box<[CListNode]> = arena.into_boxed_slice();
        let base = arena.as_mut_ptr();
        for i in 0..n {
            unsafe {
                (*base.add(i)).next = if i + 1 < n { base.add(i + 1) } else { std::ptr::null_mut() };
            }
        }

        let exp = expected(&live);
        let got = assert_same("G9/past-terminator", base, &live);
        assert_eq!(
            got, exp,
            "G9: the trap node past the terminator was read (got {got}, expected {exp})"
        );
        assert_ne!(got, i32::MIN, "G9: implementation walked past the NULL terminator");
        drop(arena);
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: a `next` pointer that is NULL on the very first node
// combined with every landmark value -- the "loop never entered" edge.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_single_node_landmarks() {
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let list = List::new(&[v]);
        assert_eq!(list.len(), 1);
        assert!(unsafe { (*list.head()).next.is_null() });
        assert_same_expect("generic/single-landmark", &list, v);
    }
}
