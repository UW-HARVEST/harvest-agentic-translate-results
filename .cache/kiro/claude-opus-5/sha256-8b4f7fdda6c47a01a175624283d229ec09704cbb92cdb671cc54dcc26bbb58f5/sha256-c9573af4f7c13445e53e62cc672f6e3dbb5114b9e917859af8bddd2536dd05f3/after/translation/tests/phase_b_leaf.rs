//! Phase B — valid-path differential tests, leaf entry points.
//!
//! Covers `CONFIGS.md` rows 1..14. Both libraries are loaded through
//! `libloading`; no Rust function is called directly.

mod common;

use common::*;
use std::ffi::c_int;

/// Values every arithmetic row is probed with in addition to the randomized
/// draws.
const BOUNDARIES: [c_int; 13] = [
    0,
    1,
    -1,
    2,
    -2,
    5,
    -5,
    127,
    128,
    -128,
    i32::MAX,
    i32::MIN,
    i32::MIN + 1,
];

const N_RAND: usize = 4000;

fn pair() -> Pair {
    Pair::new()
}

// ---------------------------------------------------------------------------
// Row 1 — add_operation
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_add_operation() {
    let p = pair();
    let mut n = 0usize;
    for &a in &BOUNDARIES {
        for &b in &BOUNDARIES {
            for &u in &[0, 1, -1, i32::MAX, i32::MIN] {
                assert_eq!(
                    p.c.add_operation(a, b, u),
                    p.r.add_operation(a, b, u),
                    "add_operation({a}, {b}, {u})"
                );
                n += 1;
            }
        }
    }
    let mut rng = Rng::new(SEED);
    for _ in 0..N_RAND {
        let (a, b, u) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32());
        assert_eq!(
            p.c.add_operation(a, b, u),
            p.r.add_operation(a, b, u),
            "add_operation({a}, {b}, {u})"
        );
        n += 1;
    }
    assert!(n >= N_RAND, "ran {n} cases");
}

// ---------------------------------------------------------------------------
// Row 2 — multiply_operation
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_multiply_operation() {
    let p = pair();
    for &a in &BOUNDARIES {
        for &b in &BOUNDARIES {
            for &u in &[0, 7, -7] {
                assert_eq!(
                    p.c.multiply_operation(a, b, u),
                    p.r.multiply_operation(a, b, u),
                    "multiply_operation({a}, {b}, {u})"
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N_RAND {
        let (a, b, u) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32());
        assert_eq!(
            p.c.multiply_operation(a, b, u),
            p.r.multiply_operation(a, b, u),
            "multiply_operation({a}, {b}, {u})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 — subtract_operation
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_subtract_operation() {
    let p = pair();
    for &a in &BOUNDARIES {
        for &b in &BOUNDARIES {
            for &u in &[0, 3, -3] {
                assert_eq!(
                    p.c.subtract_operation(a, b, u),
                    p.r.subtract_operation(a, b, u),
                    "subtract_operation({a}, {b}, {u})"
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N_RAND {
        let (a, b, u) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32());
        assert_eq!(
            p.c.subtract_operation(a, b, u),
            p.r.subtract_operation(a, b, u),
            "subtract_operation({a}, {b}, {u})"
        );
    }
}

/// `INT_MIN / -1` and `INT_MIN % -1` both trap; they are covered separately in
/// Phase C via a child process, so the in-process rows exclude that one pair.
fn is_trapping(a: c_int, b: c_int) -> bool {
    a == i32::MIN && b == -1
}

// ---------------------------------------------------------------------------
// Row 4 — divide_operation, b != 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_divide_operation_nonzero_divisor() {
    let p = pair();
    for &a in &BOUNDARIES {
        for &b in &BOUNDARIES {
            if b == 0 || is_trapping(a, b) {
                continue;
            }
            assert_eq!(
                p.c.divide_operation(a, b, 0),
                p.r.divide_operation(a, b, 0),
                "divide_operation({a}, {b})"
            );
        }
    }
    // Explicit truncation-toward-zero probes with mixed signs.
    for &(a, b) in &[
        (7, 2),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (1, i32::MAX),
        (i32::MIN, 2),
        (i32::MIN, -2),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (-1, i32::MIN),
    ] {
        assert_eq!(
            p.c.divide_operation(a, b, 0),
            p.r.divide_operation(a, b, 0),
            "divide_operation({a}, {b})"
        );
    }
    let mut rng = Rng::new(SEED ^ 4);
    let mut done = 0;
    while done < N_RAND {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if b == 0 || is_trapping(a, b) {
            continue;
        }
        let u = rng.next_i32();
        assert_eq!(
            p.c.divide_operation(a, b, u),
            p.r.divide_operation(a, b, u),
            "divide_operation({a}, {b}, {u})"
        );
        done += 1;
    }
}

// ---------------------------------------------------------------------------
// Row 5 — divide_operation, b == 0 sentinel
// ---------------------------------------------------------------------------
#[test]
fn cfg_05_divide_operation_zero_divisor() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N_RAND {
        let a = if i < BOUNDARIES.len() { BOUNDARIES[i] } else { rng.next_i32_mixed() };
        let c = p.c.divide_operation(a, 0, 0);
        let r = p.r.divide_operation(a, 0, 0);
        assert_eq!(c, r, "divide_operation({a}, 0)");
        assert_eq!(c, 0, "C sentinel for divide by zero must be 0");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — modulo_operation, b != 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_06_modulo_operation_nonzero_divisor() {
    let p = pair();
    for &a in &BOUNDARIES {
        for &b in &BOUNDARIES {
            if b == 0 || is_trapping(a, b) {
                continue;
            }
            assert_eq!(
                p.c.modulo_operation(a, b, 0),
                p.r.modulo_operation(a, b, 0),
                "modulo_operation({a}, {b})"
            );
        }
    }
    for &(a, b) in &[
        (7, 5),
        (-7, 5),
        (7, -5),
        (-7, -5),
        (i32::MIN, 5),
        (i32::MIN, -5),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (0, -1),
    ] {
        assert_eq!(
            p.c.modulo_operation(a, b, 0),
            p.r.modulo_operation(a, b, 0),
            "modulo_operation({a}, {b})"
        );
    }
    let mut rng = Rng::new(SEED ^ 6);
    let mut done = 0;
    while done < N_RAND {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if b == 0 || is_trapping(a, b) {
            continue;
        }
        let u = rng.next_i32();
        assert_eq!(
            p.c.modulo_operation(a, b, u),
            p.r.modulo_operation(a, b, u),
            "modulo_operation({a}, {b}, {u})"
        );
        done += 1;
    }
}

// ---------------------------------------------------------------------------
// Row 7 — modulo_operation, b == 0 sentinel
// ---------------------------------------------------------------------------
#[test]
fn cfg_07_modulo_operation_zero_divisor() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N_RAND {
        let a = if i < BOUNDARIES.len() { BOUNDARIES[i] } else { rng.next_i32_mixed() };
        let c = p.c.modulo_operation(a, 0, 0);
        let r = p.r.modulo_operation(a, 0, 0);
        assert_eq!(c, r, "modulo_operation({a}, 0)");
        assert_eq!(c, 0, "C sentinel for modulo by zero must be 0");
    }
}

// ---------------------------------------------------------------------------
// Row 8 — is_valid_operation over all 256 char bit patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_08_is_valid_operation_exhaustive() {
    let p = pair();
    for byte in 0u16..256 {
        let ch = byte as u8 as i8; // `char` is signed on this target
        let c = p.c.is_valid_operation(ch);
        let r = p.r.is_valid_operation(ch);
        assert_eq!(c, r, "is_valid_operation({ch}) [0x{byte:02x}]");
        // Cross-check against the C predicate spelled out.
        let expected = ch != 0 && ch >= b'1' as i8 && ch <= b'5' as i8;
        assert_eq!(c, expected, "C predicate mismatch for {ch}");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — get_operation_priority, in-range ops
// ---------------------------------------------------------------------------
#[test]
fn cfg_09_get_operation_priority_in_range() {
    let p = pair();
    for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
        let c = p.c.get_operation_priority(op);
        assert_eq!(c, p.r.get_operation_priority(op), "get_operation_priority({op})");
        assert_eq!(c, op * 10);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — get_operation_priority, out-of-range / overflowing ops
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_get_operation_priority_out_of_range() {
    let p = pair();
    for op in [0, 6, 7, -1, -2, -3, -4, 100, i32::MAX, i32::MIN, i32::MAX / 10, i32::MAX / 10 + 1] {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.r.get_operation_priority(op),
            "get_operation_priority({op})"
        );
    }
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N_RAND {
        let op = rng.next_i32_mixed();
        assert_eq!(
            p.c.get_operation_priority(op),
            p.r.get_operation_priority(op),
            "get_operation_priority({op})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 11 — get_computation_timestamp
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_get_computation_timestamp() {
    let p = pair();
    for _ in 0..64 {
        let c = p.c.get_computation_timestamp();
        let r = p.r.get_computation_timestamp();
        assert_eq!(c, r, "get_computation_timestamp() must agree (time() >> 29)");
        // Sanity: the shift really is applied.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(c, now >> 29, "timestamp is not time() >> 29");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — allocate_results, valid counts: non-NULL and fully zeroed
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_allocate_results_valid_counts() {
    let p = pair();
    for count in [1, 2, 9, 10, 11, 100, 1000] {
        let pc = p.c.allocate_results(count);
        let pr = p.r.allocate_results(count);
        assert!(!pc.is_null(), "C allocate_results({count}) returned NULL");
        assert!(!pr.is_null(), "Rust allocate_results({count}) returned NULL");
        assert_eq!(pc as usize % 8, 0, "C buffer must be 8-byte aligned");
        assert_eq!(pr as usize % 8, 0, "Rust buffer must be 8-byte aligned");
        let cb = unsafe { read_history_bytes(pc, count as usize) };
        let rb = unsafe { read_history_bytes(pr, count as usize) };
        assert_bytes_eq(&format!("allocate_results({count})"), &cb, &rb);
        assert!(cb.iter().all(|&b| b == 0), "calloc must zero all {} bytes", cb.len());
    }
}

// ---------------------------------------------------------------------------
// Row 13 — select_operation, in-range ops: identity + behaviour
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_select_operation_in_range() {
    let p = pair();
    let expect = [
        (OP_ADD, "add_operation"),
        (OP_MULTIPLY, "multiply_operation"),
        (OP_SUBTRACT, "subtract_operation"),
        (OP_DIVIDE, "divide_operation"),
        (OP_MODULO, "modulo_operation"),
    ];
    let mut rng = Rng::new(SEED ^ 13);
    for (op, name) in expect {
        let fc = p.c.select_operation(op);
        let fr = p.r.select_operation(op);
        assert!(!fc.is_null() && !fr.is_null(), "select_operation({op}) returned NULL");
        assert_eq!(p.c.identify_mathfn(fc), name, "C select_operation({op})");
        assert_eq!(p.r.identify_mathfn(fr), name, "Rust select_operation({op})");
        for _ in 0..500 {
            let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (op == OP_DIVIDE || op == OP_MODULO) && is_trapping(a, b) {
                continue;
            }
            let u = rng.next_i32();
            let rc = unsafe { p.c.call_mathfn(fc, a, b, u) };
            let rr = unsafe { p.r.call_mathfn(fr, a, b, u) };
            assert_eq!(rc, rr, "{name} via select_operation({op}) on ({a}, {b}, {u})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — select_operation, out-of-range ops fall back to add_operation
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_select_operation_out_of_range() {
    let p = pair();
    let mut ops: Vec<c_int> =
        vec![0, 6, 7, 8, -1, -2, -3, -4, -5, 100, i32::MAX, i32::MIN, i32::MAX - 1];
    let mut rng = Rng::new(SEED ^ 14);
    while ops.len() < 300 {
        let op = rng.next_i32();
        if !(OP_ADD..=OP_MODULO).contains(&op) {
            ops.push(op);
        }
    }
    for op in ops {
        let fc = p.c.select_operation(op);
        let fr = p.r.select_operation(op);
        assert!(!fc.is_null(), "C select_operation({op}) returned NULL, expected add_operation");
        assert!(!fr.is_null(), "Rust select_operation({op}) returned NULL");
        assert_eq!(p.c.identify_mathfn(fc), "add_operation", "C default: for op={op}");
        assert_eq!(p.r.identify_mathfn(fr), "add_operation", "Rust default: for op={op}");
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let rc = unsafe { p.c.call_mathfn(fc, a, b, 0) };
        let rr = unsafe { p.r.call_mathfn(fr, a, b, 0) };
        assert_eq!(rc, rr, "fallback op={op} on ({a}, {b})");
        assert_eq!(rc, a.wrapping_add(b), "fallback must be addition");
    }
}
