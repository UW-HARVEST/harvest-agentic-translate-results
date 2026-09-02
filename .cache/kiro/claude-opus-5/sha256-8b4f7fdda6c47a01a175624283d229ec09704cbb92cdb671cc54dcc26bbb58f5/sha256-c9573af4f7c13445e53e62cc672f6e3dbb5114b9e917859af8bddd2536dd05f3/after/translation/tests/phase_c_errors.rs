//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows whose C behaviour is a fatal signal
//! (`SIGFPE` from `INT_MIN / -1`, `SIGSEGV` from a NULL out-param) are run in a
//! child process so that *how the process died* can be compared instead of just
//! "something went wrong".

mod common;

use common::*;
use std::ffi::c_int;

const SIGFPE: i32 = 8;
const SIGSEGV: i32 = 11;

// ---------------------------------------------------------------------------
// Child-process entry point for the crashing rows.
//
// Marked `#[ignore]` so a normal `cargo test` run never executes it; the parent
// re-invokes this binary with `--ignored --exact crash_child_entry` and
// `HARVEST_CRASH_SCENARIO=<scenario>:<c|rust>`.
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn crash_child_entry() {
    let spec = match std::env::var(CRASH_ENV) {
        Ok(s) => s,
        Err(_) => return, // invoked without a scenario: nothing to do
    };
    let (scenario, which) = spec.split_once(':').expect("scenario must be `name:lib`");
    let lib = match which {
        "c" => Lib::c(),
        "rust" => Lib::rust(),
        other => panic!("unknown library selector {other}"),
    };
    // Volatile-ish sink: the value is printed so nothing can be optimised out.
    // (Every call goes through a `dlsym`ed pointer anyway.)
    let sink: i32 = match scenario {
        // ERRORS rows 6 / 7 — INT_MIN / -1 and INT_MIN % -1 reach `idiv`.
        "div_intmin" => lib.divide_operation(i32::MIN, -1, 0),
        "mod_intmin" => lib.modulo_operation(i32::MIN, -1, 0),
        // ERRORS row 16 — NULL out-parameters are unchecked.
        "null_history" => unsafe {
            let mut count: c_int = 0;
            lib.perform_computation_with_history(1, 2, OP_ADD, std::ptr::null_mut(), &mut count)
        },
        "null_count" => unsafe {
            let mut buf = HistoryBuf::zeroed(HISTORY_CAPACITY);
            let mut hist = buf.as_mut_ptr();
            lib.perform_computation_with_history(1, 2, OP_ADD, &mut hist, std::ptr::null_mut())
        },
        "null_both" => unsafe {
            lib.perform_computation_with_history(
                1,
                2,
                OP_ADD,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        // ERRORS row 24 — the trap propagates out of `mathop`.
        // param3 == 3 -> selected_op == 3 % 5 + 1 == OP_DIVIDE.
        "mathop_div_fpe" => {
            let _quiet = SilentStdout::new();
            lib.mathop(i32::MIN, -1, 3, 7)
        }
        // param3 == 4 -> selected_op == OP_MODULO.
        "mathop_mod_fpe" => {
            let _quiet = SilentStdout::new();
            lib.mathop(i32::MIN, -1, 4, 7)
        }
        // Control: a scenario that must NOT crash, proving the harness can
        // tell a clean exit from a signal.
        "control_ok" => lib.divide_operation(i32::MIN, 1, 0),
        other => panic!("unknown scenario {other}"),
    };
    // Keep the value alive.
    std::hint::black_box(sink);
}

/// Runs one scenario on both libraries and requires identical termination.
fn assert_same_death(scenario: &str, expected: Outcome) {
    let c = run_crash_child(scenario, "c");
    let r = run_crash_child(scenario, "rust");
    assert_eq!(c, expected, "C outcome for scenario `{scenario}`");
    assert_eq!(r, expected, "Rust outcome for scenario `{scenario}`");
    assert_eq!(c, r, "C and Rust must terminate identically for `{scenario}`");
}

// ---------------------------------------------------------------------------
// Rows 1-3 — is_valid_operation rejections
// ---------------------------------------------------------------------------
#[test]
fn err_01_03_is_valid_operation_rejections() {
    let p = Pair::new();

    // Row 1: the NUL short-circuit.
    assert!(!p.c.is_valid_operation(0), "C must reject '\\0'");
    assert_eq!(p.r.is_valid_operation(0), p.c.is_valid_operation(0), "row 1: op_char == 0");

    // Row 2: below '1' (0x31), including negatives — `char` is signed here.
    for ch in [-128i8, -127, -100, -1, 1, 9, 32, 47, 48] {
        let (c, r) = (p.c.is_valid_operation(ch), p.r.is_valid_operation(ch));
        assert_eq!(c, r, "row 2: is_valid_operation({ch})");
        assert!(!c, "row 2: C must reject {ch} (< '1')");
    }

    // Row 3: above '5' (0x35).
    for ch in [54i8, 55, 65, 90, 97, 122, 126, 127] {
        let (c, r) = (p.c.is_valid_operation(ch), p.r.is_valid_operation(ch));
        assert_eq!(c, r, "row 3: is_valid_operation({ch})");
        assert!(!c, "row 3: C must reject {ch} (> '5')");
    }

    // Accepted band, for contrast.
    for ch in 49i8..=53 {
        assert!(p.c.is_valid_operation(ch), "C must accept {ch}");
        assert!(p.r.is_valid_operation(ch), "Rust must accept {ch}");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — divide_operation, b == 0 -> sentinel 0
// ---------------------------------------------------------------------------
#[test]
fn err_04_divide_by_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x04);
    for a in [0i32, 1, -1, i32::MIN, i32::MAX, 42, -42] {
        let (c, r) = (p.c.divide_operation(a, 0, 0), p.r.divide_operation(a, 0, 0));
        assert_eq!(c, 0, "row 4: C sentinel must be exactly 0 for a={a}");
        assert_eq!(c, r, "row 4: divide_operation({a}, 0)");
    }
    for _ in 0..2000 {
        let a = rng.next_i32();
        assert_eq!(p.c.divide_operation(a, 0, 0), p.r.divide_operation(a, 0, 0));
    }
    // The scenario really is survivable: nothing crashed getting here.
    assert_eq!(run_crash_child("control_ok", "c"), Outcome::Exited(0));
}

// ---------------------------------------------------------------------------
// Row 5 — modulo_operation, b == 0 -> sentinel 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_modulo_by_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x05);
    for a in [0i32, 1, -1, i32::MIN, i32::MAX, 42, -42] {
        let (c, r) = (p.c.modulo_operation(a, 0, 0), p.r.modulo_operation(a, 0, 0));
        assert_eq!(c, 0, "row 5: C sentinel must be exactly 0 for a={a}");
        assert_eq!(c, r, "row 5: modulo_operation({a}, 0)");
    }
    for _ in 0..2000 {
        let a = rng.next_i32();
        assert_eq!(p.c.modulo_operation(a, 0, 0), p.r.modulo_operation(a, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// Rows 6 / 7 — INT_MIN / -1 and INT_MIN % -1 must trap identically
// ---------------------------------------------------------------------------
#[test]
fn err_06_07_intmin_div_neg1_sigfpe() {
    // Control first: the child mechanism can observe a clean exit.
    assert_same_death("control_ok", Outcome::Exited(0));
    // Row 6
    assert_same_death("div_intmin", Outcome::Signal(SIGFPE));
    // Row 7
    assert_same_death("mod_intmin", Outcome::Signal(SIGFPE));
}

// ---------------------------------------------------------------------------
// Row 8 — select_operation out of range returns add_operation, never NULL
// ---------------------------------------------------------------------------
#[test]
fn err_08_select_operation_out_of_range() {
    let p = Pair::new();
    let mut probes: Vec<c_int> = vec![0, 6, -1, -2, -3, -5, i32::MIN, i32::MAX, i32::MIN + 1, 1000];
    let mut rng = Rng::new(SEED ^ 0x08);
    while probes.len() < 500 {
        let op = rng.next_i32();
        if !(OP_ADD..=OP_MODULO).contains(&op) {
            probes.push(op);
        }
    }
    for op in probes {
        let fc = p.c.select_operation(op);
        let fr = p.r.select_operation(op);
        assert!(!fc.is_null(), "row 8: C must not return NULL for op {op}");
        assert!(!fr.is_null(), "row 8: Rust must not return NULL for op {op}");
        assert_eq!(p.c.identify_mathfn(fc), "add_operation", "row 8: C default for op {op}");
        assert_eq!(p.r.identify_mathfn(fr), "add_operation", "row 8: Rust default for op {op}");
        // Same rejection => same observable behaviour.
        assert_eq!(
            unsafe { p.c.call_mathfn(fc, 100, 7, 0) },
            unsafe { p.r.call_mathfn(fr, 100, 7, 0) },
            "row 8: fallback behaviour for op {op}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 — allocate_results with a negative count -> NULL
// ---------------------------------------------------------------------------
#[test]
fn err_09_allocate_results_negative() {
    let p = Pair::new();
    for count in [-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1] {
        let pc = p.c.allocate_results(count);
        let pr = p.r.allocate_results(count);
        assert!(pc.is_null(), "row 9: C allocate_results({count}) must be NULL, got {pc:?}");
        assert!(pr.is_null(), "row 9: Rust allocate_results({count}) must be NULL, got {pr:?}");
        assert_eq!(pc.is_null(), pr.is_null(), "row 9: same NULL sentinel for {count}");
    }
}

// ---------------------------------------------------------------------------
// Row 10 — allocate_results with an oversized count -> NULL
// ---------------------------------------------------------------------------
#[test]
fn err_10_allocate_results_oversized() {
    let p = Pair::new();
    for count in [i32::MAX, i32::MAX - 1, i32::MAX / 2, 1 << 30] {
        let pc = p.c.allocate_results(count);
        let pr = p.r.allocate_results(count);
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "row 10: allocate_results({count}) — C NULL={} Rust NULL={}",
            pc.is_null(),
            pr.is_null()
        );
        assert!(pc.is_null(), "row 10: C allocate_results({count}) must fail");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — allocate_results(0): glibc calloc returns a non-NULL pointer
// ---------------------------------------------------------------------------
#[test]
fn err_11_allocate_results_zero() {
    let p = Pair::new();
    let pc = p.c.allocate_results(0);
    let pr = p.r.allocate_results(0);
    assert_eq!(pc.is_null(), pr.is_null(), "row 11: allocate_results(0) nullity must agree");
    assert!(!pc.is_null(), "row 11: glibc calloc(0, 24) returns a unique non-NULL pointer");
}

// ---------------------------------------------------------------------------
// Row 12 — *history == NULL bootstraps and resets the count
// ---------------------------------------------------------------------------
#[test]
fn err_12_history_null_bootstraps() {
    let p = Pair::new();
    // A deliberately bogus incoming count must be discarded by the bootstrap.
    for start in [0i32, 3, 9, 10, 11, 12345, i32::MAX] {
        let mut chist: *mut ComputationResult = std::ptr::null_mut();
        let mut rhist: *mut ComputationResult = std::ptr::null_mut();
        let mut cc = start;
        let mut rc_ = start;
        let vc = unsafe { p.c.perform_computation_with_history(9, 4, OP_SUBTRACT, &mut chist, &mut cc) };
        let vr = unsafe { p.r.perform_computation_with_history(9, 4, OP_SUBTRACT, &mut rhist, &mut rc_) };
        assert_eq!(vc, vr, "row 12: return value (start count {start})");
        assert_eq!(vc, 5);
        assert!(!chist.is_null() && !rhist.is_null(), "row 12: both must allocate");
        assert_eq!(cc, 1, "row 12: C must reset the count to 0 then increment");
        assert_eq!(rc_, 1, "row 12: Rust must reset the count to 0 then increment");
        assert_bytes_eq(
            &format!("row 12 start={start}"),
            &unsafe { read_history_bytes(chist, HISTORY_CAPACITY) },
            &unsafe { read_history_bytes(rhist, HISTORY_CAPACITY) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — *history_count >= 10 -> no write, no increment
// ---------------------------------------------------------------------------
#[test]
fn err_13_history_full_no_write() {
    let p = Pair::new();
    for start in [10i32, 11, 12, 50, i32::MAX] {
        let mut cbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let mut rbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let untouched = cbuf.bytes().to_vec();
        let mut chist = cbuf.as_mut_ptr();
        let mut rhist = rbuf.as_mut_ptr();
        let mut cc = start;
        let mut rc_ = start;
        let vc = unsafe { p.c.perform_computation_with_history(6, 7, OP_MULTIPLY, &mut chist, &mut cc) };
        let vr = unsafe { p.r.perform_computation_with_history(6, 7, OP_MULTIPLY, &mut rhist, &mut rc_) };
        // The value is still computed and returned, just not recorded.
        assert_eq!(vc, 42, "row 13: C still returns the computed value");
        assert_eq!(vc, vr, "row 13: return value (start {start})");
        assert_eq!(cc, start, "row 13: C must not increment past capacity");
        assert_eq!(rc_, start, "row 13: Rust must not increment past capacity");
        assert_eq!(cbuf.bytes(), &untouched[..], "row 13: C wrote to a full history");
        assert_eq!(rbuf.bytes(), &untouched[..], "row 13: Rust wrote to a full history");
    }
}

// ---------------------------------------------------------------------------
// Row 14 — status is always STATUS_SUCCESS; the other enumerators are dead
// ---------------------------------------------------------------------------
#[test]
fn err_14_status_always_success() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x14);
    for _ in 0..500 {
        let op = rng.next_i32_mixed();
        let (a, mut b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let eff = if (OP_ADD..=OP_MODULO).contains(&op) { op } else { OP_ADD };
        if (eff == OP_DIVIDE || eff == OP_MODULO) && a == i32::MIN && b == -1 {
            b = 0; // exercise the sentinel path instead of the trap
        }
        let mut cbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let mut rbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let mut chist = cbuf.as_mut_ptr();
        let mut rhist = rbuf.as_mut_ptr();
        let (mut cc, mut rc_) = (0, 0);
        unsafe { p.c.perform_computation_with_history(a, b, op, &mut chist, &mut cc) };
        unsafe { p.r.perform_computation_with_history(a, b, op, &mut rhist, &mut rc_) };
        assert_eq!(cbuf.slot(0).status, 0, "row 14: C status must be STATUS_SUCCESS");
        assert_eq!(rbuf.slot(0).status, 0, "row 14: Rust status must be STATUS_SUCCESS");
        assert_ne!(cbuf.slot(0).status, -1, "STATUS_ERROR is unreachable");
        assert_ne!(cbuf.slot(0).status, 1, "STATUS_WARNING is unreachable");
        assert_bytes_eq("row 14", cbuf.bytes(), rbuf.bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 15 — out-of-range op through pcwh silently adds
// ---------------------------------------------------------------------------
#[test]
fn err_15_pcwh_out_of_range_op_adds() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x15);
    for op in [0i32, 6, -1, -3, i32::MIN, i32::MAX, 12345] {
        for _ in 0..100 {
            let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            let mut cbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
            let mut rbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
            let mut chist = cbuf.as_mut_ptr();
            let mut rhist = rbuf.as_mut_ptr();
            let (mut cc, mut rc_) = (0, 0);
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut chist, &mut cc) };
            let vr = unsafe { p.r.perform_computation_with_history(a, b, op, &mut rhist, &mut rc_) };
            assert_eq!(vc, vr, "row 15: op {op} on ({a}, {b})");
            assert_eq!(vc, a.wrapping_add(b), "row 15: the rejection is a silent addition");
            assert_bytes_eq("row 15", cbuf.bytes(), rbuf.bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — NULL out-parameters are unchecked -> same fatal signal
// ---------------------------------------------------------------------------
#[test]
fn err_16_null_outparams_segv() {
    // `*history` on a NULL `history`.
    assert_same_death("null_history", Outcome::Signal(SIGSEGV));
    // `*history_count` on a NULL `history_count`.
    assert_same_death("null_count", Outcome::Signal(SIGSEGV));
    // Both NULL.
    assert_same_death("null_both", Outcome::Signal(SIGSEGV));
}

// ---------------------------------------------------------------------------
// Row 17 — the replaced validation_char cannot affect the result
//
// Proved by an independent model of `mathop` that never looks at
// `validation_char`: if C matches the model over thousands of inputs, the
// variable really is dead, and Rust must match too.
// ---------------------------------------------------------------------------
fn apply_op(op: c_int, a: c_int, b: c_int) -> c_int {
    match op {
        OP_MULTIPLY => a.wrapping_mul(b),
        OP_SUBTRACT => a.wrapping_sub(b),
        OP_DIVIDE => {
            if b == 0 {
                0
            } else {
                a.wrapping_div(b)
            }
        }
        OP_MODULO => {
            if b == 0 {
                0
            } else {
                a.wrapping_rem(b)
            }
        }
        // OP_ADD and every out-of-range value alike (`default:`).
        _ => a.wrapping_add(b),
    }
}

/// `mathop` with `validation_char` removed entirely.
fn model_mathop(p1: c_int, p2: c_int, p3: c_int, p4: c_int, ts: i64) -> c_int {
    let selected_op = p3.wrapping_rem(5).wrapping_add(1);
    let priority = selected_op.wrapping_mul(10);
    let intermediate = apply_op(selected_op, p1, p2);
    let second_op = p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    let mut final_result = apply_op(second_op, intermediate, p4);
    final_result = final_result.wrapping_add(priority);
    final_result.wrapping_add((ts % 100) as c_int)
}

fn model_would_trap(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> bool {
    let selected_op = p3.wrapping_rem(5).wrapping_add(1);
    if (selected_op == OP_DIVIDE || selected_op == OP_MODULO) && p1 == i32::MIN && p2 == -1 {
        return true;
    }
    let intermediate = apply_op(selected_op, p1, p2);
    let second_op = p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    (second_op == OP_DIVIDE || second_op == OP_MODULO) && intermediate == i32::MIN && p4 == -1
}

#[test]
fn err_17_mathop_invalid_validation_char_is_dead() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let ts = p.c.get_computation_timestamp();
    assert_eq!(ts, p.r.get_computation_timestamp());

    let mut rng = Rng::new(SEED ^ 0x17);
    let mut invalid_seen = 0usize;
    let mut valid_seen = 0usize;
    let mut n = 0usize;
    while n < 3000 {
        let (p1, p2, p3, p4) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        if model_would_trap(p1, p2, p3, p4) {
            continue;
        }
        let vc = p.c.mathop(p1, p2, p3, p4);
        let vr = p.r.mathop(p1, p2, p3, p4);
        assert_eq!(vc, vr, "row 17: mathop({p1}, {p2}, {p3}, {p4})");
        assert_eq!(
            vc,
            model_mathop(p1, p2, p3, p4, ts),
            "row 17: C must match the validation_char-free model for \
             mathop({p1}, {p2}, {p3}, {p4})"
        );
        let vch = (p1 % 128) as i8;
        if vch != 0 && (b'1' as i8..=b'5' as i8).contains(&vch) {
            valid_seen += 1;
        } else {
            invalid_seen += 1;
        }
        n += 1;
    }
    assert!(invalid_seen > 100, "row 17 needs invalid-char cases, saw {invalid_seen}");
    assert!(valid_seen > 0, "row 17 needs valid-char cases too, saw {valid_seen}");
}

// ---------------------------------------------------------------------------
// Row 18 — negative param3 -> out-of-range op and non-positive priority
// ---------------------------------------------------------------------------
#[test]
fn err_18_mathop_negative_param3() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x18);
    for p3 in [-1i32, -2, -3, -4, -5, -6, -9, -1234, i32::MIN, i32::MIN + 1] {
        let op = p3.wrapping_rem(5).wrapping_add(1);
        assert!(op <= 1, "row 18: negative param3 {p3} yields op {op} (<= 1)");
        // The priority the C computes for that out-of-range op.
        let prio = p.c.get_operation_priority(op);
        assert_eq!(prio, p.r.get_operation_priority(op));
        assert!(prio <= 10, "row 18: priority for op {op} is {prio}");
        // And select_operation rejects it into add_operation (unless op == 1).
        let f = p.c.select_operation(op);
        assert_eq!(p.c.identify_mathfn(f), "add_operation");

        for _ in 0..200 {
            let (p1, p2, p4) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
            if model_would_trap(p1, p2, p3, p4) {
                continue;
            }
            assert_eq!(
                p.c.mathop(p1, p2, p3, p4),
                p.r.mathop(p1, p2, p3, p4),
                "row 18: mathop({p1}, {p2}, {p3}, {p4})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — param4 == INT_MAX overflows param4 + 1
// ---------------------------------------------------------------------------
#[test]
fn err_19_mathop_param4_intmax_overflow() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x19);
    let p4 = i32::MAX;
    let second_op = p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    assert_eq!(second_op, -2, "row 19: INT_MAX + 1 wraps to INT_MIN, giving op -2");
    let f = p.c.select_operation(second_op);
    assert_eq!(p.c.identify_mathfn(f), "add_operation", "row 19: op -2 must fall to default:");
    assert_eq!(p.r.identify_mathfn(p.r.select_operation(second_op)), "add_operation");
    for _ in 0..1000 {
        let (p1, p2, p3) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
        if model_would_trap(p1, p2, p3, p4) {
            continue;
        }
        assert_eq!(
            p.c.mathop(p1, p2, p3, p4),
            p.r.mathop(p1, p2, p3, p4),
            "row 19: mathop({p1}, {p2}, {p3}, {p4})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 20 — param1 == INT_MIN -> validation char is NUL -> invalid
// ---------------------------------------------------------------------------
#[test]
fn err_20_mathop_param1_intmin() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let vch = (i32::MIN % 128) as i8;
    assert_eq!(vch, 0, "row 20: INT_MIN % 128 == 0");
    assert!(!p.c.is_valid_operation(vch), "row 20: NUL must be rejected");
    assert!(!p.r.is_valid_operation(vch));

    let mut rng = Rng::new(SEED ^ 0x20);
    for _ in 0..1000 {
        let (p2, p3, p4) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
        if model_would_trap(i32::MIN, p2, p3, p4) {
            continue;
        }
        assert_eq!(
            p.c.mathop(i32::MIN, p2, p3, p4),
            p.r.mathop(i32::MIN, p2, p3, p4),
            "row 20: mathop(INT_MIN, {p2}, {p3}, {p4})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 21 — the static history saturates after 5 mathop calls, and that must
// NOT change the return value
// ---------------------------------------------------------------------------
#[test]
fn err_21_history_saturates_after_five_calls() {
    let _quiet = SilentStdout::new();
    let c = Lib::c();
    let r = Lib::rust();
    let ts = c.get_computation_timestamp();
    // Same arguments 20 times in a row: the first 5 calls fill the 10 slots,
    // every later call finds the history full (row 13). The result must be
    // constant, and identical between the libraries.
    let (p1, p2, p3, p4) = (1234, 56, 7, 89);
    let expected = model_mathop(p1, p2, p3, p4, ts);
    for call in 0..20 {
        let vc = c.mathop(p1, p2, p3, p4);
        let vr = r.mathop(p1, p2, p3, p4);
        assert_eq!(vc, vr, "row 21: call {call}");
        assert_eq!(vc, expected, "row 21: call {call} — the history must not affect the result");
    }
}

// ---------------------------------------------------------------------------
// Row 22 — divide-by-zero reached from mathop's first computation
// ---------------------------------------------------------------------------
#[test]
fn err_22_mathop_divide_by_zero_path() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x22);
    // param3 % 5 == 3 -> selected_op == OP_DIVIDE
    for p3 in [3i32, 8, 13, 103, 998, 3 + 5 * 400] {
        assert_eq!(p3.wrapping_rem(5).wrapping_add(1), OP_DIVIDE, "p3 {p3} selects divide");
        for _ in 0..300 {
            let (p1, p4) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if model_would_trap(p1, 0, p3, p4) {
                continue;
            }
            let vc = p.c.mathop(p1, 0, p3, p4);
            let vr = p.r.mathop(p1, 0, p3, p4);
            assert_eq!(vc, vr, "row 22: mathop({p1}, 0, {p3}, {p4})");
            // The intermediate really was the 0 sentinel.
            assert_eq!(
                vc,
                model_mathop(p1, 0, p3, p4, p.c.get_computation_timestamp()),
                "row 22: the divide-by-zero sentinel must be 0"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — modulo-by-zero reached from mathop's second computation
// ---------------------------------------------------------------------------
#[test]
fn err_23_mathop_modulo_by_zero_path() {
    let _quiet = SilentStdout::new();
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 0x23);
    // First drive the *first* computation into modulo-by-zero:
    // param3 % 5 == 4 -> selected_op == OP_MODULO, with param2 == 0.
    // (C's `%` truncates toward zero, so only non-negative param3 lands on 4.)
    for p3 in [4i32, 9, 14, 99, 4 + 5 * 400] {
        assert_eq!(p3.wrapping_rem(5).wrapping_add(1), OP_MODULO, "p3 {p3} selects modulo");
        for _ in 0..300 {
            let p1 = rng.next_i32_mixed();
            let p4 = rng.next_i32_mixed();
            if model_would_trap(p1, 0, p3, p4) {
                continue;
            }
            assert_eq!(
                p.c.mathop(p1, 0, p3, p4),
                p.r.mathop(p1, 0, p3, p4),
                "row 23: mathop({p1}, 0, {p3}, {p4})"
            );
        }
    }
    // param4 == 0 makes the second computation's divisor zero; sweep every
    // second_op reachable with param4 == 0 plus every first op.
    for p3 in -6i32..=10 {
        for _ in 0..200 {
            let (p1, p2) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            if model_would_trap(p1, p2, p3, 0) {
                continue;
            }
            assert_eq!(
                p.c.mathop(p1, p2, p3, 0),
                p.r.mathop(p1, p2, p3, 0),
                "row 23: mathop({p1}, {p2}, {p3}, 0)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — the SIGFPE propagates out of mathop
// ---------------------------------------------------------------------------
#[test]
fn err_24_mathop_sigfpe_propagates() {
    assert_same_death("mathop_div_fpe", Outcome::Signal(SIGFPE));
    assert_same_death("mathop_mod_fpe", Outcome::Signal(SIGFPE));
}

// ---------------------------------------------------------------------------
// Row 25 — signed overflow wraps identically (no rejection)
// ---------------------------------------------------------------------------
#[test]
fn err_25_signed_overflow_wraps() {
    let p = Pair::new();
    let overflow_pairs: [(c_int, c_int); 12] = [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (1, i32::MIN),
        (-1, i32::MIN),
        (i32::MAX, -1),
        (65536, 65536),
        (46341, 46341),
        (-46341, 46341),
        (i32::MAX / 2 + 1, 2),
    ];
    for (a, b) in overflow_pairs {
        assert_eq!(p.c.add_operation(a, b, 0), p.r.add_operation(a, b, 0), "row 25: add({a},{b})");
        assert_eq!(
            p.c.subtract_operation(a, b, 0),
            p.r.subtract_operation(a, b, 0),
            "row 25: sub({a},{b})"
        );
        assert_eq!(
            p.c.multiply_operation(a, b, 0),
            p.r.multiply_operation(a, b, 0),
            "row 25: mul({a},{b})"
        );
    }
    // get_operation_priority overflow: op * 10.
    for op in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 214748365, -214748365] {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.r.get_operation_priority(op),
            "row 25: get_operation_priority({op})"
        );
    }
    let mut rng = Rng::new(SEED ^ 0x25);
    for _ in 0..3000 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        assert_eq!(p.c.add_operation(a, b, 0), p.r.add_operation(a, b, 0));
        assert_eq!(p.c.subtract_operation(a, b, 0), p.r.subtract_operation(a, b, 0));
        assert_eq!(p.c.multiply_operation(a, b, 0), p.r.multiply_operation(a, b, 0));
        assert_eq!(p.c.get_operation_priority(a), p.r.get_operation_priority(a));
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary coverage required in addition to the table:
// out-of-range enum values, one-step-past-range values.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_out_of_range_enum_values() {
    let p = Pair::new();
    // One step past each end of the documented Operation range, plus the whole
    // small neighbourhood, through every entry point that takes an Operation.
    for op in -16i32..=20 {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.r.get_operation_priority(op),
            "get_operation_priority({op})"
        );
        assert_eq!(
            p.c.identify_mathfn(p.c.select_operation(op)),
            p.r.identify_mathfn(p.r.select_operation(op)),
            "select_operation({op})"
        );
        let mut cbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let mut rbuf = HistoryBuf::zeroed(HISTORY_CAPACITY);
        let mut chist = cbuf.as_mut_ptr();
        let mut rhist = rbuf.as_mut_ptr();
        let (mut cc, mut rc_) = (0, 0);
        let vc = unsafe { p.c.perform_computation_with_history(-9, 4, op, &mut chist, &mut cc) };
        let vr = unsafe { p.r.perform_computation_with_history(-9, 4, op, &mut rhist, &mut rc_) };
        assert_eq!(vc, vr, "pcwh with enum value {op}");
        assert_eq!(cc, rc_);
        assert_bytes_eq(&format!("pcwh enum {op}"), cbuf.bytes(), rbuf.bytes());
    }
    // Also the exact enum-range boundaries as `StatusCode`-shaped values.
    for op in [i32::MIN, i32::MIN + 1, -1, 0, 1, 5, 6, i32::MAX - 1, i32::MAX] {
        assert_eq!(p.c.get_operation_priority(op), p.r.get_operation_priority(op));
        assert_eq!(
            p.c.identify_mathfn(p.c.select_operation(op)),
            p.r.identify_mathfn(p.r.select_operation(op))
        );
    }
}

/// `char` is an 8-bit type crossing the FFI boundary: cover the full domain
/// plus the "one past the valid band" values explicitly.
#[test]
fn err_generic_char_domain_and_boundaries() {
    let p = Pair::new();
    for b in 0u16..256 {
        let ch = b as u8 as i8;
        assert_eq!(p.c.is_valid_operation(ch), p.r.is_valid_operation(ch), "is_valid_operation({ch})");
    }
    for ch in [48i8, 49, 53, 54] {
        // '0', '1', '5', '6' — the two steps outside the accepted band.
        assert_eq!(p.c.is_valid_operation(ch), p.r.is_valid_operation(ch));
    }
}

/// Zero and oversized lengths for the only length-taking entry point.
#[test]
fn err_generic_lengths() {
    let p = Pair::new();
    for count in [0i32, 1, -1, i32::MAX, i32::MIN, 1 << 20] {
        let pc = p.c.allocate_results(count);
        let pr = p.r.allocate_results(count);
        assert_eq!(pc.is_null(), pr.is_null(), "allocate_results({count}) nullity");
    }
}

/// Misaligned out-parameters and a misaligned history buffer.
///
/// x86-64 C performs the unaligned 8- and 4-byte accesses without complaint, so
/// the Rust must too — this is the alignment half of the same unchecked-pointer
/// class as row 16, and it is checked in `dev` builds by default.
#[test]
fn err_generic_misaligned_pointers() {
    let p = Pair::new();
    for skew in 1usize..8 {
        // Backing storage with room for the skew.
        let mut cstore = vec![0u8; skew + HISTORY_CAPACITY * RESULT_SIZE + 8];
        let mut rstore = vec![0u8; skew + HISTORY_CAPACITY * RESULT_SIZE + 8];
        // Misaligned out-parameter cells, also skewed.
        let mut cmeta = vec![0u8; 32];
        let mut rmeta = vec![0u8; 32];

        unsafe {
            let chist_buf = cstore.as_mut_ptr().add(skew) as *mut ComputationResult;
            let rhist_buf = rstore.as_mut_ptr().add(skew) as *mut ComputationResult;

            let chist_cell = cmeta.as_mut_ptr().add(skew) as *mut *mut ComputationResult;
            let rhist_cell = rmeta.as_mut_ptr().add(skew) as *mut *mut ComputationResult;
            chist_cell.write_unaligned(chist_buf);
            rhist_cell.write_unaligned(rhist_buf);

            let ccount_cell = cmeta.as_mut_ptr().add(16 + skew) as *mut c_int;
            let rcount_cell = rmeta.as_mut_ptr().add(16 + skew) as *mut c_int;
            ccount_cell.write_unaligned(0);
            rcount_cell.write_unaligned(0);

            for step in 0..3 {
                let vc = p.c.perform_computation_with_history(
                    10 + step,
                    3,
                    OP_SUBTRACT,
                    chist_cell,
                    ccount_cell,
                );
                let vr = p.r.perform_computation_with_history(
                    10 + step,
                    3,
                    OP_SUBTRACT,
                    rhist_cell,
                    rcount_cell,
                );
                assert_eq!(vc, vr, "skew {skew} step {step}: return value");
                assert_eq!(
                    ccount_cell.read_unaligned(),
                    rcount_cell.read_unaligned(),
                    "skew {skew} step {step}: history_count"
                );
                assert_eq!(
                    chist_cell.read_unaligned(),
                    chist_buf,
                    "skew {skew}: C must not replace the history pointer"
                );
            }
        }
        // Compare the whole skewed region byte-for-byte.
        assert_bytes_eq(&format!("misaligned skew {skew}"), &cstore, &rstore);
    }
}
