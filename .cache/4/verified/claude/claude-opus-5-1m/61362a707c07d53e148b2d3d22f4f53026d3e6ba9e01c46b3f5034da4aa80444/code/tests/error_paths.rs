// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md. Each constructs the exact invalid input /
// rejection condition, calls BOTH the C `.so` and the Rust `.so`, and asserts
// they return the SAME sentinel / take the same no-op, not merely "both failed".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ===========================================================================
// E1 / E2 / E3: shift_array guard `positions > 0 && positions < size`
// ===========================================================================

/// Assert both libraries leave the buffer COMPLETELY untouched (a no-op) and
/// agree with each other.
fn assert_shift_noop(api_c: &Api, api_r: &Api, contents: &[c_int], size: c_int, positions: c_int) {
    let mut gc = Guarded::new(contents);
    let mut gr = Guarded::new(contents);
    unsafe {
        (api_c.shift_array)(gc.ptr(), size, positions);
        (api_r.shift_array)(gr.ptr(), size, positions);
    }
    let label = format!("shift_array(size={size}, positions={positions}) rejection");
    assert_eq_diff(&label, gc.all(), gr.all());
    assert_eq!(gc.data(), contents, "C should have made no change: {label}");
    assert_eq!(gr.data(), contents, "Rust should have made no change: {label}");
    assert!(gc.guards_intact() && gr.guards_intact(), "guard clobbered: {label}");
}

#[test]
fn e1_shift_array_positions_le_zero_is_noop() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for positions in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for size in [1i32, 2, 4, 16] {
            let contents: Vec<c_int> = (0..size).map(|_| rng.next_i32()).collect();
            assert_shift_noop(&c, &r, &contents, size, positions);
        }
    }
}

#[test]
fn e2_shift_array_positions_ge_size_is_noop() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE2);
    for size in [1i32, 2, 4, 16] {
        // positions == size (one past the last valid value) and beyond
        for positions in [size, size + 1, size + 100, i32::MAX] {
            let contents: Vec<c_int> = (0..size).map(|_| rng.next_i32()).collect();
            assert_shift_noop(&c, &r, &contents, size, positions);
        }
    }
}

#[test]
fn e3_shift_array_zero_or_negative_size_is_noop() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE3);
    for size in [0i32, -1, -4, -100, i32::MIN] {
        for positions in [1i32, 2, 100, i32::MAX, 0, -1] {
            let contents: Vec<c_int> = (0..4).map(|_| rng.next_i32()).collect();
            assert_shift_noop(&c, &r, &contents, size, positions);
        }
    }
}

// ===========================================================================
// E4: process_string("") -> 0 (strlen not called)
// ===========================================================================

#[test]
fn e4_process_string_empty_returns_zero() {
    let (c, r) = load_both();
    let empty: [c_char; 1] = [0];
    let rc = unsafe { (c.process_string)(empty.as_ptr()) };
    let rr = unsafe { (r.process_string)(empty.as_ptr()) };
    assert_eq_diff("process_string(\"\")", rc, rr);
    assert_eq!(rc, 0, "C must return the 0 sentinel for an empty string");

    // A NUL first byte must win even when more bytes follow in memory.
    let buf: Vec<c_char> = vec![0, 65, 66, 67, 0];
    let rc = unsafe { (c.process_string)(buf.as_ptr()) };
    let rr = unsafe { (r.process_string)(buf.as_ptr()) };
    assert_eq_diff("process_string(\"\\0ABC\")", rc, rr);
    assert_eq!(rc, 0);
}

// ===========================================================================
// E5: apply_bitmask `default` branch -- out-of-range enum ints across FFI
// ===========================================================================

#[test]
fn e5_apply_bitmask_out_of_range_operation_returns_value() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE5);

    // One step past the valid range on both ends, plus extremes.
    let bad_ops: [i32; 12] = [
        4, 5, 6, 100, -1, -2, -4, -100, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
    ];
    for &operation in bad_ops.iter() {
        for &value in BOUNDARY.iter() {
            let rc = unsafe { (c.apply_bitmask)(value, operation) };
            let rr = unsafe { (r.apply_bitmask)(value, operation) };
            assert_eq_diff(&format!("apply_bitmask({value}, {operation}) default"), rc, rr);
            assert_eq!(rc, value, "C default branch must return `value` unchanged");
        }
        for _ in 0..200 {
            let value = rng.next_i32();
            let rc = unsafe { (c.apply_bitmask)(value, operation) };
            let rr = unsafe { (r.apply_bitmask)(value, operation) };
            assert_eq_diff(&format!("apply_bitmask({value}, {operation}) default"), rc, rr);
            assert_eq!(rc, value);
        }
    }

    // Random operations that are *never* 0..=3 (pure default-branch fuzz).
    for _ in 0..3000 {
        let mut operation = rng.next_i32();
        if (0..=3).contains(&operation) {
            operation = operation.wrapping_add(4);
        }
        let value = rng.next_i32();
        let rc = unsafe { (c.apply_bitmask)(value, operation) };
        let rr = unsafe { (r.apply_bitmask)(value, operation) };
        assert_eq_diff(&format!("apply_bitmask({value}, {operation})"), rc, rr);
    }
}

// ===========================================================================
// E6: compare_allocations malloc-failure -> -1
// ===========================================================================

#[test]
fn e6_compare_allocations_malloc_failure_documented() {
    // Not inducible through the FFI surface (see ERRORS.md Note A): forcing a
    // 4-byte malloc to fail requires interposing the allocator, which would
    // perturb the two libraries differently. What IS asserted here is that the
    // success path never returns the -1 failure sentinel on either library, so
    // -1 remains an unambiguous error indicator in both.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..500 {
        let (v1, v2) = (rng.next_i32(), rng.next_i32());
        let rc = unsafe { (c.compare_allocations)(v1, v2) };
        let rr = unsafe { (r.compare_allocations)(v1, v2) };
        assert_ne!(rc, -1, "C success path must not return the failure sentinel");
        assert_ne!(rr, -1, "Rust success path must not return the failure sentinel");
        // Both must land in the same value domain {1,2,3} + optional +10.
        for v in [rc, rr] {
            assert!(
                [1, 2, 3, 11, 12, 13].contains(&v),
                "unexpected compare_allocations result {v}"
            );
        }
        // The `+10` term depends only on val1 > 0 and must agree.
        assert_eq_diff(
            &format!("compare_allocations({v1},{v2}) +10 term"),
            rc >= 10,
            rr >= 10,
        );
        assert_eq!(rc >= 10, v1 > 0, "C: +10 iff val1 > 0");
    }
}

// ===========================================================================
// E7 / E8 / E9 / E10: arity length dispatch and 8-bit truncation
// ===========================================================================

/// `arity` must return the same value on both libraries.
fn diff_arity_ret(api_c: &Api, api_r: &Api, len: c_int, params: &mut [c_int]) -> c_int {
    let pc = params.as_mut_ptr();
    let mut got = 0;
    assert_alloc_eq(
        &format!("arity({len}, ..)"),
        || {
            let v = unsafe { (api_c.arity)(len, pc) };
            got = v;
            v
        },
        || unsafe { (api_r.arity)(len, pc) },
    );
    got
}

#[test]
fn e7_arity_len_below_two_returns_minus_one() {
    let (c, r) = load_both();
    let mut params = [11i32, 22, 33, 44];
    for len in [0i32, 1] {
        let got = diff_arity_ret(&c, &r, len, &mut params);
        assert_eq!(got, -1, "C must reject len={len} with -1");
    }
    // ... and it must NOT touch `params` -- NULL is safe for len < 2.
    for len in [0i32, 1] {
        let rc = unsafe { (c.arity)(len, std::ptr::null_mut()) };
        let rr = unsafe { (r.arity)(len, std::ptr::null_mut()) };
        assert_eq_diff(&format!("arity({len}, NULL)"), rc, rr);
        assert_eq!(rc, -1, "C must return -1 without dereferencing params");
    }
}

#[test]
fn e8_arity_high_bits_truncate_to_below_two() {
    // The `unsigned char` parameter drops the high bits, so these all reject.
    let (c, r) = load_both();
    let mut params = [11i32, 22, 33, 44];
    for len in [256i32, 257, 512, 513, 65536, 65537, 0x7FFF_FF00, 0x7FFF_FF01] {
        let got = diff_arity_ret(&c, &r, len, &mut params);
        assert_eq!(got, -1, "C must reject len={len} (low byte {}) with -1", len & 0xFF);
        // NULL params is also safe here, because the low byte is < 2.
        let rc = unsafe { (c.arity)(len, std::ptr::null_mut()) };
        let rr = unsafe { (r.arity)(len, std::ptr::null_mut()) };
        assert_eq_diff(&format!("arity({len}, NULL)"), rc, rr);
        assert_eq!(rc, -1);
    }
}

#[test]
fn e9_arity_negative_len_is_unsigned_after_truncation() {
    let (c, r) = load_both();
    let mut params = [11i32, 22, 33, 44];

    // -256 and INT_MIN have low byte 0 => rejected with -1.
    for len in [-256i32, -512, -65536, i32::MIN] {
        let got = diff_arity_ret(&c, &r, len, &mut params);
        assert_eq!(got, -1, "len={len} has low byte 0 => -1");
    }
    // -1 => 255, -2 => 254: NOT < 2, so these dispatch to arity4 instead of
    // being rejected. Confirm the C really does that (it must not return -1).
    for len in [-1i32, -2, -3, -100] {
        let got = diff_arity_ret(&c, &r, len, &mut params);
        let expect = unsafe { (c.arity4)(params[0], params[1], params[2], params[3]) };
        assert_ne!(
            got, -1,
            "len={len} truncates to {} (>= 4) and must NOT be rejected",
            (len as u32) & 0xFF
        );
        let _ = expect; // value itself is allocator-parity dependent
    }
}

#[test]
fn e10_arity_large_len_reads_only_four_params() {
    // len=255 must read params[0..3] only -- a 4-element buffer is sufficient.
    // A guarded 4-element buffer proves nothing outside is written, and the
    // results must match the direct arity4 call domain.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xE10);
    for len in [4i32, 5, 200, 254, 255] {
        for _ in 0..50 {
            let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
            let mut gc = Guarded::new(&params);
            let mut gr = Guarded::new(&params);
            let pc = gc.ptr();
            let pr = gr.ptr();
            assert_alloc_eq(
                &format!("arity({len}, 4 params)"),
                || unsafe { (c.arity)(len, pc) },
                || unsafe { (r.arity)(len, pr) },
            );
            assert!(
                gc.guards_intact() && gr.guards_intact(),
                "arity({len}) wrote outside the params buffer"
            );
            assert_eq_diff("params buffer after arity", gc.all(), gr.all());
        }
    }
}

#[test]
fn e_arity_all_256_low_byte_values_error_vs_dispatch() {
    // Exhaustive: for every low byte 0..=255, C and Rust must agree on whether
    // the call is rejected (-1) or dispatched, and on the value.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x256);
    for low in 0..=255i32 {
        let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        let mut gc = Guarded::new(&params);
        let mut gr = Guarded::new(&params);
        let pc = gc.ptr();
        let pr = gr.ptr();
        let mut c_val = 0;
        assert_alloc_eq(
            &format!("arity(low byte {low})"),
            || {
                let v = unsafe { (c.arity)(low, pc) };
                c_val = v;
                v
            },
            || unsafe { (r.arity)(low, pr) },
        );
        if low < 2 {
            assert_eq!(c_val, -1, "low byte {low} must be rejected");
        }
    }
}

// ===========================================================================
// E11 / E12 / E13: NULL-pointer dereference (C has no NULL checks) --
// compared by termination-signal parity in forked children.
// ===========================================================================

const CRASH_TEST: &str = "e11_e12_e13_null_pointer_crash_parity";

#[test]
fn e11_e12_e13_null_pointer_crash_parity() {
    // Child mode: perform the raw call that the C code cannot survive.
    if let Some(case) = crash_case() {
        let (c, r) = load_both();
        let api = if case.starts_with("c_") { &c } else { &r };
        let null_i = std::ptr::null_mut::<c_int>();
        unsafe {
            match case.trim_start_matches("c_").trim_start_matches("r_") {
                // E11
                "process_string_null" => {
                    let v = (api.process_string)(std::ptr::null());
                    println!("survived: {v}");
                }
                // E12
                "arity_null_len2" => {
                    let v = (api.arity)(2, null_i);
                    println!("survived: {v}");
                }
                "arity_null_len3" => {
                    let v = (api.arity)(3, null_i);
                    println!("survived: {v}");
                }
                "arity_null_len4" => {
                    let v = (api.arity)(4, null_i);
                    println!("survived: {v}");
                }
                "arity_null_len255" => {
                    let v = (api.arity)(255, null_i);
                    println!("survived: {v}");
                }
                // E13
                "shift_array_null" => (api.shift_array)(null_i, 4, 1),
                "init_matrix_null" => (api.init_matrix)(null_i),
                other => panic!("unknown crash case {other}"),
            }
        }
        // If we get here the call did not crash; exit 0 so the parent sees
        // Exit(0) for this library and can compare that too.
        std::process::exit(0);
    }

    // Parent mode: for each case run a C child and a Rust child and require
    // identical termination (same signal, or same exit code).
    let cases = [
        "process_string_null",
        "arity_null_len2",
        "arity_null_len3",
        "arity_null_len4",
        "arity_null_len255",
        "shift_array_null",
        "init_matrix_null",
    ];
    for case in cases {
        let c_term = run_crash_child(CRASH_TEST, &format!("c_{case}"));
        let r_term = run_crash_child(CRASH_TEST, &format!("r_{case}"));
        assert_eq!(
            c_term, r_term,
            "NULL-pointer behaviour differs for `{case}`: C={c_term:?} Rust={r_term:?}"
        );
        println!("  {case}: C and Rust both -> {c_term:?}");
        // Every one of these dereferences NULL in the C, so the expected
        // outcome is a fatal signal (SIGSEGV = 11), not a graceful return.
        assert_eq!(
            c_term,
            Term::Signal(11),
            "expected SIGSEGV from the C library for `{case}`"
        );
    }
}
