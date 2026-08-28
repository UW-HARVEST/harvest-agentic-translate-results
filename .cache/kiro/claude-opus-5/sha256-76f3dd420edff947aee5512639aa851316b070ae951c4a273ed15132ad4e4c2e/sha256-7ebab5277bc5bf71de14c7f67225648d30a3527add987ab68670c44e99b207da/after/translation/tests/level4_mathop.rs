//! Level 4: `mathop`, the public entry point from `include/lib.h`.
//!
//! `mathop` owns file-scope `static` state (the history buffer and its counter)
//! and writes four lines to stdout. Both are compared: the C and Rust libraries
//! are driven in lockstep so their independent statics stay aligned, and the
//! raw stdout bytes of each call are diffed.
mod common;

use common::{both, capture_stdout, global_lock};
use std::ffi::c_int;

/// Mirrors `mathop`'s arithmetic in `i64` and reports `None` if any step would
/// overflow a signed `int` (UB in C, so not a behaviour to match) or would hit
/// `INT_MIN / -1` (traps).
fn is_ub_free(p1: i32, p2: i32, p3: i32, p4: i32) -> bool {
    fn fits(v: i64) -> Option<i32> {
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            Some(v as i32)
        } else {
            None
        }
    }
    fn apply(op: i32, a: i32, b: i32) -> Option<i32> {
        match op {
            2 => fits(a as i64 * b as i64),
            3 => fits(a as i64 - b as i64),
            4 | 5 => {
                if b == 0 {
                    Some(0)
                } else if a == i32::MIN && b == -1 {
                    None // SIGFPE in C
                } else if op == 4 {
                    Some(a / b)
                } else {
                    Some(a % b)
                }
            }
            // OP_ADD and every value that falls through `default:`
            _ => fits(a as i64 + b as i64),
        }
    }

    if p4 == i32::MAX {
        return false; // param4 + 1 overflows
    }
    let sel_op = p3 % 5 + 1;
    let priority = sel_op * 10;
    let Some(intermediate) = apply(sel_op, p1, p2) else {
        return false;
    };
    let second_op = (p4 + 1) % 5 + 1;
    let Some(mut final_result) = apply(second_op, intermediate, p4) else {
        return false;
    };
    let Some(v) = fits(final_result as i64 + priority as i64) else {
        return false;
    };
    final_result = v;
    // time_modifier is in 0..=99 for the current (positive) shifted timestamp.
    fits(final_result as i64 + 99).is_some()
}

/// Calls `mathop` on both libraries with identical arguments and asserts the
/// return value and the printed bytes match.
fn compare_mathop(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let b = both();
    let (cr, cout) = capture_stdout(|| unsafe { (b.c.mathop)(p1, p2, p3, p4) });
    let (rr, rout) = capture_stdout(|| unsafe { (b.rust.mathop)(p1, p2, p3, p4) });
    assert_eq!(
        cr, rr,
        "mathop({p1},{p2},{p3},{p4}) return value: C={cr} Rust={rr}"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "mathop({p1},{p2},{p3},{p4}) stdout differs"
    );
    assert_eq!(cout, rout, "mathop({p1},{p2},{p3},{p4}) stdout bytes differ");
    assert!(!cout.is_empty(), "expected mathop to print something");
}

#[test]
fn mathop_small_exhaustive_grid() {
    let _g = global_lock();
    // Small values sweep every operation selector, both `validation_char`
    // branches, and every divide/modulo-by-zero guard.
    let vals: [c_int; 21] = [
        -10, -9, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 48, 49, 53, 54,
    ];
    for &p1 in &vals {
        for &p2 in &vals {
            for &p3 in &[-6_i32, -5, -1, 0, 1, 2, 3, 4, 5, 6] {
                for &p4 in &[-6_i32, -1, 0, 1, 2, 3, 4, 5] {
                    if is_ub_free(p1, p2, p3, p4) {
                        compare_mathop(p1, p2, p3, p4);
                    }
                }
            }
        }
    }
}

#[test]
fn mathop_validation_char_branches() {
    let _g = global_lock();
    // `is_valid_operation((char)(param1 % 128))` is true only for '1'..'5'
    // (49..53) and their negative-modulo counterparts. Cover the boundary
    // values on both sides plus the wrap-around at 128.
    let mut params: Vec<c_int> = vec![
        0, 47, 48, 49, 50, 51, 52, 53, 54, 55, 127, 128, 129, 176, 177, 180, 181, 182, 256, 305,
        -47, -48, -49, -53, -54, -128, -129, -177, -181,
    ];
    params.extend(120..=136);
    for p1 in params {
        for p3 in [0, 1, 2, 3, 4] {
            for p4 in [0, 1, 2] {
                if is_ub_free(p1, 3, p3, p4) {
                    compare_mathop(p1, 3, p3, p4);
                }
            }
        }
    }
}

#[test]
fn mathop_operation_selector_sweep() {
    let _g = global_lock();
    // param3 and param4 drive `selected_op` and `second_op`, including the
    // negative-modulo results that fall through to `default:` (add).
    let sel: Vec<c_int> = (-15..=15).collect();
    for &p3 in &sel {
        for &p4 in &sel {
            if is_ub_free(37, 6, p3, p4) {
                compare_mathop(37, 6, p3, p4);
            }
            if is_ub_free(-37, -6, p3, p4) {
                compare_mathop(-37, -6, p3, p4);
            }
        }
    }
}

#[test]
fn mathop_zero_divisor_paths() {
    let _g = global_lock();
    // second_op == OP_DIVIDE / OP_MODULO with param4 == 0 exercises the
    // `b == 0 -> 0` guards inside the second computation.
    for p3 in [-5_i32, 0, 3, 4, 8] {
        for p1 in [-100_i32, -1, 0, 1, 100, 9999] {
            for p2 in [0_i32, 1, -1, 7] {
                // param4 == 3 -> second_op 4 (divide); param4 == 4 -> 5 (modulo)
                for p4 in [0_i32, 3, 4] {
                    if is_ub_free(p1, p2, p3, p4) {
                        compare_mathop(p1, p2, p3, p4);
                    }
                }
            }
        }
    }
}

#[test]
fn mathop_large_and_extreme_values() {
    let _g = global_lock();
    let vals: [c_int; 17] = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_000,
        -100_000,
        -46_341,
        -46_340,
        -1024,
        -128,
        0,
        128,
        1024,
        46_340,
        46_341,
        100_000,
        1_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &p1 in &vals {
        for &p2 in &vals {
            for &p3 in &[0_i32, 1, 2, 3, 4, -1, -2] {
                for &p4 in &[0_i32, 1, -1, 2, 3, 4, 1000, -1000] {
                    if is_ub_free(p1, p2, p3, p4) {
                        compare_mathop(p1, p2, p3, p4);
                    }
                }
            }
        }
    }
}

#[test]
fn mathop_pseudo_random_sweep() {
    let _g = global_lock();
    // Deterministic xorshift so failures are reproducible.
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 32) as u32 as i32
    };
    let mut checked = 0usize;
    for _ in 0..40_000 {
        let (p1, p2, p3, p4) = (next(), next(), next(), next());
        // Mix in narrower magnitudes so arithmetic often stays in range.
        let scale = |v: i32, m: i32| if m == 0 { v } else { v % m };
        for m in [0, 1_000_000, 1000, 50] {
            let (a, b, c, d) = (scale(p1, m), scale(p2, m), scale(p3, m), scale(p4, m));
            if is_ub_free(a, b, c, d) {
                compare_mathop(a, b, c, d);
                checked += 1;
            }
        }
        if checked > 6000 {
            break;
        }
    }
    assert!(checked > 1000, "only {checked} random cases were checked");
}

#[test]
fn mathop_history_counter_saturates_identically() {
    let _g = global_lock();
    // Each call appends two entries, so the printed "History entries" line
    // climbs 2 at a time and then sticks at 10. Because the C and Rust statics
    // are separate, this only matches if the saturation logic is identical.
    let mut c_lines = Vec::new();
    let mut r_lines = Vec::new();
    for i in 0..12 {
        let b = both();
        let (_, cout) = capture_stdout(|| unsafe { (b.c.mathop)(i + 1, i + 2, i, i) });
        let (_, rout) = capture_stdout(|| unsafe { (b.rust.mathop)(i + 1, i + 2, i, i) });
        let pick = |o: &[u8]| {
            String::from_utf8_lossy(o)
                .lines()
                .find(|l| l.starts_with("History entries:"))
                .unwrap_or("<missing>")
                .to_string()
        };
        c_lines.push(pick(&cout));
        r_lines.push(pick(&rout));
    }
    assert_eq!(c_lines, r_lines, "history counter progression differs");
    assert_eq!(
        c_lines.last().map(String::as_str),
        Some("History entries: 10"),
        "counter should be saturated at 10 by now: {c_lines:?}"
    );
}

#[test]
fn mathop_stdout_format_shape() {
    let _g = global_lock();
    let b = both();
    let (_, out) = capture_stdout(|| unsafe { (b.rust.mathop)(7, 3, 1, 1) });
    let text = String::from_utf8_lossy(&out).to_string();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "expected 4 printed lines, got {lines:?}");
    assert!(lines[0].starts_with("Computation performed at timestamp: "));
    assert!(lines[1].starts_with("Operation priority: "));
    assert!(lines[2].starts_with("History entries: "));
    assert!(lines[3].starts_with("Final result: "));
    assert!(text.ends_with('\n'));
}
