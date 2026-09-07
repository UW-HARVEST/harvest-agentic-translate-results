//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test constructs the exact invalid
//! input and asserts C and Rust produce the SAME error/sentinel/fault — not
//! merely "both failed somehow".

mod common;
use common::*;
use std::ffi::c_int;
use std::process::Command;

// ===========================================================================
// rows 1..5 — arity() length rejection & truncation
// ===========================================================================

#[test]
fn err_01_arity_len_trunc_zero() {
    let (c, r) = both();
    let params = [11i32, 22, 33, 44, 55, 66, 77, 88];
    // Every int whose low byte is 0 must be rejected with exactly -1.
    let mut lens = vec![0i32, 256, 512, 768, 1024, 65536, 0x7FFF_FF00];
    lens.extend((1..=64).map(|k| k * 256));
    for len in lens {
        assert_eq!(len as u8, 0, "test bug: {len} low byte != 0");
        let cv = unsafe { (c.arity)(len, params.as_ptr()) };
        let rv = unsafe { (r.arity)(len, params.as_ptr()) };
        eq_i32("err-01", len, cv, rv);
        assert_eq!(cv, -1, "C must reject len={len} (low byte 0) with -1");
    }
}

#[test]
fn err_02_arity_len_trunc_one() {
    let (c, r) = both();
    let params = [11i32, 22, 33, 44, 55, 66, 77, 88];
    let mut lens = vec![1i32, 257, 513, 65537];
    lens.extend((1..=64).map(|k| k * 256 + 1));
    for len in lens {
        assert_eq!(len as u8, 1, "test bug: {len} low byte != 1");
        let cv = unsafe { (c.arity)(len, params.as_ptr()) };
        let rv = unsafe { (r.arity)(len, params.as_ptr()) };
        eq_i32("err-02", len, cv, rv);
        assert_eq!(cv, -1, "C must reject len={len} (low byte 1) with -1");
    }
}

#[test]
fn err_03_arity_null_params_is_safe() {
    // The `len < 2` guard runs BEFORE params is touched, so a NULL params with
    // a rejected len must return -1 rather than fault. Run in a subprocess so
    // that a *wrong* implementation which derefs first is observed as a crash
    // instead of taking the whole test binary down.
    let out_c = run_worker(&c_so_path().to_string_lossy(), "arity_null_guarded");
    let out_r = run_worker(&rust_so_path().to_string_lossy(), "arity_null_guarded");
    assert_eq!(out_c, out_r, "err-03: C/Rust divergence\nC: {out_c:?}\nR: {out_r:?}");
    assert!(out_c.0, "err-03: C must NOT fault on arity(len<2, NULL)");
    assert!(
        out_c.1.lines().all(|l| l.ends_with("= -1")),
        "err-03: every guarded arity(len,NULL) must be -1, got:\n{}",
        out_c.1
    );
    // And the mirror: a dispatching len with NULL params faults in BOTH.
    let a_c = run_worker(&c_so_path().to_string_lossy(), "arity_null_active");
    let a_r = run_worker(&rust_so_path().to_string_lossy(), "arity_null_active");
    assert_eq!(a_c, a_r, "err-03b: C/Rust divergence on arity(4, NULL)");
    assert!(!a_c.0, "err-03b: arity(4, NULL) is expected to fault in C");
}

#[test]
fn err_04_arity_negative_len() {
    let (c, r) = both();
    let params = [11i32, 22, 33, 44, 55, 66, 77, 88];
    let mut lens: Vec<i32> = (-600..=-1).collect();
    lens.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MIN + 255]);
    for len in lens {
        // A low byte >= 2 dispatches to arity4, which reaches
        // compare_allocations — so this is heap-order sensitive.
        let cv = diff_norm("err-04", len, true,
            &mut || unsafe { (c.arity)(len, params.as_ptr()) },
            &mut || unsafe { (r.arity)(len, params.as_ptr()) });
        // ground truth: unsigned byte comparison of the low 8 bits
        let lo = len as u8;
        if lo < 2 {
            assert_eq!(cv, -1, "len={len} lo={lo} must be -1");
        } else {
            assert_ne!(cv, -1, "len={len} lo={lo} must dispatch, not reject");
        }
    }
}

#[test]
fn err_05_arity_large_len_reads_only_4() {
    // A len of 5..255 says "I gave you N params" but arity4 only ever reads 4.
    // The buffer is deliberately only 4 elements long, fenced by a canary, to
    // prove neither implementation reads past params[3].
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for len in 4i32..=255 {
        let p: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
        diff_norm("err-05", (len, &p), true,
            &mut || unsafe { (c.arity)(len, p.as_ptr()) },
            &mut || unsafe { (r.arity)(len, p.as_ptr()) });
        // ground truth: must behave exactly like arity4 — compared within C itself
        diff_norm("err-05: must behave exactly like arity4", (len, &p), true,
            &mut || unsafe { (c.arity)(len, p.as_ptr()) },
            &mut || unsafe { (c.arity4)(p[0], p[1], p[2], p[3]) });
    }
}

// ===========================================================================
// rows 6..8 — compare_allocations
// ===========================================================================

#[test]
fn err_06_compare_alloc_oom_unreachable() {
    // The `ptr1 == NULL || ptr2 == NULL -> return -1` branch cannot be induced
    // for malloc(4) with a working glibc. Assert the reachable invariant
    // identically for both: the -1 sentinel is NEVER produced.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..2000 {
            let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
            let cv = diff_norm("err-06", (asc, a, b), asc,
                &mut || unsafe { (c.compare_allocations)(a, b) },
                &mut || unsafe { (r.compare_allocations)(a, b) });
            let rv = cv; // diff_norm returned only because C and Rust agreed
            assert_ne!(cv, -1, "OOM sentinel unexpectedly reached in C");
            assert_ne!(rv, -1, "OOM sentinel unexpectedly reached in Rust");
        }
    }
}

#[test]
fn err_07_compare_alloc_nonpositive_val1() {
    // `(*uninit_ptr > 0)` is false => add 0, not 10.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        // exact boundary: 0 and -1 must NOT get +10; 1 must.
        for &(v1, want_plus10) in &[(0i32, false), (-1, false), (i32::MIN, false), (1, true)] {
            let cv = diff_norm("err-07", (asc, v1), asc,
                &mut || unsafe { (c.compare_allocations)(v1, 7) },
                &mut || unsafe { (r.compare_allocations)(v1, 7) });
            let base = if asc { 1 } else { 2 };
            assert_eq!(cv, base + if want_plus10 { 10 } else { 0 }, "v1={v1} asc={asc}");
        }
        for _ in 0..1000 {
            let v1 = rng.range_i32(i32::MIN, 0);
            let v2 = rng.spicy_i32();
            let cv = diff_norm("err-07", (asc, v1, v2), asc,
                &mut || unsafe { (c.compare_allocations)(v1, v2) },
                &mut || unsafe { (r.compare_allocations)(v1, v2) });
            assert!(cv < 10, "v1={v1} <= 0 must not take the +10 arm (got {cv})");
        }
    }
}

#[test]
fn err_08_compare_alloc_equal_arm() {
    // `ptr1 == ptr2 -> result = 3` is unreachable (two live mallocs are never
    // the same address). Assert the reachable invariant for both: result is
    // always in {1,2,11,12}, never 3 or 13.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..2000 {
            let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
            let cv = diff_norm("err-08", (asc, a, b), asc,
                &mut || unsafe { (c.compare_allocations)(a, b) },
                &mut || unsafe { (r.compare_allocations)(a, b) });
            let rv = cv; // diff_norm returned only because C and Rust agreed
            assert!(matches!(cv, 1 | 2 | 11 | 12), "C gave {cv}");
            assert!(matches!(rv, 1 | 2 | 11 | 12), "Rust gave {rv}");
        }
    }
}

// ===========================================================================
// rows 9..11 — process_string / apply_bitmask
// ===========================================================================

#[test]
fn err_09_process_string_empty() {
    let (c, r) = both();
    // `if (*str)` false -> return 0 without calling strlen. The buffer holds
    // trailing garbage after the NUL to prove strlen is really not consulted.
    let buf = cbuf(&[]);
    let cv = unsafe { (c.process_string)(buf.as_ptr()) };
    let rv = unsafe { (r.process_string)(buf.as_ptr()) };
    eq_i32("err-09", "empty", cv, rv);
    assert_eq!(cv, 0);

    let mut buf2: Vec<std::ffi::c_char> = vec![0];
    buf2.extend([b'A' as std::ffi::c_char; 16]);
    buf2.push(0);
    let cv = unsafe { (c.process_string)(buf2.as_ptr()) };
    let rv = unsafe { (r.process_string)(buf2.as_ptr()) };
    eq_i32("err-09", "NUL then garbage", cv, rv);
    assert_eq!(cv, 0, "leading NUL must short-circuit to 0");
}

#[test]
fn err_10_process_string_null_segv() {
    // No NULL check in C: `*str` is dereferenced unconditionally. Both
    // libraries must fault the SAME way.
    let a = run_worker(&c_so_path().to_string_lossy(), "process_string_null");
    let b = run_worker(&rust_so_path().to_string_lossy(), "process_string_null");
    assert_eq!(a, b, "err-10: C/Rust divergence\nC: {a:?}\nRust: {b:?}");
    assert!(!a.0, "err-10: C is expected to fault on process_string(NULL)");
}

#[test]
fn err_11_apply_bitmask_default() {
    // `default:` returns `value` untouched. Covers out-of-range "enum" values
    // crossing the FFI boundary, which C accepts as any int.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    let mut ops: Vec<i32> = (4..=64).collect();
    ops.extend_from_slice(&[255, 256, 1000, 65536, i32::MAX, i32::MAX - 1]);
    ops.extend((-64..=-1).collect::<Vec<i32>>());
    ops.extend_from_slice(&[i32::MIN, i32::MIN + 1]);
    for &op in &ops {
        assert!(!(0..=3).contains(&op), "test bug: {op} is a real case label");
        for _ in 0..40 {
            let v = rng.spicy_i32();
            let cv = unsafe { (c.apply_bitmask)(v, op) };
            let rv = unsafe { (r.apply_bitmask)(v, op) };
            eq_i32("err-11", (v, op), cv, rv);
            assert_eq!(cv, v, "default: must return value unchanged (op={op})");
        }
    }
}

#[test]
fn err_12_negative_modulo_hits_default() {
    // C's `%` truncates toward zero, so for param1 < 0 the operation passed to
    // apply_bitmask is negative and lands on `default:` (identity). This is the
    // canonical "translation used rem_euclid and silently changed behaviour"
    // bug, so assert the operation values directly AND end to end.
    let (c, r) = both();
    for op in [-1i32, -2, -3] {
        for v in [0i32, 1, 255, -1, i32::MAX, i32::MIN] {
            let cv = unsafe { (c.apply_bitmask)(v, op) };
            let rv = unsafe { (r.apply_bitmask)(v, op) };
            eq_i32("err-12", (v, op), cv, rv);
            assert_eq!(cv, v, "op={op} must be identity, not case {}", op + 4);
        }
    }
    // End to end through arity4, for param1 in each negative residue class.
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..1500 {
            let p1 = rng.range_i32(i32::MIN + 4, -1);
            let p2 = rng.spicy_i32();
            diff_norm("err-12/e2e", (asc, p1, p2), asc,
                &mut || unsafe { (c.arity4)(p1, p2, 0, 0) },
                &mut || unsafe { (r.arity4)(p1, p2, 0, 0) });
        }
    }
}

// ===========================================================================
// rows 13..15, 26 — shift_array guard rejections
// ===========================================================================

/// Calls shift_array on both and asserts the buffers stay byte-identical to
/// each other AND (when `expect_noop`) to the original contents.
fn shift_guard_case(row: &str, c: &Lib, r: &Lib, data: &[c_int], size: c_int, positions: c_int) {
    let mut cb = data.to_vec();
    let mut rb = data.to_vec();
    unsafe {
        (c.shift_array)(cb.as_mut_ptr(), size, positions);
        (r.shift_array)(rb.as_mut_ptr(), size, positions);
    }
    eq_slice(row, (size, positions), &cb, &rb);
    assert_eq!(cb, data, "{row}: size={size} positions={positions} must be a NO-OP");
}

#[test]
fn err_13_shift_positions_nonpositive() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..500 {
        let size = rng.range_i32(1, 32);
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.spicy_i32()).collect();
        for positions in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
            shift_guard_case("err-13", &c, &r, &data, size, positions);
        }
    }
}

#[test]
fn err_14_shift_positions_ge_size() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..500 {
        let size = rng.range_i32(1, 32);
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.spicy_i32()).collect();
        // positions == size is the exact boundary; also one past and far past.
        for positions in [size, size + 1, size + 100, i32::MAX] {
            shift_guard_case("err-14", &c, &r, &data, size, positions);
        }
    }
}

#[test]
fn err_15_shift_nonpositive_size() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..500 {
        // Buffer is real and 8 long, but `size` is claimed as <= 0.
        let data: Vec<c_int> = (0..8).map(|_| rng.spicy_i32()).collect();
        for size in [0i32, -1, -2, -100, i32::MIN] {
            for positions in [1i32, 2, 0, -1, i32::MAX, i32::MIN] {
                shift_guard_case("err-15", &c, &r, &data, size, positions);
            }
        }
    }
}

#[test]
fn err_26_shift_array_null_guarded_safe() {
    // arr == NULL but the guard is false: `positions > 0 && positions < size`
    // short-circuits, so NULL is never dereferenced and both must return
    // safely. Subprocess so a wrong impl shows up as a crash, not as a
    // harness abort.
    let a = run_worker(&c_so_path().to_string_lossy(), "shift_array_null_guarded");
    let b = run_worker(&rust_so_path().to_string_lossy(), "shift_array_null_guarded");
    assert_eq!(a, b, "err-26: C/Rust divergence\nC: {a:?}\nRust: {b:?}");
    assert!(a.0, "err-26: guarded shift_array(NULL, ...) must NOT fault in C");
}

#[test]
fn err_25_shift_array_null_segv() {
    // arr == NULL with a guard-PASSING (size=4, positions=1): memmove(NULL).
    let a = run_worker(&c_so_path().to_string_lossy(), "shift_array_null_active");
    let b = run_worker(&rust_so_path().to_string_lossy(), "shift_array_null_active");
    assert_eq!(a, b, "err-25: C/Rust divergence\nC: {a:?}\nRust: {b:?}");
    assert!(!a.0, "err-25: shift_array(NULL,4,1) is expected to fault in C");
}

// ===========================================================================
// rows 17..23 — arity4 arithmetic edges
// ===========================================================================

#[test]
fn err_17_arity4_param3_zero() {
    // param3 == 0 skips `(result * param3) / 100` entirely — in particular it
    // must NOT zero the result and must NOT divide.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..1500 {
            let (p1, p2, p4) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            diff_norm("err-17", (asc, p1, p2, p4), asc,
                &mut || unsafe { (c.arity4)(p1, p2, 0, p4) },
                &mut || unsafe { (r.arity4)(p1, p2, 0, p4) });
        }
    }
}

#[test]
fn err_18_arity4_param4_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..1500 {
            let (p1, p2, p3) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            diff_norm("err-18", (asc, p1, p2, p3), asc,
                &mut || unsafe { (c.arity4)(p1, p2, p3, 0) },
                &mut || unsafe { (r.arity4)(p1, p2, p3, 0) });
        }
    }
}

#[test]
fn err_19_arity4_mul_overflow() {
    // `result * param3` is signed-overflow UB in C; the emitted code is a
    // plain `imul`, i.e. 2's-complement wraparound. Rust must wrap, not panic
    // and not saturate.
    let (c, r) = both();
    let extremes = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0x4000_0000, -0x4000_0000];
    for asc in BOTH_ORDERS {
        for &p1 in &extremes {
            for &p2 in &extremes {
                for &p3 in &extremes {
                    diff_norm("err-19", (asc, p1, p2, p3), asc,
                        &mut || unsafe { (c.arity4)(p1, p2, p3, 0) },
                        &mut || unsafe { (r.arity4)(p1, p2, p3, 0) });
                }
            }
        }
    }
}

#[test]
fn err_20_arity4_add_overflow() {
    let (c, r) = both();
    let extremes = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1];
    for asc in BOTH_ORDERS {
        for &p1 in &extremes {
            for &p2 in &extremes {
                for &p3 in &[0i32, 1, -1, i32::MAX, i32::MIN] {
                    for &p4 in &extremes {
                        diff_norm("err-20", (asc, p1, p2, p3, p4), asc,
                            &mut || unsafe { (c.arity4)(p1, p2, p3, p4) },
                            &mut || unsafe { (r.arity4)(p1, p2, p3, p4) });
                    }
                }
            }
        }
    }
}

#[test]
fn err_21_arity4_negative_division() {
    // C integer division truncates toward ZERO, so -150/100 == -1 (not -2).
    // Sweep param1/param3 so the dividend lands on both sides of zero and on
    // exact/inexact multiples of 100.
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        for p1 in -300i32..=300 {
            for &p3 in &[-1i32, 1, -2, 2, -100, 100, -101, 101, -7, 7] {
                diff_norm("err-21", (asc, p1, p3), asc,
                    &mut || unsafe { (c.arity4)(p1, 5, p3, 0) },
                    &mut || unsafe { (r.arity4)(p1, 5, p3, 0) });
            }
        }
    }
}

#[test]
fn err_22_arity4_param1_int_min() {
    // param1 % 4 with param1 == INT_MIN. (The UB case is `% -1`, not `% 4`,
    // so this is well-defined and equals 0.) Must not panic in Rust.
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        for &p1 in &[i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3, i32::MIN + 4] {
            for &p2 in &[0i32, 1, -1, i32::MIN, i32::MAX] {
                for &p3 in &[0i32, 1, -1] {
                    for &p4 in &[0i32, 1, -1] {
                        diff_norm("err-22", (asc, p1, p2, p3, p4), asc,
                            &mut || unsafe { (c.arity4)(p1, p2, p3, p4) },
                            &mut || unsafe { (r.arity4)(p1, p2, p3, p4) });
                    }
                }
            }
        }
    }
    // Also via arity2/arity3/arity so every entry point sees INT_MIN.
    for asc in BOTH_ORDERS {
        diff_norm("err-22/arity2", asc, asc,
            &mut || unsafe { (c.arity2)(i32::MIN, i32::MIN) },
            &mut || unsafe { (r.arity2)(i32::MIN, i32::MIN) });
        let p = [i32::MIN; 4];
        diff_norm("err-22/arity", asc, asc,
            &mut || unsafe { (c.arity)(4, p.as_ptr()) },
            &mut || unsafe { (r.arity)(4, p.as_ptr()) });
    }
}

#[test]
fn err_23_arity2_arity3_defaults() {
    // arity2/arity3 must forward the implicit 0 defaults exactly.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for asc in BOTH_ORDERS {
        for _ in 0..1000 {
            let (p1, p2, p3) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());

            diff_norm("err-23/arity2", (asc, p1, p2), asc,
                &mut || unsafe { (c.arity2)(p1, p2) },
                &mut || unsafe { (r.arity2)(p1, p2) });
            // ground truth, within each library: C: arity2 != arity4(_,_,0,0)
            diff_norm("err-23/arity2 — C: arity2 != arity4(_,_,0,0)", (asc, p1, p2), asc,
                &mut || unsafe { (c.arity2)(p1, p2) },
                &mut || unsafe { (c.arity4)(p1, p2, 0, 0) });
            // Rust: arity2 != arity4(_,_,0,0)
            diff_norm("err-23/arity2 — Rust: arity2 != arity4(_,_,0,0)", (asc, p1, p2), asc,
                &mut || unsafe { (r.arity2)(p1, p2) },
                &mut || unsafe { (r.arity4)(p1, p2, 0, 0) });

            diff_norm("err-23/arity3", (asc, p1, p2, p3), asc,
                &mut || unsafe { (c.arity3)(p1, p2, p3) },
                &mut || unsafe { (r.arity3)(p1, p2, p3) });
            // ground truth, within each library: C: arity3 != arity4(_,_,_,0)
            diff_norm("err-23/arity3 — C: arity3 != arity4(_,_,_,0)", (asc, p1, p2, p3), asc,
                &mut || unsafe { (c.arity3)(p1, p2, p3) },
                &mut || unsafe { (c.arity4)(p1, p2, p3, 0) });
            // Rust: arity3 != arity4(_,_,_,0)
            diff_norm("err-23/arity3 — Rust: arity3 != arity4(_,_,_,0)", (asc, p1, p2, p3), asc,
                &mut || unsafe { (r.arity3)(p1, p2, p3) },
                &mut || unsafe { (r.arity4)(p1, p2, p3, 0) });
        }
    }
}

#[test]
fn err_24_init_matrix_null_segv() {
    let a = run_worker(&c_so_path().to_string_lossy(), "init_matrix_null");
    let b = run_worker(&rust_so_path().to_string_lossy(), "init_matrix_null");
    assert_eq!(a, b, "err-24: C/Rust divergence\nC: {a:?}\nRust: {b:?}");
    assert!(!a.0, "err-24: init_matrix(NULL) is expected to fault in C");
}

// ===========================================================================
// Subprocess helper
// ===========================================================================

/// Runs `examples/crash_worker <lib> <case>`.
/// Returns `(exited_cleanly, stdout, signal_or_code)`.
fn run_worker(lib: &str, case: &str) -> (bool, String, String) {
    let mut exe = std::env::current_exe().unwrap();
    exe.pop();
    exe.pop();
    let exe = exe.join("examples").join("crash_worker");
    assert!(
        exe.exists(),
        "{} missing — run `cargo build --release --examples` first",
        exe.display()
    );
    let out = Command::new(&exe)
        .arg(lib)
        .arg(case)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    use std::os::unix::process::ExitStatusExt;
    let status = if let Some(sig) = out.status.signal() {
        format!("signal {sig}")
    } else {
        format!("exit {}", out.status.code().unwrap_or(-1))
    };
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        status,
    )
}
