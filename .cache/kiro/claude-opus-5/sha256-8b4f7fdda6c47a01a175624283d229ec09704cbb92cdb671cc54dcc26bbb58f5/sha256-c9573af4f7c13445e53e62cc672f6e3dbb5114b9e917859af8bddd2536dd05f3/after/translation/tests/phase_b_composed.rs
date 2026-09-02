//! Phase B — valid-path differential tests for the composed entry points.
//!
//! Covers `CONFIGS.md` rows 15..32: `perform_computation_with_history` driven
//! directly (the low-level entry point, not just the `mathop` wrapper), the
//! full `mathop` pipeline, the composed `select_operation` -> fn-pointer ->
//! history path, and the `ComputationResult` ABI.

mod common;

use common::*;
use std::ffi::c_int;

const ALL_OPS: [c_int; 5] = [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO];

fn is_trapping(a: c_int, b: c_int) -> bool {
    a == i32::MIN && b == -1
}

/// One `perform_computation_with_history` call against a caller-owned buffer,
/// on both libraries, comparing the return value, the resulting count, and the
/// whole buffer byte-for-byte.
struct HistPair {
    cbuf: HistoryBuf,
    rbuf: HistoryBuf,
    ccount: c_int,
    rcount: c_int,
}

impl HistPair {
    fn new(len: usize, start_count: c_int) -> Self {
        HistPair {
            cbuf: HistoryBuf::zeroed(len),
            rbuf: HistoryBuf::zeroed(len),
            ccount: start_count,
            rcount: start_count,
        }
    }

    fn step(&mut self, p: &Pair, a: c_int, b: c_int, op: c_int, ctx: &str) {
        let mut cptr = self.cbuf.as_mut_ptr();
        let mut rptr = self.rbuf.as_mut_ptr();
        let rc = unsafe {
            p.c.perform_computation_with_history(a, b, op, &mut cptr, &mut self.ccount)
        };
        let rr = unsafe {
            p.r.perform_computation_with_history(a, b, op, &mut rptr, &mut self.rcount)
        };
        assert_eq!(rc, rr, "{ctx}: return value for pcwh({a}, {b}, {op})");
        assert_eq!(self.ccount, self.rcount, "{ctx}: history_count after pcwh({a}, {b}, {op})");
        assert_eq!(cptr, self.cbuf.as_mut_ptr(), "{ctx}: C must not replace a non-NULL history");
        assert_eq!(rptr, self.rbuf.as_mut_ptr(), "{ctx}: Rust must not replace a non-NULL history");
        assert_bytes_eq(
            &format!("{ctx}: pcwh({a}, {b}, {op})"),
            self.cbuf.bytes(),
            self.rbuf.bytes(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 — bootstrap path (*history == NULL): the library callocs 10 slots
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_pcwh_bootstrap_each_op() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 15);
    for op in ALL_OPS {
        for _ in 0..200 {
            let (mut a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (op == OP_DIVIDE || op == OP_MODULO) && is_trapping(a, b) {
                b = 3;
            }
            // A non-zero starting count must be reset to 0 by the bootstrap.
            let mut ccount: c_int = rng.next_i32_mixed().rem_euclid(1000);
            let mut rcount: c_int = ccount;
            let mut chist: *mut ComputationResult = std::ptr::null_mut();
            let mut rhist: *mut ComputationResult = std::ptr::null_mut();

            let rc = unsafe {
                p.c.perform_computation_with_history(a, b, op, &mut chist, &mut ccount)
            };
            let rr = unsafe {
                p.r.perform_computation_with_history(a, b, op, &mut rhist, &mut rcount)
            };
            assert_eq!(rc, rr, "bootstrap pcwh({a}, {b}, {op}) return value");
            assert!(!chist.is_null(), "C must allocate a history on bootstrap");
            assert!(!rhist.is_null(), "Rust must allocate a history on bootstrap");
            assert_eq!(ccount, rcount, "bootstrap must reset then increment history_count");
            assert_eq!(ccount, 1, "bootstrap writes slot 0 and leaves count == 1");

            let cb = unsafe { read_history_bytes(chist, HISTORY_CAPACITY) };
            let rb = unsafe { read_history_bytes(rhist, HISTORY_CAPACITY) };
            assert_bytes_eq(&format!("bootstrap pcwh({a}, {b}, {op})"), &cb, &rb);
            // Slots 1..10 must still be the zeroes calloc produced.
            assert!(
                cb[RESULT_SIZE..].iter().all(|&x| x == 0),
                "unused bootstrap slots must stay zero"
            );
            a = a.wrapping_add(1);
            let _ = a;
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — caller-allocated history, appending from count == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_pcwh_caller_buffer_appends() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 16);
    for trial in 0..200 {
        let mut h = HistPair::new(HISTORY_CAPACITY, 0);
        for step in 0..9 {
            let op = ALL_OPS[step % ALL_OPS.len()];
            let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (op == OP_DIVIDE || op == OP_MODULO) && is_trapping(a, b) {
                b = -7;
            }
            h.step(&p, a, b, op, &format!("trial {trial} step {step}"));
            assert_eq!(h.ccount as usize, step + 1, "count must advance one per append");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — caller-allocated history already exactly full (count == 10)
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_pcwh_count_exactly_capacity() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 17);
    for op in ALL_OPS {
        for _ in 0..100 {
            let mut h = HistPair::new(HISTORY_CAPACITY, HISTORY_CAPACITY as c_int);
            let before = h.cbuf.bytes().to_vec();
            let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (op == OP_DIVIDE || op == OP_MODULO) && is_trapping(a, b) {
                b = 11;
            }
            h.step(&p, a, b, op, "count == 10");
            assert_eq!(h.ccount, HISTORY_CAPACITY as c_int, "count must not advance past 10");
            assert_eq!(h.cbuf.bytes(), &before[..], "no slot may be written when full");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — caller-allocated history over capacity (count > 10)
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_pcwh_count_over_capacity() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 18);
    for start in [11i32, 12, 100, 1000, i32::MAX] {
        for op in ALL_OPS {
            let mut h = HistPair::new(HISTORY_CAPACITY, start);
            let before = h.cbuf.bytes().to_vec();
            let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (op == OP_DIVIDE || op == OP_MODULO) && is_trapping(a, b) {
                b = 13;
            }
            h.step(&p, a, b, op, &format!("count == {start}"));
            assert_eq!(h.ccount, start, "count must be untouched when >= 10");
            assert_eq!(h.cbuf.bytes(), &before[..], "no slot may be written when over capacity");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — out-of-range op through pcwh (bootstrap and caller buffer)
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_pcwh_out_of_range_op() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 19);
    let mut ops: Vec<c_int> = vec![0, 6, 7, -1, -2, -3, -4, 99, i32::MAX, i32::MIN];
    while ops.len() < 120 {
        let op = rng.next_i32();
        if !(OP_ADD..=OP_MODULO).contains(&op) {
            ops.push(op);
        }
    }
    for op in ops {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());

        // caller buffer
        let mut h = HistPair::new(HISTORY_CAPACITY, 0);
        h.step(&p, a, b, op, &format!("out-of-range op {op}"));
        assert_eq!(h.cbuf.slot(0).value, a.wrapping_add(b), "default: must be addition");
        assert_eq!(h.cbuf.slot(0).status, 0, "status must be STATUS_SUCCESS");

        // bootstrap
        let mut chist: *mut ComputationResult = std::ptr::null_mut();
        let mut rhist: *mut ComputationResult = std::ptr::null_mut();
        let (mut cc, mut rc_) = (0, 0);
        let rc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut chist, &mut cc) };
        let rr = unsafe { p.r.perform_computation_with_history(a, b, op, &mut rhist, &mut rc_) };
        assert_eq!(rc, rr, "bootstrap out-of-range op {op}");
        assert_eq!(rc, a.wrapping_add(b));
        assert_bytes_eq(
            &format!("bootstrap out-of-range op {op}"),
            &unsafe { read_history_bytes(chist, HISTORY_CAPACITY) },
            &unsafe { read_history_bytes(rhist, HISTORY_CAPACITY) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 20 — divide/modulo by zero recorded through pcwh
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_pcwh_divide_modulo_by_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 20);
    for op in [OP_DIVIDE, OP_MODULO] {
        for _ in 0..200 {
            let a = rng.next_i32_mixed();
            let mut h = HistPair::new(HISTORY_CAPACITY, 0);
            h.step(&p, a, 0, op, &format!("op {op} with b == 0"));
            assert_eq!(h.cbuf.slot(0).value, 0, "the b==0 sentinel 0 must be recorded");
            assert_eq!(h.cbuf.slot(0).status, 0);
            assert_eq!(h.ccount, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — a whole bootstrap-fill-saturate sequence, compared at every step
// ---------------------------------------------------------------------------
#[test]
fn cfg_21_pcwh_full_sequence_to_saturation() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 21);
    for trial in 0..100 {
        let mut chist: *mut ComputationResult = std::ptr::null_mut();
        let mut rhist: *mut ComputationResult = std::ptr::null_mut();
        let mut cc: c_int = 0;
        let mut rc_: c_int = 0;
        // 14 steps: 10 fill the history, the last 4 must be dropped.
        for step in 0..14 {
            let op = rng.next_i32_mixed();
            let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            let effective = if (OP_ADD..=OP_MODULO).contains(&op) { op } else { OP_ADD };
            if (effective == OP_DIVIDE || effective == OP_MODULO) && is_trapping(a, b) {
                b = 17;
            }
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut chist, &mut cc) };
            let vr =
                unsafe { p.r.perform_computation_with_history(a, b, op, &mut rhist, &mut rc_) };
            let ctx = format!("trial {trial} step {step} op {op}");
            assert_eq!(vc, vr, "{ctx}: return value");
            assert_eq!(cc, rc_, "{ctx}: history_count");
            assert_eq!(
                cc as usize,
                (step + 1).min(HISTORY_CAPACITY),
                "{ctx}: count must saturate at 10"
            );
            assert_bytes_eq(
                &ctx,
                &unsafe { read_history_bytes(chist, HISTORY_CAPACITY) },
                &unsafe { read_history_bytes(rhist, HISTORY_CAPACITY) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — mathop with param3 selecting each in-range operation
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_mathop_param3_selects_each_op() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 22);
    for base in 0..5i32 {
        for k in 0..10i32 {
            let param3 = base + 5 * k;
            assert_eq!(param3 % 5 + 1, base + 1, "param3 must select op {}", base + 1);
            for _ in 0..100 {
                let (p1, p2, p4) =
                    (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
                if mathop_traps(p1, p2, param3, p4) {
                    continue;
                }
                assert_eq!(
                    p.c.mathop(p1, p2, param3, p4),
                    p.r.mathop(p1, p2, param3, p4),
                    "mathop({p1}, {p2}, {param3}, {p4})"
                );
            }
        }
    }
}

/// Does this `mathop` argument tuple reach an `INT_MIN / -1` (trapping) `idiv`?
///
/// Derived from `mathop`'s own selection arithmetic:
///
/// * first computation:  `op = param3 % 5 + 1`, operands `(param1, param2)`
/// * second computation: `op = (param4 + 1) % 5 + 1`, operands
///   `(intermediate, param4)`
///
/// The second computation can never trap: it only divides when
/// `(param4 + 1) % 5 + 1` is 4 or 5, i.e. `(param4 + 1) % 5` is 3 or 4, while
/// trapping additionally requires `param4 == -1`, which gives
/// `(0) % 5 + 1 == 1` (addition). So the *only* trapping tuple is a divide or
/// modulo first computation with `param1 == INT_MIN && param2 == -1`.
fn mathop_traps(p1: c_int, p2: c_int, p3: c_int, _p4: c_int) -> bool {
    let selected_op = p3.wrapping_rem(5).wrapping_add(1);
    (selected_op == OP_DIVIDE || selected_op == OP_MODULO) && p1 == i32::MIN && p2 == -1
}

// ---------------------------------------------------------------------------
// Row 23 — mathop with negative param3 (out-of-range selected_op)
// ---------------------------------------------------------------------------
#[test]
fn cfg_23_mathop_negative_param3() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 23);
    for param3 in [-1i32, -2, -3, -4, -5, -6, -7, -10, -100, i32::MIN, i32::MIN + 1] {
        let op = param3 % 5 + 1;
        assert!(
            !(OP_ADD..=OP_MODULO).contains(&op) || op == OP_ADD,
            "negative param3 {param3} yields op {op}"
        );
        for _ in 0..200 {
            let (p1, p2, p4) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
            if mathop_traps(p1, p2, param3, p4) {
                continue;
            }
            assert_eq!(
                p.c.mathop(p1, p2, param3, p4),
                p.r.mathop(p1, p2, param3, p4),
                "mathop({p1}, {p2}, {param3}, {p4})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — mathop: param4 drives second_op (incl. INT_MAX overflow)
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_mathop_param4_selects_second_op() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 24);
    let mut param4s: Vec<c_int> = vec![i32::MAX, i32::MIN, 0, -1, -2, -3, -4, -5, -6];
    for base in 0..5i32 {
        for k in 0..4i32 {
            param4s.push(base + 5 * k);
        }
    }
    for param4 in param4s {
        for _ in 0..100 {
            let (p1, p2, p3) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
            if mathop_traps(p1, p2, p3, param4) {
                continue;
            }
            assert_eq!(
                p.c.mathop(p1, p2, p3, param4),
                p.r.mathop(p1, p2, p3, param4),
                "mathop({p1}, {p2}, {p3}, {param4})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 25 / 26 — mathop with a valid vs invalid validation char
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_26_mathop_validation_char_classes() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 25);

    // Valid: (char)(param1 % 128) in '1'..='5'  (49..=53)
    let mut valid: Vec<c_int> = Vec::new();
    for c in 49..=53i32 {
        for k in 0..8i32 {
            valid.push(c + 128 * k);
        }
    }
    // Invalid: 0, '0', '6', 127, and negatives (char is signed).
    let mut invalid: Vec<c_int> = vec![0, 48, 54, 127, 128, 256, -48, -49, -53, -127, i32::MIN];
    for k in 0..8i32 {
        invalid.push(128 * k);
    }

    for (label, params) in [("valid", valid), ("invalid", invalid)] {
        for param1 in params {
            let vc = (param1 % 128) as i8;
            let is_valid = vc != 0 && vc >= b'1' as i8 && vc <= b'5' as i8;
            assert_eq!(is_valid, label == "valid", "param1 {param1} class check (char {vc})");
            assert_eq!(
                p.c.is_valid_operation(vc),
                is_valid,
                "C is_valid_operation disagrees for {vc}"
            );
            for _ in 0..60 {
                let (p2, p3, p4) =
                    (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
                if mathop_traps(param1, p2, p3, p4) {
                    continue;
                }
                assert_eq!(
                    p.c.mathop(param1, p2, p3, p4),
                    p.r.mathop(param1, p2, p3, p4),
                    "{label}: mathop({param1}, {p2}, {p3}, {p4})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27 — divide/modulo-by-zero reached from inside mathop
// ---------------------------------------------------------------------------
#[test]
fn cfg_27_mathop_inner_divide_by_zero() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 27);

    // selected_op == OP_DIVIDE  <=>  param3 % 5 + 1 == 4  <=>  param3 % 5 == 3
    // selected_op == OP_MODULO  <=>  param3 % 5 == 4
    for (want_op, base) in [(OP_DIVIDE, 3i32), (OP_MODULO, 4i32)] {
        for k in 0..6i32 {
            let param3 = base + 5 * k;
            assert_eq!(param3 % 5 + 1, want_op);
            for _ in 0..200 {
                let p1 = rng.next_i32_mixed();
                let p4 = rng.next_i32_mixed();
                if mathop_traps(p1, 0, param3, p4) {
                    continue;
                }
                // param2 == 0 -> the inner divide/modulo takes its 0 sentinel.
                assert_eq!(
                    p.c.mathop(p1, 0, param3, p4),
                    p.r.mathop(p1, 0, param3, p4),
                    "mathop({p1}, 0, {param3}, {p4})"
                );
            }
        }
    }

    // second_op == OP_DIVIDE  <=>  (param4+1) % 5 + 1 == 4, and param4 == 0
    // means the second computation divides by zero.
    for _ in 0..200 {
        let (p1, p2, p3) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
        assert_eq!(
            p.c.mathop(p1, p2, p3, 0),
            p.r.mathop(p1, p2, p3, 0),
            "mathop({p1}, {p2}, {p3}, 0)"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 28 — mathop boundary cross-product
// ---------------------------------------------------------------------------
#[test]
fn cfg_28_mathop_boundary_cross_product() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let vals: [c_int; 9] = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    let mut cases = 0usize;
    for &p1 in &vals {
        for &p2 in &vals {
            for &p3 in &vals {
                for &p4 in &vals {
                    if mathop_traps(p1, p2, p3, p4) {
                        continue;
                    }
                    assert_eq!(
                        p.c.mathop(p1, p2, p3, p4),
                        p.r.mathop(p1, p2, p3, p4),
                        "mathop({p1}, {p2}, {p3}, {p4})"
                    );
                    cases += 1;
                }
            }
        }
    }
    assert!(cases > 3000, "expected a large cross-product, ran {cases}");
}

// ---------------------------------------------------------------------------
// Row 29 — mathop's static history bootstraps, fills and saturates
// ---------------------------------------------------------------------------
#[test]
fn cfg_29_mathop_static_history_saturation() {
    let _quiet = SilentStdout::new();
    // Fresh handles so each library starts from its own pristine statics.
    let c = Lib::c();
    let r = Lib::rust();
    let mut rng = Rng::new(SEED ^ 29);
    // 12 consecutive calls: calls 1..5 fill the 10 slots, 6..12 saturate.
    for call in 0..12 {
        let (p1, p2, p3, p4) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        if mathop_traps(p1, p2, p3, p4) {
            continue;
        }
        assert_eq!(
            c.mathop(p1, p2, p3, p4),
            r.mathop(p1, p2, p3, p4),
            "call {call}: mathop({p1}, {p2}, {p3}, {p4}) — the static history must not \
             change the return value"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 30 — fully randomized mathop quadruples
// ---------------------------------------------------------------------------
#[test]
fn cfg_30_mathop_randomized() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 30);
    let mut done = 0usize;
    while done < 2000 {
        let (p1, p2, p3, p4) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        if mathop_traps(p1, p2, p3, p4) {
            continue;
        }
        assert_eq!(
            p.c.mathop(p1, p2, p3, p4),
            p.r.mathop(p1, p2, p3, p4),
            "mathop({p1}, {p2}, {p3}, {p4})"
        );
        done += 1;
    }
    // Plus a pass with unbiased full-range draws.
    let mut done = 0usize;
    while done < 2000 {
        let (p1, p2, p3, p4) =
            (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        if mathop_traps(p1, p2, p3, p4) {
            continue;
        }
        assert_eq!(
            p.c.mathop(p1, p2, p3, p4),
            p.r.mathop(p1, p2, p3, p4),
            "mathop({p1}, {p2}, {p3}, {p4})"
        );
        done += 1;
    }
}

// ---------------------------------------------------------------------------
// Row 31 — composed pipeline: select_operation -> fn ptr -> pcwh
// ---------------------------------------------------------------------------
#[test]
fn cfg_31_composed_select_then_history() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 31);
    for trial in 0..200 {
        let mut h = HistPair::new(HISTORY_CAPACITY, 0);
        for step in 0..12 {
            let op = if step % 3 == 0 { rng.next_i32() } else { ALL_OPS[step % 5] };
            let effective = if (OP_ADD..=OP_MODULO).contains(&op) { op } else { OP_ADD };
            let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if (effective == OP_DIVIDE || effective == OP_MODULO) && is_trapping(a, b) {
                b = 19;
            }

            // Stage 1: resolve the operation through the public selector and
            // invoke the returned pointer directly.
            let fc = p.c.select_operation(op);
            let fr = p.r.select_operation(op);
            let dc = unsafe { p.c.call_mathfn(fc, a, b, 0) };
            let dr = unsafe { p.r.call_mathfn(fr, a, b, 0) };
            assert_eq!(dc, dr, "trial {trial} step {step}: direct fn-ptr call op {op}");

            // Stage 2: feed the same inputs through the history recorder and
            // require the recorded value to equal the direct call — but only
            // when the write actually happened (the guard is `count < 10`).
            let before = h.ccount;
            h.step(&p, a, b, op, &format!("composed trial {trial} step {step}"));
            if h.ccount == before + 1 {
                let slot = h.cbuf.slot(before as usize);
                assert_eq!(slot.value, dc, "recorded value must match the direct fn-ptr call");
            } else {
                assert_eq!(h.ccount, before, "count changed without a write");
                assert!(before >= HISTORY_CAPACITY as i32, "no write below capacity");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 32 — ComputationResult ABI, observed through the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn cfg_32_computation_result_abi() {
    assert_eq!(std::mem::size_of::<ComputationResult>(), 24, "sizeof mismatch");
    assert_eq!(std::mem::align_of::<ComputationResult>(), 8, "alignof mismatch");

    let p = Pair::new();
    // A one-slot history: the library must touch bytes 0..4 (value),
    // 8..16 (timestamp) and 16..20 (status), leaving 4..8 and 20..24 zero.
    let mut h = HistPair::new(1, 0);
    h.step(&p, 0x1122_3344, 0x0000_0001, OP_ADD, "abi single slot");
    let bytes = h.cbuf.bytes();
    assert_eq!(bytes.len(), 24);
    assert_eq!(&bytes[4..8], &[0, 0, 0, 0], "padding after `value` must stay zero");
    assert_eq!(&bytes[20..24], &[0, 0, 0, 0], "tail padding must stay zero");
    let slot = h.cbuf.slot(0);
    assert_eq!(slot.value, 0x1122_3345);
    assert_eq!(slot.status, 0);
    assert_eq!(slot.timestamp, p.c.get_computation_timestamp());
    assert_bytes_eq("abi single slot", h.cbuf.bytes(), h.rbuf.bytes());

    // Little-endian field placement, checked byte by byte.
    assert_eq!(&bytes[0..4], &[0x45, 0x33, 0x22, 0x11], "value is a little-endian int32 at +0");
}
