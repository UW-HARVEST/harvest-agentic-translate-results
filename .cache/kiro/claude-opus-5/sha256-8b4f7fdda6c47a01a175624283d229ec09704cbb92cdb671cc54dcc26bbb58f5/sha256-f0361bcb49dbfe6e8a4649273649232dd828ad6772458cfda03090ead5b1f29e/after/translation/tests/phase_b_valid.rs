//! Phase B — valid-path differential tests, gated on `CONFIGS.md`.
//!
//! Every function is reached through `dlopen`/`dlsym` on both the C `.so` and
//! the Rust `.so`; the Rust crate is never called directly.
//!
//! Row numbers refer to the table in `CONFIGS.md`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

unsafe extern "C" {
    fn free(p: *mut c_void);
}

const VALID_OPS: [c_int; 5] = [1, 2, 3, 4, 5];
const DEFAULT_BUCKET: [c_int; 9] = [0, 6, 7, 100, -1, -5, -100, c_int::MIN, c_int::MAX];

// --------------------------------------------------------------------------
// Row 1 — is_valid_operation over the exhaustive char domain.
// --------------------------------------------------------------------------
#[test]
fn row01_is_valid_operation_exhaustive_char() {
    for raw in 0u16..256 {
        let ch = raw as u8 as i8 as std::ffi::c_char;
        let cv = unsafe { (c().is_valid_operation)(ch) };
        let rv = unsafe { (r().is_valid_operation)(ch) };
        assert_eq!(cv, rv, "is_valid_operation({ch}) raw byte C={cv} RUST={rv}");
    }
}

// --------------------------------------------------------------------------
// Rows 2 & 3 — get_operation_priority.
// --------------------------------------------------------------------------
#[test]
fn row02_get_operation_priority_valid_enumerators() {
    for op in VALID_OPS {
        let cv = unsafe { (c().get_operation_priority)(op) };
        let rv = unsafe { (r().get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
}

#[test]
fn row03_get_operation_priority_out_of_range_and_random() {
    for op in DEFAULT_BUCKET.iter().copied().chain(BOUNDARIES) {
        let cv = unsafe { (c().get_operation_priority)(op) };
        let rv = unsafe { (r().get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
    let mut rng = Rng::new(0xC0FFEE_01);
    for _ in 0..5000 {
        let op = rng.next_i32_mixed();
        let cv = unsafe { (c().get_operation_priority)(op) };
        let rv = unsafe { (r().get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
}

// --------------------------------------------------------------------------
// Rows 4-8 — the five arithmetic primitives.
// --------------------------------------------------------------------------
fn diff_binop(label: &str, cf: FnMathOp, rf: FnMathOp, seed: u64, skip_zero_b: bool) {
    // Boundary grid.
    for &a in BOUNDARIES.iter() {
        for &b in BOUNDARIES.iter() {
            if skip_zero_b && b == 0 {
                continue;
            }
            // INT_MIN / -1 traps (ERRORS.md rows 7/8) — covered out-of-process.
            if skip_zero_b && a == c_int::MIN && b == -1 {
                continue;
            }
            for &u in &[0, 1, -1, c_int::MAX] {
                let cv = unsafe { cf(a, b, u) };
                let rv = unsafe { rf(a, b, u) };
                assert_eq!(cv, rv, "{label}({a}, {b}, {u})");
            }
        }
    }
    // Randomized, property-style.
    let mut rng = Rng::new(seed);
    for _ in 0..20_000 {
        let a = rng.next_i32_mixed();
        let mut b = rng.next_i32_mixed();
        if skip_zero_b {
            if b == 0 {
                b = 7;
            }
            if a == c_int::MIN && b == -1 {
                b = 3;
            }
        }
        let u = rng.next_i32();
        let cv = unsafe { cf(a, b, u) };
        let rv = unsafe { rf(a, b, u) };
        assert_eq!(cv, rv, "{label}({a}, {b}, {u})");
    }
}

#[test]
fn row04_add_operation() {
    diff_binop("add_operation", c().add_operation, r().add_operation, 0xADD_1, false);
}

#[test]
fn row05_multiply_operation() {
    diff_binop("multiply_operation", c().multiply_operation, r().multiply_operation, 0x111_2, false);
}

#[test]
fn row06_subtract_operation() {
    diff_binop("subtract_operation", c().subtract_operation, r().subtract_operation, 0x5AB_3, false);
}

#[test]
fn row07_divide_operation() {
    // b == 0 is a valid, non-trapping input here (guarded by the C code) and is
    // additionally asserted in Phase C; the grid below covers b != 0.
    diff_binop("divide_operation", c().divide_operation, r().divide_operation, 0xD1D4, true);
    // Explicit truncation-toward-zero cases.
    for &(a, b) in &[
        (7, 2),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (1, c_int::MIN),
        (c_int::MIN, 1),
        (c_int::MIN, 2),
        (c_int::MAX, -1),
        (c_int::MIN, c_int::MAX),
    ] {
        let cv = unsafe { (c().divide_operation)(a, b, 0) };
        let rv = unsafe { (r().divide_operation)(a, b, 0) };
        assert_eq!(cv, rv, "divide_operation({a}, {b})");
    }
}

#[test]
fn row08_modulo_operation() {
    diff_binop("modulo_operation", c().modulo_operation, r().modulo_operation, 0x30D_5, true);
    for &(a, b) in &[
        (7, 2),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (1, c_int::MIN),
        (c_int::MIN, 1),
        (c_int::MIN, 2),
        (c_int::MAX, -1),
        (c_int::MIN, c_int::MAX),
    ] {
        let cv = unsafe { (c().modulo_operation)(a, b, 0) };
        let rv = unsafe { (r().modulo_operation)(a, b, 0) };
        assert_eq!(cv, rv, "modulo_operation({a}, {b})");
    }
}

// --------------------------------------------------------------------------
// Rows 9-11 — select_operation: identity and dispatch.
// --------------------------------------------------------------------------
#[test]
fn row09_select_operation_valid_enumerators() {
    let expected = [
        "add_operation",
        "multiply_operation",
        "subtract_operation",
        "divide_operation",
        "modulo_operation",
    ];
    for (i, op) in VALID_OPS.iter().copied().enumerate() {
        let cp = unsafe { (c().select_operation)(op) };
        let rp = unsafe { (r().select_operation)(op) };
        let cid = c().op_identity(cp);
        let rid = r().op_identity(rp);
        assert_eq!(cid, rid, "select_operation({op}) identity");
        assert_eq!(cid, expected[i], "select_operation({op}) is not the C switch arm");
    }
}

#[test]
fn row10_select_operation_default_bucket() {
    for op in DEFAULT_BUCKET.iter().copied().chain(BOUNDARIES) {
        let cid = c().op_identity(unsafe { (c().select_operation)(op) });
        let rid = r().op_identity(unsafe { (r().select_operation)(op) });
        assert_eq!(cid, rid, "select_operation({op}) identity");
        assert_eq!(cid, "add_operation", "select_operation({op}) default arm");
    }
    let mut rng = Rng::new(0x5E1EC7);
    for _ in 0..5000 {
        let op = rng.next_i32_mixed();
        let cid = c().op_identity(unsafe { (c().select_operation)(op) });
        let rid = r().op_identity(unsafe { (r().select_operation)(op) });
        assert_eq!(cid, rid, "select_operation({op}) identity");
        if !(1..=5).contains(&op) {
            assert_eq!(cid, "add_operation", "select_operation({op}) default arm");
        }
    }
}

#[test]
fn row11_select_operation_dispatch_invoked() {
    let mut rng = Rng::new(0xD15A7C);
    for _ in 0..8000 {
        let op = if rng.next_u64() % 2 == 0 {
            rng.range(1, 5) as c_int
        } else {
            rng.next_i32_mixed()
        };
        let a = rng.next_i32_mixed();
        let mut b = rng.next_i32_mixed();
        // Avoid the trapping INT_MIN / -1 for the div/mod arms.
        if a == c_int::MIN && b == -1 {
            b = 3;
        }
        let u = rng.next_i32();
        let cf: FnMathOp = unsafe { std::mem::transmute((c().select_operation)(op)) };
        let rf: FnMathOp = unsafe { std::mem::transmute((r().select_operation)(op)) };
        let cv = unsafe { cf(a, b, u) };
        let rv = unsafe { rf(a, b, u) };
        assert_eq!(cv, rv, "select_operation({op})({a},{b},{u})");
    }
}

// --------------------------------------------------------------------------
// Row 12 — get_computation_timestamp.
// --------------------------------------------------------------------------
#[test]
fn row12_get_computation_timestamp() {
    for _ in 0..100 {
        let cv = unsafe { (c().get_computation_timestamp)() };
        let rv = unsafe { (r().get_computation_timestamp)() };
        assert_eq!(cv, rv, "get_computation_timestamp (time()>>29)");
        // Sanity: 2025-ish unix time >> 29 == 3.
        assert!(cv > 0, "timestamp should be positive, got {cv}");
    }
}

// --------------------------------------------------------------------------
// Rows 13-16 — allocate_results.
// --------------------------------------------------------------------------
fn diff_allocate(count: c_int, check_zero: bool) {
    let cp = unsafe { (c().allocate_results)(count) };
    let rp = unsafe { (r().allocate_results)(count) };
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "allocate_results({count}) NULL-ness: C={:?} RUST={:?}",
        cp.is_null(),
        rp.is_null()
    );
    if !cp.is_null() && check_zero && count > 0 {
        let n = count as usize;
        let cb = unsafe { raw_bytes(cp, n) };
        let rb = unsafe { raw_bytes(rp, n) };
        assert_eq!(cb, rb, "allocate_results({count}) contents differ");
        assert!(
            cb.iter().all(|&x| x == 0),
            "allocate_results({count}) must be zeroed (calloc)"
        );
        assert_eq!(cb.len(), n * SIZEOF_COMPUTATION_RESULT);
    }
    unsafe {
        free(cp as *mut c_void);
        free(rp as *mut c_void);
    }
}

#[test]
fn row13_allocate_results_one() {
    diff_allocate(1, true);
}

#[test]
fn row14_allocate_results_ten() {
    diff_allocate(10, true);
}

#[test]
fn row15_allocate_results_zero() {
    diff_allocate(0, false);
}

#[test]
fn row16_allocate_results_random_counts() {
    let mut rng = Rng::new(0xA110C);
    for _ in 0..200 {
        let n = rng.range(1, 64) as c_int;
        diff_allocate(n, true);
    }
}

// --------------------------------------------------------------------------
// Row 17 — struct layout probe: record into slot k and compare every byte,
// padding included.
// --------------------------------------------------------------------------
#[test]
fn row17_struct_layout_bytes() {
    let n = 10usize;
    for k in 0..n as c_int {
        let cp = unsafe { (c().allocate_results)(n as c_int) };
        let rp = unsafe { (r().allocate_results)(n as c_int) };
        assert!(!cp.is_null() && !rp.is_null());

        let mut ch = cp;
        let mut rh = rp;
        let mut cc = k;
        let mut rc = k;
        let cv = unsafe { (c().perform_computation_with_history)(1234, 56, 2, &mut ch, &mut cc) };
        let rv = unsafe { (r().perform_computation_with_history)(1234, 56, 2, &mut rh, &mut rc) };
        assert_eq!(cv, rv, "return value, slot {k}");
        assert_eq!(cc, rc, "history_count, slot {k}");
        let cb = unsafe { raw_bytes(cp, n) };
        let rb = unsafe { raw_bytes(rp, n) };
        assert_eq!(cb, rb, "24-byte layout mismatch writing slot {k}");
        unsafe {
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}

// --------------------------------------------------------------------------
// Row 18 — *history == NULL: lazy allocation resets the count.
// --------------------------------------------------------------------------
#[test]
fn row18_null_history_lazy_alloc_resets_count() {
    let mut rng = Rng::new(0x1A21);
    for start_count in [0, 1, 5, 9, 10, 11, -1, c_int::MAX] {
        for op in VALID_OPS.iter().copied().chain(DEFAULT_BUCKET) {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }

            let mut ch: *mut ComputationResult = std::ptr::null_mut();
            let mut rh: *mut ComputationResult = std::ptr::null_mut();
            let mut cc = start_count;
            let mut rc = start_count;

            let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
            let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            assert_eq!(cv, rv, "ret for op={op} a={a} b={b} start={start_count}");
            assert_eq!(cc, rc, "count for op={op} start={start_count}");
            assert_eq!(cc, 1, "C resets to 0 then records: count must be 1");
            assert_eq!(ch.is_null(), rh.is_null(), "allocation NULL-ness");
            let cb = unsafe { raw_bytes(ch, 10) };
            let rb = unsafe { raw_bytes(rh, 10) };
            assert_eq!(cb, rb, "buffer bytes for op={op} a={a} b={b}");
            unsafe {
                free(ch as *mut c_void);
                free(rh as *mut c_void);
            }
        }
    }
}

// --------------------------------------------------------------------------
// Rows 19-23 — non-NULL history at various counts / ops.
// --------------------------------------------------------------------------
fn diff_pcwh_at(start_count: c_int, op: c_int, a: c_int, b: c_int) {
    let n = 10usize;
    let cp = unsafe { (c().allocate_results)(n as c_int) };
    let rp = unsafe { (r().allocate_results)(n as c_int) };
    assert!(!cp.is_null() && !rp.is_null());

    let mut ch = cp;
    let mut rh = rp;
    let mut cc = start_count;
    let mut rc = start_count;
    let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
    let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

    assert_eq!(cv, rv, "ret: start={start_count} op={op} a={a} b={b}");
    assert_eq!(cc, rc, "count: start={start_count} op={op} a={a} b={b}");
    assert_eq!(ch, cp, "C must not reallocate a non-NULL history");
    assert_eq!(rh, rp, "Rust must not reallocate a non-NULL history");
    let cb = unsafe { raw_bytes(cp, n) };
    let rb = unsafe { raw_bytes(rp, n) };
    assert_eq!(cb, rb, "bytes: start={start_count} op={op} a={a} b={b}");
    unsafe {
        free(cp as *mut c_void);
        free(rp as *mut c_void);
    }
}

#[test]
fn row19_pcwh_count_zero_all_valid_ops() {
    let mut rng = Rng::new(0x19_19);
    for op in VALID_OPS {
        for _ in 0..200 {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }
            diff_pcwh_at(0, op, a, b);
        }
    }
}

#[test]
fn row20_pcwh_mid_buffer_counts() {
    let mut rng = Rng::new(0x20_20);
    for start in 1..=9 {
        for op in VALID_OPS {
            for _ in 0..40 {
                let a = rng.next_i32_mixed();
                let mut b = rng.next_i32_mixed();
                if a == c_int::MIN && b == -1 {
                    b = 3;
                }
                diff_pcwh_at(start, op, a, b);
            }
        }
    }
}

#[test]
fn row21_pcwh_last_legal_slot() {
    let mut rng = Rng::new(0x21_21);
    for op in VALID_OPS {
        for _ in 0..100 {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }
            diff_pcwh_at(9, op, a, b);
        }
    }
}

#[test]
fn row22_pcwh_default_op_bucket() {
    let mut rng = Rng::new(0x22_22);
    for op in DEFAULT_BUCKET {
        for start in [0, 3, 9] {
            for _ in 0..60 {
                let a = rng.next_i32_mixed();
                let b = rng.next_i32_mixed();
                diff_pcwh_at(start, op, a, b);
            }
        }
    }
}

#[test]
fn row23_pcwh_divide_modulo_by_zero_recorded() {
    for op in [4, 5] {
        for &a in BOUNDARIES.iter() {
            diff_pcwh_at(0, op, a, 0);
            diff_pcwh_at(7, op, a, 0);
        }
    }
}

// --------------------------------------------------------------------------
// Row 24 — a full sequence past the cap on a single buffer.
// --------------------------------------------------------------------------
#[test]
fn row24_pcwh_sequence_past_capacity() {
    let mut rng = Rng::new(0x24_24);
    for trial in 0..30 {
        let n = 10usize;
        let cp = unsafe { (c().allocate_results)(n as c_int) };
        let rp = unsafe { (r().allocate_results)(n as c_int) };
        let mut ch = cp;
        let mut rh = rp;
        let mut cc: c_int = 0;
        let mut rc: c_int = 0;

        for step in 0..12 {
            let op = if step % 3 == 0 {
                rng.next_i32_mixed()
            } else {
                rng.range(1, 5) as c_int
            };
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }
            let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
            let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };
            assert_eq!(cv, rv, "trial {trial} step {step}: ret (op={op} a={a} b={b})");
            assert_eq!(cc, rc, "trial {trial} step {step}: count");
            assert_eq!(cc, (step + 1).min(10), "trial {trial} step {step}: count caps at 10");
            let cb = unsafe { raw_bytes(cp, n) };
            let rb = unsafe { raw_bytes(rp, n) };
            assert_eq!(cb, rb, "trial {trial} step {step}: buffer bytes");
        }
        unsafe {
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}
