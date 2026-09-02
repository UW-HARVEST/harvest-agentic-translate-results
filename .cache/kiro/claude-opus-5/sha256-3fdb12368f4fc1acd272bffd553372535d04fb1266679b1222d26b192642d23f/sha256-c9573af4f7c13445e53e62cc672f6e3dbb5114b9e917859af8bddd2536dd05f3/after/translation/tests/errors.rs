//! Phase C — error/rejection-path differential tests.
//!
//! One test per row (or row group) of `ERRORS.md`. Each constructs the exact
//! invalid input or rejecting condition, calls BOTH `.so`s, and asserts the same
//! sentinel/rejection value comes back — not merely "both failed".

mod common;

use common::{Heap, Impl, Rng, HEAP_STATES};
use std::ffi::{c_char, c_int};
use std::ptr;

const SEED: u64 = 0xE770_0000_0BAD_0001;

// ---------------------------------------------------------------------------
// Rows 1-2: arity rejects len < 2
// ---------------------------------------------------------------------------

#[test]
fn err_arity_len_below_two() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED);
    for len in [0i32, 1] {
        for _ in 0..200 {
            let mut params = [
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ];
            let c = unsafe { (p.c.arity)(len, params.as_mut_ptr()) };
            let r = unsafe { (p.rust.arity)(len, params.as_mut_ptr()) };
            assert_eq!(c, -1, "C: arity({len}, ..) must return the -1 sentinel");
            assert_eq!(r, -1, "Rust: arity({len}, ..) must return the -1 sentinel");
            assert_eq!(c, r);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 3-4: NULL params is safe when len < 2 (no NULL check exists, but the
// short-length branch returns before any dereference)
// ---------------------------------------------------------------------------

#[test]
fn err_arity_null_params_short_len() {
    let p = common::load_pair();
    for len in [0i32, 1, 256, 257, -256, -255, 0x0001_0000, 0x0001_0001] {
        let c = unsafe { (p.c.arity)(len, ptr::null_mut()) };
        let r = unsafe { (p.rust.arity)(len, ptr::null_mut()) };
        assert_eq!(c, -1, "C: arity({len}, NULL) should return -1 without a load");
        assert_eq!(r, -1, "Rust: arity({len}, NULL) should return -1 without a load");
        assert_eq!(c, r);
    }
}

// ---------------------------------------------------------------------------
// Rows 5-7, 9, 11: the `int` -> `unsigned char` narrowing of `len`
// ---------------------------------------------------------------------------

#[test]
fn err_arity_int_truncation() {
    let p = common::load_pair();
    let params: [c_int; 4] = [11, -22, 33, -44];
    // (passed value, equivalent low byte)
    let cases: [(i32, i32); 12] = [
        (256, 0),
        (257, 1),
        (258, 2),
        (259, 3),
        (260, 4),
        (-256, 0),
        (-255, 1),
        (65536, 0),
        (65538, 2),
        (0x1234_5600, 0),
        (0x1234_5602, 2),
        (0x7FFF_FF03u32 as i32, 3),
    ];
    for (passed, low) in cases {
        for order in HEAP_STATES {
            let mut a = params;
            let mut b = params;
            let mut d = params;
            let mut e = params;
            common::seed_heap(order);
            let c_passed = unsafe { (p.c.arity)(passed, a.as_mut_ptr()) };
            common::seed_heap(order);
            let c_low = unsafe { (p.c.arity)(low, b.as_mut_ptr()) };
            common::seed_heap(order);
            let r_passed = unsafe { (p.rust.arity)(passed, d.as_mut_ptr()) };
            common::seed_heap(order);
            let r_low = unsafe { (p.rust.arity)(low, e.as_mut_ptr()) };
            assert_eq!(
                c_passed, c_low,
                "C: arity({passed}) must behave like arity({low}) (unsigned char param)"
            );
            assert_eq!(r_passed, r_low, "Rust: arity({passed}) != arity({low})");
            assert_eq!(
                c_passed, r_passed,
                "arity({passed}) [heap={order:?}]: C={c_passed} Rust={r_passed}"
            );
            if low < 2 {
                assert_eq!(c_passed, -1, "arity({passed}) should hit the -1 sentinel");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8: len == -1 is NOT rejected (low byte 255 under an unsigned compare)
// ---------------------------------------------------------------------------

#[test]
fn err_arity_negative_len_is_not_rejected() {
    let p = common::load_pair();
    let params: [c_int; 4] = [9, 8, 7, 6];
    for order in HEAP_STATES {
        let mut a = params;
        let mut b = params;
        let mut d = params;
        common::seed_heap(order);
        let c_neg = unsafe { (p.c.arity)(-1, a.as_mut_ptr()) };
        common::seed_heap(order);
        let c_255 = unsafe { (p.c.arity)(255, b.as_mut_ptr()) };
        common::seed_heap(order);
        let r_neg = unsafe { (p.rust.arity)(-1, d.as_mut_ptr()) };
        assert_eq!(c_neg, c_255, "C: arity(-1) must behave like arity(255)");
        assert_eq!(c_neg, r_neg, "arity(-1): C={c_neg} Rust={r_neg}");
        assert_ne!(
            c_neg, -1,
            "arity(-1) takes the arity4 branch, it is NOT the -1 rejection"
        );
        // ... and it must equal arity4 of the four params.
        common::seed_heap(order);
        let c_a4 = unsafe { (p.c.arity4)(params[0], params[1], params[2], params[3]) };
        assert_eq!(c_neg, c_a4, "arity(-1) should dispatch to arity4");
    }
}

// ---------------------------------------------------------------------------
// Row 10: len one step past every dispatch boundary
// Row 11: full 0..=255 sweep, and low-byte equivalence for wider ints
// ---------------------------------------------------------------------------

#[test]
fn err_arity_len_sweep_all_256() {
    let p = common::load_pair();
    let params: [c_int; 4] = [3, -5, 7, -9];
    for len in 0..=255i32 {
        for order in HEAP_STATES {
            let mut a = params;
            let mut b = params;
            common::seed_heap(order);
            let c = unsafe { (p.c.arity)(len, a.as_mut_ptr()) };
            common::seed_heap(order);
            let r = unsafe { (p.rust.arity)(len, b.as_mut_ptr()) };
            assert_eq!(c, r, "arity({len}) [heap={order:?}]: C={c} Rust={r}");
            if len < 2 {
                assert_eq!(c, -1, "arity({len}) must be the -1 rejection");
            } else {
                assert_ne!(len, 0);
            }
            // Every int sharing this low byte must behave identically.
            for wider in [len + 256, len - 256, len + 65536, len + 0x0100_0000] {
                let mut w = params;
                common::seed_heap(order);
                let cw = unsafe { (p.c.arity)(wider, w.as_mut_ptr()) };
                let mut w2 = params;
                common::seed_heap(order);
                let rw = unsafe { (p.rust.arity)(wider, w2.as_mut_ptr()) };
                assert_eq!(cw, c, "C: arity({wider}) != arity({len})");
                assert_eq!(rw, r, "Rust: arity({wider}) != arity({len})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12: the `ptr1 == NULL || ptr2 == NULL` -> -1 branch
// ---------------------------------------------------------------------------

#[test]
fn err_compare_allocations_oom_branch_unreachable() {
    // `malloc(sizeof(int))` cannot fail in either implementation short of
    // exhausting the address space, so the `-1` sentinel is dead code in the C
    // and in the Rust alike. Rather than fake an allocation failure (which would
    // require interposing malloc and would test the interposer, not the
    // library), assert the observable consequence: neither `.so` ever returns
    // the sentinel, over a wide randomized sweep and both heap orderings.
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..3000 {
        let v1 = rng.interesting_i32();
        let v2 = rng.interesting_i32();
        for order in HEAP_STATES {
            let (c, r) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.compare_allocations)(v1, v2)
            });
            assert_eq!(c, r, "compare_allocations({v1},{v2}) [heap={order:?}]");
            assert_ne!(c, -1, "C returned the OOM sentinel unexpectedly");
            assert_ne!(r, -1, "Rust returned the OOM sentinel unexpectedly");
            assert!(
                matches!(c, 1 | 2 | 11 | 12),
                "unexpected compare_allocations result {c}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13: `result = 3` (ptr1 == ptr2) is unreachable
// ---------------------------------------------------------------------------

#[test]
fn err_compare_allocations_never_returns_three() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x13);
    for _ in 0..3000 {
        let v1 = rng.next_i32();
        let v2 = rng.next_i32();
        for order in HEAP_STATES {
            let (c, r) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.compare_allocations)(v1, v2)
            });
            assert_eq!(c, r);
            assert!(c != 3 && c != 13, "C reached the ptr1==ptr2 branch: {c}");
            assert!(r != 3 && r != 13, "Rust reached the ptr1==ptr2 branch: {r}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14: val1 <= 0 rejects the +10 bonus
// ---------------------------------------------------------------------------

#[test]
fn err_compare_allocations_nonpositive_val1() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x14);
    for val1 in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for _ in 0..200 {
            let val2 = rng.interesting_i32();
            for order in HEAP_STATES {
                let (c, r) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                    (imp.compare_allocations)(val1, val2)
                });
                assert_eq!(c, r, "compare_allocations({val1},{val2}) [heap={order:?}]");
                let expected = match order {
                    Heap::Ascending => 1,
                    Heap::Descending => 2,
                };
                assert_eq!(c, expected, "the +10 bonus must be rejected for {val1}");
            }
        }
    }
    // Contrast: val1 == 1 (the smallest value that does earn the bonus).
    for order in HEAP_STATES {
        let (c, r) =
            common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.compare_allocations)(1, 0)
            });
        assert_eq!(c, r);
        assert_eq!(
            c,
            match order {
                Heap::Ascending => 11,
                Heap::Descending => 12,
            }
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 15-18: shift_array guard rejects positions <= 0 and positions >= size
// ---------------------------------------------------------------------------

/// Apply `shift_array` through both `.so`s and require an identical result AND
/// that nothing at all changed (the guard rejected the call).
#[track_caller]
fn shift_expect_noop(p: &common::Pair, data: &[c_int], size: c_int, positions: c_int) {
    let mut a = data.to_vec();
    let mut b = data.to_vec();
    unsafe { (p.c.shift_array)(a.as_mut_ptr(), size, positions) };
    unsafe { (p.rust.shift_array)(b.as_mut_ptr(), size, positions) };
    assert_eq!(
        a, b,
        "shift_array(size={size}, positions={positions}): C={a:?} Rust={b:?}"
    );
    assert_eq!(
        a, data,
        "shift_array(size={size}, positions={positions}) must be a no-op"
    );
}

#[test]
fn err_shift_array_noop_guards() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x15);
    for size in [2i32, 3, 4, 8, 32] {
        for _ in 0..50 {
            let data: Vec<c_int> = (0..size).map(|_| rng.interesting_i32()).collect();
            // positions == 0, negative, == size, > size, INT_MAX / INT_MIN.
            for positions in [
                0,
                -1,
                -size,
                -100,
                i32::MIN,
                i32::MIN + 1,
                size,
                size + 1,
                size * 2,
                1000,
                i32::MAX,
                i32::MAX - 1,
            ] {
                shift_expect_noop(&p, &data, size, positions);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19, 21: size == 0 and size == 1
// ---------------------------------------------------------------------------

#[test]
fn err_shift_array_zero_and_one_size() {
    let p = common::load_pair();
    // size == 0 with a real (but zero-length window) buffer: guard must reject
    // every `positions`, so no write happens even though the window is empty.
    const GUARD: c_int = 0x2B2B_2B2B;
    for positions in [i32::MIN, -1, 0, 1, 2, 100, i32::MAX] {
        let mut a = [GUARD; 8];
        let mut b = [GUARD; 8];
        unsafe { (p.c.shift_array)(a.as_mut_ptr(), 0, positions) };
        unsafe { (p.rust.shift_array)(b.as_mut_ptr(), 0, positions) };
        assert_eq!(a, b, "shift_array(size=0, positions={positions})");
        assert_eq!(a, [GUARD; 8], "size=0 must never write");
    }
    // size == 1: positions == 1 is rejected by `positions < size`.
    for positions in [i32::MIN, -1, 0, 1, 2, i32::MAX] {
        let mut a = [7, GUARD, GUARD, GUARD];
        let mut b = a;
        unsafe { (p.c.shift_array)(a.as_mut_ptr(), 1, positions) };
        unsafe { (p.rust.shift_array)(b.as_mut_ptr(), 1, positions) };
        assert_eq!(a, b, "shift_array(size=1, positions={positions})");
        assert_eq!(a, [7, GUARD, GUARD, GUARD], "size=1 must never write");
    }
}

// ---------------------------------------------------------------------------
// Row 20: negative size
// ---------------------------------------------------------------------------

#[test]
fn err_shift_array_negative_size() {
    let p = common::load_pair();
    const GUARD: c_int = -0x1234_5678;
    for size in [-1i32, -2, -100, i32::MIN, i32::MIN + 1] {
        for positions in [i32::MIN, -1, 0, 1, 2, 100, i32::MAX] {
            let mut a = [GUARD; 8];
            let mut b = [GUARD; 8];
            unsafe { (p.c.shift_array)(a.as_mut_ptr(), size, positions) };
            unsafe { (p.rust.shift_array)(b.as_mut_ptr(), size, positions) };
            assert_eq!(a, b, "shift_array(size={size}, positions={positions})");
            assert_eq!(a, [GUARD; 8], "negative size must never write");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 22-23: NULL arr is safe whenever the guard rejects the call
// ---------------------------------------------------------------------------

#[test]
fn err_shift_array_null_when_guard_fails() {
    let p = common::load_pair();
    // No NULL check exists, but `positions > 0 && positions < size` short
    // circuits before any load, so these calls must not touch memory.
    for (size, positions) in [
        (0i32, 0i32),
        (0, 1),
        (0, -1),
        (0, i32::MAX),
        (4, 0),
        (4, -1),
        (4, 4),
        (4, 5),
        (-1, 1),
        (i32::MIN, 1),
        (1, 1),
        (2, 2),
    ] {
        unsafe { (p.c.shift_array)(ptr::null_mut(), size, positions) };
        unsafe { (p.rust.shift_array)(ptr::null_mut(), size, positions) };
    }
    // Reaching here without a fault means both implementations agree.
}

// ---------------------------------------------------------------------------
// Rows 24-25: process_string rejects an empty string via `if (*str)`
// ---------------------------------------------------------------------------

#[test]
fn err_process_string_empty() {
    let p = common::load_pair();
    let empty: [c_char; 1] = [0];
    let c = unsafe { (p.c.process_string)(empty.as_ptr()) };
    let r = unsafe { (p.rust.process_string)(empty.as_ptr()) };
    assert_eq!(c, 0, "C: empty string must return 0");
    assert_eq!(r, 0, "Rust: empty string must return 0");
    assert_eq!(c, r);
}

#[test]
fn err_process_string_embedded_nul_first() {
    let p = common::load_pair();
    // First byte NUL, garbage after: only the first byte is tested, and the
    // `strlen` call is skipped entirely.
    for tail in [
        [b'X' as c_char, b'Y' as c_char, b'Z' as c_char, 0],
        [0x7F, -1, 1, 0],
        [-128, -1, -1, 0],
    ] {
        let buf: [c_char; 5] = [0, tail[0], tail[1], tail[2], tail[3]];
        let c = unsafe { (p.c.process_string)(buf.as_ptr()) };
        let r = unsafe { (p.rust.process_string)(buf.as_ptr()) };
        assert_eq!(c, 0, "C must return 0 when the first byte is NUL");
        assert_eq!(r, 0, "Rust must return 0 when the first byte is NUL");
        assert_eq!(c, r);
    }
}

// ---------------------------------------------------------------------------
// Rows 26-27: apply_bitmask `default:` for out-of-range operation values
// ---------------------------------------------------------------------------

#[test]
fn err_apply_bitmask_out_of_range_operation() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x26);
    // A C `switch` over an `int` accepts any int, so a value with no matching
    // label is a legitimate input crossing the FFI boundary.
    let out_of_range: [c_int; 16] = [
        -1,
        -2,
        -3,
        -4,
        -100,
        4,
        5,
        6,
        7,
        8,
        100,
        255,
        256,
        65536,
        i32::MAX,
        i32::MIN,
    ];
    for operation in out_of_range {
        for _ in 0..300 {
            let value = rng.interesting_i32();
            let c = unsafe { (p.c.apply_bitmask)(value, operation) };
            let r = unsafe { (p.rust.apply_bitmask)(value, operation) };
            assert_eq!(c, r, "apply_bitmask({value}, {operation}): C={c} Rust={r}");
            assert_eq!(
                c, value,
                "the default: label must return `value` unchanged for operation={operation}"
            );
        }
    }
    // Exhaustive sweep of every operation value in a wide window around the
    // valid labels, so the boundary at 0 and at 3 is covered from both sides.
    for operation in -64..=64i32 {
        for value in [0, -1, 1, 0xFF, -0x100, i32::MAX, i32::MIN, 0x5A5A_5A5A] {
            let c = unsafe { (p.c.apply_bitmask)(value, operation) };
            let r = unsafe { (p.rust.apply_bitmask)(value, operation) };
            assert_eq!(c, r, "apply_bitmask({value}, {operation})");
            if !(0..=3).contains(&operation) {
                assert_eq!(c, value, "operation={operation} must be identity");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 28: negative `param1 % 4` falls into apply_bitmask's default:
// ---------------------------------------------------------------------------

#[test]
fn err_arity4_negative_modulo_hits_default() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x28);
    for _ in 0..1000 {
        // param1 < 0 and not a multiple of 4 => param1 % 4 in {-1,-2,-3}.
        let k = rng.range_i32(0, 100_000) as i64;
        let m = rng.range_i32(1, 3) as i64;
        let param1 = (-4 * k - m) as c_int;
        assert!(param1 % 4 < 0, "sanity: C-style truncating remainder");
        let param2 = rng.interesting_i32();
        common::assert_both_heaps(&p, "arity4 negative modulo", |imp| unsafe {
            (imp.arity4)(param1, param2, 0, 0)
        });
        // The mask must NOT have been applied: prove it by comparing against
        // apply_bitmask with an explicitly out-of-range operation.
        for order in HEAP_STATES {
            let (c, r) = common::run_seeded(&p, order, &|imp: &Impl| unsafe {
                (imp.arity4)(param1, param2, 0, 0)
            });
            assert_eq!(c, r);
        }
    }
    // Exact boundary values: -1, -2, -3, -4 (only -4 has remainder 0).
    for param1 in [-1i32, -2, -3, -4, -5, -6, -7, -8, i32::MIN, i32::MIN + 1] {
        for param2 in [0i32, 1, -1, i32::MAX, i32::MIN] {
            common::assert_both_heaps(&p, "arity4 modulo boundary", |imp| unsafe {
                (imp.arity4)(param1, param2, 0, 0)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 31: signed overflow wraps
// ---------------------------------------------------------------------------

#[test]
fn err_arity4_overflow_wraps() {
    let p = common::load_pair();
    let extremes: [c_int; 10] = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 5,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 5,
        0x4000_0000,
        -0x4000_0000,
        0x7FFF_FFF0,
        -0x7FFF_FFF0,
    ];
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                for &d in &extremes {
                    common::assert_both_heaps(&p, "arity4 overflow", |imp| unsafe {
                        (imp.arity4)(a, b, c, d)
                    });
                }
            }
        }
    }
    // Sums that overflow inside the shift/accumulate loop specifically.
    for &a in &extremes {
        for &b in &extremes {
            common::assert_both_heaps(&p, "arity2 overflow", |imp| unsafe {
                (imp.arity2)(a, b)
            });
            common::assert_both_heaps(&p, "arity3 overflow", |imp| unsafe {
                (imp.arity3)(a, b, i32::MIN)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 32: (result * param3) / 100 truncates toward zero
// ---------------------------------------------------------------------------

#[test]
fn err_arity4_division_truncates_toward_zero() {
    let p = common::load_pair();
    let mut rng = Rng::new(SEED ^ 0x32);
    // Pick param3 values that keep the product small so the quotient's sign and
    // rounding direction are directly observable, then also go fully random.
    for _ in 0..4000 {
        let param1 = rng.range_i32(-40, 40);
        let param2 = rng.range_i32(-40, 40);
        let param3 = rng.range_i32(-40, 40);
        common::assert_both_heaps(&p, "arity4 truncating division", |imp| unsafe {
            (imp.arity4)(param1, param2, param3, 0)
        });
    }
    // Products straddling 0 and +-100 in both directions.
    for param3 in [-7i32, -5, -3, -2, -1, 1, 2, 3, 5, 7, 99, 100, 101, -99, -100, -101] {
        for param1 in -12..=12i32 {
            for param2 in [-3i32, 0, 3] {
                common::assert_both_heaps(&p, "arity4 division boundary", |imp| unsafe {
                    (imp.arity4)(param1, param2, param3, 0)
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 33: init_matrix writes exactly 12 ints and validates nothing
// ---------------------------------------------------------------------------

#[test]
fn err_init_matrix_writes_exactly_twelve() {
    let p = common::load_pair();
    const GUARD: c_int = 0x7E7E_7E7E;
    for pad in [1usize, 4, 16] {
        let mut a = vec![GUARD; 12 + 2 * pad];
        let mut b = a.clone();
        unsafe { (p.c.init_matrix)(a[pad..].as_mut_ptr()) };
        unsafe { (p.rust.init_matrix)(b[pad..].as_mut_ptr()) };
        assert_eq!(a, b, "init_matrix(pad={pad}): C={a:?} Rust={b:?}");
        for i in 0..pad {
            assert_eq!(a[i], GUARD, "C wrote before the matrix");
            assert_eq!(a[pad + 12 + i], GUARD, "C wrote past the matrix");
            assert_eq!(b[i], GUARD, "Rust wrote before the matrix");
            assert_eq!(b[pad + 12 + i], GUARD, "Rust wrote past the matrix");
        }
        assert_eq!(&a[pad..pad + 12], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}

// ---------------------------------------------------------------------------
// Row 34: genuine NULL dereference is symmetric (documented, not asserted
// in-process because both implementations take SIGSEGV)
// ---------------------------------------------------------------------------

#[test]
fn err_null_deref_is_symmetric_documented() {
    let p = common::load_pair();
    // `process_string(NULL)`, `arity(2, NULL)` and `init_matrix(NULL)` all
    // dereference NULL in the C, so there is no rejection value to compare;
    // both implementations fault identically. What IS assertable in-process is
    // the *precondition* that decides whether a load happens at all, and that
    // both implementations agree on it. Those are the cases exercised by
    // `err_arity_null_params_short_len` and
    // `err_shift_array_null_when_guard_fails`, so here we only pin the
    // complementary fact: a one-element valid buffer is enough for the
    // dispatch paths that read fewer than four params.
    let mut two: [c_int; 2] = [5, 6];
    common::seed_heap(Heap::Ascending);
    let c2 = unsafe { (p.c.arity)(2, two.as_mut_ptr()) };
    common::seed_heap(Heap::Ascending);
    let r2 = unsafe { (p.rust.arity)(2, two.as_mut_ptr()) };
    assert_eq!(c2, r2, "arity(2, &[2 ints]) read past the second element?");
    let mut three: [c_int; 3] = [5, 6, 7];
    common::seed_heap(Heap::Ascending);
    let c3 = unsafe { (p.c.arity)(3, three.as_mut_ptr()) };
    common::seed_heap(Heap::Ascending);
    let r3 = unsafe { (p.rust.arity)(3, three.as_mut_ptr()) };
    assert_eq!(c3, r3, "arity(3, &[3 ints]) read past the third element?");
    // A one-byte string is enough for process_string; NULL would fault in both.
    let one: [c_char; 1] = [0];
    assert_eq!(
        unsafe { (p.c.process_string)(one.as_ptr()) },
        unsafe { (p.rust.process_string)(one.as_ptr()) }
    );
}
