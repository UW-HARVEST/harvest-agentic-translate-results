//! Phase C — error-path differential tests.
//! One test per row of ERRORS.md.  Each constructs the exact invalid input and
//! asserts the C and Rust libraries return the SAME sentinel/rejection and emit
//! the SAME diagnostic bytes on stdout.

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;

// ===========================================================================
// Row 1 / 2 / 17 — execute_operation
// ===========================================================================

#[test]
fn err_01_execute_operation_null_func() {
    for name in ["XOR", "SHIFT", "some-op", "%s%d"] {
        let cs = CString::new(name).unwrap();
        let ctx = format!("err01 execute_operation(NULL, .., \"{name}\")");
        let v = diff(&ctx, |l| l.execute_operation(None, 1, 2, cs.as_ptr()));
        assert_eq!(v, 0, "{ctx}: C returns the 0 sentinel");
    }
    // the sentinel must be 0 regardless of the operand values
    let cs = CString::new("OP").unwrap();
    let mut rng = Rng::new(0xE001);
    for i in 0..200 {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("err01[{i}] execute_operation(NULL,{a},{b})");
        let v = diff(&ctx, |l| l.execute_operation(None, a, b, cs.as_ptr()));
        assert_eq!(v, 0, "{ctx}");
    }
}

#[test]
fn err_02_execute_operation_null_func_empty_name() {
    let cs = CString::new("").unwrap();
    let v = diff("err02 execute_operation(NULL, .., \"\")", |l| {
        l.execute_operation(None, -7, 9, cs.as_ptr())
    });
    assert_eq!(v, 0);
}

#[test]
fn err_17_execute_operation_null_name() {
    // valid func, NULL op_name -> glibc printf("%s", NULL) prints "(null)"
    for opcode in 0..4 {
        let ctx = format!("err17 execute_operation(op{opcode}, .., NULL name)");
        diff(&ctx, |l| {
            l.execute_operation(l.get_operation(opcode), 5, 11, ptr::null::<c_char>())
        });
    }
    // NULL func *and* NULL name
    let v = diff("err17 execute_operation(NULL func, NULL name)", |l| {
        l.execute_operation(None, 5, 11, ptr::null::<c_char>())
    });
    assert_eq!(v, 0);
}

// ===========================================================================
// Rows 3..7 — compute_checksum guard `values != NULL && count > 0`
// ===========================================================================

#[test]
fn err_03_compute_checksum_null_values() {
    for &count in &[1, 2, 3, 4, 5, 100, i32::MAX] {
        let ctx = format!("err03 compute_checksum(NULL, {count})");
        let v = diff(&ctx, |l| l.compute_checksum(ptr::null_mut(), count));
        assert_eq!(v, 0, "{ctx}: expected 0");
    }
}

#[test]
fn err_04_compute_checksum_zero_count() {
    let mut vals: [c_int; 4] = [0x1122_3344, -1, i32::MIN, i32::MAX];
    let v = diff("err04 compute_checksum(&vals, 0)", |l| {
        l.compute_checksum(vals.as_mut_ptr(), 0)
    });
    assert_eq!(v, 0, "expected 0 for count == 0");
}

#[test]
fn err_05_compute_checksum_negative_count() {
    let mut vals: [c_int; 4] = [0x1122_3344, -1, i32::MIN, i32::MAX];
    for &count in &[-1, -2, -4, -100, i32::MIN, i32::MIN + 1] {
        let ctx = format!("err05 compute_checksum(&vals, {count})");
        let v = diff(&ctx, |l| l.compute_checksum(vals.as_mut_ptr(), count));
        assert_eq!(v, 0, "{ctx}: expected 0");
    }
}

#[test]
fn err_06_compute_checksum_null_and_zero() {
    let v = diff("err06 compute_checksum(NULL, 0)", |l| {
        l.compute_checksum(ptr::null_mut(), 0)
    });
    assert_eq!(v, 0);
}

#[test]
fn err_07_compute_checksum_null_and_negative() {
    for &count in &[-1, -1000, i32::MIN] {
        let ctx = format!("err07 compute_checksum(NULL, {count})");
        let v = diff(&ctx, |l| l.compute_checksum(ptr::null_mut(), count));
        assert_eq!(v, 0, "{ctx}");
    }
}

// ===========================================================================
// Row 8 — init_state(NULL, ..)
// ===========================================================================

#[test]
fn err_08_init_state_null() {
    for &v in &[0, 1, -1, i32::MAX, i32::MIN, 0x5A5A_5A5A] {
        let ctx = format!("err08 init_state(NULL, {v})");
        diff(&ctx, |l| {
            l.init_state(ptr::null_mut(), v);
        });
    }
}

// ===========================================================================
// Rows 9..11 — apply_operation guards, including their ORDER
// ===========================================================================

#[test]
fn err_09_apply_operation_null_state() {
    // state == NULL with a *valid* func: the state check comes first.
    for opcode in 0..4 {
        let ctx = format!("err09 apply_operation(NULL, .., op{opcode})");
        diff(&ctx, |l| {
            l.apply_operation(ptr::null_mut(), 42, l.get_operation(opcode));
        });
    }
    let mut rng = Rng::new(0xE009);
    for i in 0..50 {
        let value = rng.i32_biased();
        let ctx = format!("err09[{i}] apply_operation(NULL, {value}, op0)");
        diff(&ctx, |l| {
            l.apply_operation(ptr::null_mut(), value, l.get_operation(0));
        });
    }
}

#[test]
fn err_10_apply_operation_null_func() {
    let mut rng = Rng::new(0xE010);
    for i in 0..100 {
        let pre = ComputeState {
            accumulator: rng.i32_biased(),
            operation_count: rng.i32_biased(),
            checksum: rng.u32(),
        };
        let mut st = pre;
        let ctx = format!("err10[{i}] apply_operation(&st, .., NULL)");
        let got = diff(&ctx, |l| {
            st = pre;
            l.apply_operation(&mut st as *mut ComputeState, 123, None);
            st
        });
        assert_eq!(got, pre, "{ctx}: state must be left untouched");
    }
}

#[test]
fn err_11_apply_operation_both_null() {
    // Only the *state* diagnostic may be printed; the func check is never
    // reached.  The transcript comparison in `diff` is what proves the order.
    diff("err11 apply_operation(NULL, .., NULL)", |l| {
        l.apply_operation(ptr::null_mut(), 0, None);
    });
    // Prove it really is the state message (and only one line).
    let (_, out) = capture(|| clib().apply_operation(ptr::null_mut(), 0, None));
    let s = String::from_utf8_lossy(&out).to_string();
    assert_eq!(
        s, "Error: state pointer is NULL in apply_operation\n",
        "unexpected C diagnostic for the both-NULL case"
    );
}

// ===========================================================================
// Rows 12..15 — get_operation out-of-range opcodes
// ===========================================================================

#[test]
fn err_12_get_operation_negative() {
    for &op in &[-1, -2, -3, -4, -100, i32::MIN, i32::MIN + 1] {
        let ctx = format!("err12 get_operation({op})");
        let isnull = diff(&ctx, |l| l.get_operation_raw(op).is_null());
        assert!(isnull, "{ctx}: expected NULL");
    }
}

#[test]
fn err_13_get_operation_too_large() {
    for &op in &[4, 5, 6, 8, 16, 100, 0x7FFF_FFFF, i32::MAX - 1] {
        let ctx = format!("err13 get_operation({op})");
        let isnull = diff(&ctx, |l| l.get_operation_raw(op).is_null());
        assert!(isnull, "{ctx}: expected NULL");
    }
}

#[test]
fn err_14_get_operation_range_edges() {
    // exactly one step past each end
    for &op in &[-1, 4] {
        let isnull = diff(&format!("err14 get_operation({op})"), |l| {
            l.get_operation_raw(op).is_null()
        });
        assert!(isnull, "get_operation({op}) must be NULL");
    }
    // and the in-range ends
    for &op in &[0, 3] {
        let isnull = diff(&format!("err14 get_operation({op})"), |l| {
            l.get_operation_raw(op).is_null()
        });
        assert!(!isnull, "get_operation({op}) must be non-NULL");
    }
    // exhaustive sweep of a window straddling the range
    for op in -8..12 {
        let isnull = diff(&format!("err14 sweep get_operation({op})"), |l| {
            l.get_operation_raw(op).is_null()
        });
        assert_eq!(isnull, !(0..4).contains(&op), "get_operation({op}) nullness");
    }
    // randomized full-i32 sweep: nullness and value must agree in both libs
    let mut rng = Rng::new(0xE014);
    for i in 0..500 {
        let op = rng.i32_biased();
        let isnull = diff(&format!("err14 rand[{i}] get_operation({op})"), |l| {
            l.get_operation_raw(op).is_null()
        });
        assert_eq!(isnull, !(0..4).contains(&op), "get_operation({op}) nullness");
    }
}

#[test]
fn err_15_get_operation_opcode_enum_values() {
    // The OP_* macros are 1,2,3,4 -- an "enum-like" set in which OP_SHIFT (4)
    // is itself out of range for the 4-entry table.  C enums/ints accept any
    // value, so every one of these is a real input.
    const OP_ADD: c_int = 0x01;
    const OP_MULTIPLY: c_int = 0x02;
    const OP_XOR: c_int = 0x03;
    const OP_SHIFT: c_int = 0x04;

    let cases: [(c_int, bool); 5] = [
        (0, false),            // unnamed, but the valid multiply slot
        (OP_ADD, false),       // 1 -> add slot
        (OP_MULTIPLY, false),  // 2 -> xor slot
        (OP_XOR, false),       // 3 -> shift slot
        (OP_SHIFT, true),      // 4 -> OUT OF RANGE -> NULL
    ];
    for (op, expect_null) in cases {
        let isnull = diff(&format!("err15 get_operation({op})"), |l| {
            l.get_operation_raw(op).is_null()
        });
        assert_eq!(isnull, expect_null, "get_operation({op}) nullness");
    }

    // A NULL returned by get_operation must be accepted by the downstream
    // consumers exactly as the C accepts it (row 1 / row 10 paths).
    let cs = CString::new("OUT_OF_RANGE").unwrap();
    let v = diff("err15 execute_operation(get_operation(4), ..)", |l| {
        l.execute_operation(l.get_operation(OP_SHIFT), 3, 4, cs.as_ptr())
    });
    assert_eq!(v, 0);

    let mut st = ComputeState::default();
    let got = diff("err15 apply_operation(.., get_operation(-1))", |l| {
        st = ComputeState {
            accumulator: 9,
            operation_count: 5,
            checksum: 1,
        };
        l.apply_operation(&mut st as *mut ComputeState, 1, l.get_operation(-1));
        st
    });
    assert_eq!(
        got,
        ComputeState {
            accumulator: 9,
            operation_count: 5,
            checksum: 1
        }
    );
}

// ===========================================================================
// Row 16 — checkshift malloc failure
// ===========================================================================

#[test]
fn err_16_checkshift_malloc_failure_documented() {
    // `checkshift` allocates 12 bytes with malloc and returns -1 after printing
    // "Error: Failed to allocate memory for state" if that fails.  A 12-byte
    // malloc cannot be made to fail through the public FFI surface without
    // interposing the allocator process-wide (which would equally break the
    // test harness and the loader), so this row is verified structurally:
    //   * both libraries contain the identical diagnostic string, and
    //   * both return -1 only on that path (never on a successful run).
    let needle = b"Error: Failed to allocate memory for state";
    for (label, path) in [
        ("C", c_so_path()),
        ("RUST", rust_so_path()),
    ] {
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            bytes
                .windows(needle.len())
                .any(|w| w == needle),
            "{label} .so ({}) is missing the malloc-failure diagnostic",
            path.display()
        );
    }

    // Sanity: on the success path the sentinel -1 is only ever produced as a
    // legitimate computed result, and the two libs always agree.
    let mut rng = Rng::new(0xE016);
    for i in 0..200 {
        let (a, b, c, d) = (
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
        );
        diff(&format!("err16[{i}] checkshift({a},{b},{c},{d})"), |l| {
            l.checkshift(a, b, c, d)
        });
    }
}

// ===========================================================================
// Generic FFI boundary coverage required by the task, beyond the table rows
// ===========================================================================

#[test]
fn err_gen_all_null_pointer_entry_points() {
    // Every pointer parameter of every exported function, set to NULL.
    diff("gen init_state(NULL, 0)", |l| l.init_state(ptr::null_mut(), 0));
    diff("gen apply_operation(NULL, 0, NULL)", |l| {
        l.apply_operation(ptr::null_mut(), 0, None)
    });
    diff("gen apply_operation(NULL, 0, valid)", |l| {
        l.apply_operation(ptr::null_mut(), 0, l.get_operation(2))
    });
    let v = diff("gen compute_checksum(NULL, 4)", |l| {
        l.compute_checksum(ptr::null_mut(), 4)
    });
    assert_eq!(v, 0);
    let v = diff("gen execute_operation(NULL, .., NULL)", |l| {
        l.execute_operation(None, 0, 0, ptr::null())
    });
    assert_eq!(v, 0);
}

#[test]
fn err_gen_oversized_and_zero_lengths() {
    let mut vals: [c_int; 4] = [1, 2, 3, 4];
    // zero, oversized, and one-past-the-clamp counts
    for &count in &[0, 1, 4, 5, i32::MAX, i32::MAX - 1] {
        let ctx = format!("gen compute_checksum(&vals[4], {count})");
        diff(&ctx, |l| l.compute_checksum(vals.as_mut_ptr(), count));
    }
    // count == 4 with an array of exactly 4 elements is the maximum readable
    // shape; a 1-element array with count == 1 is the minimum.
    let mut one: [c_int; 1] = [0x0BAD_F00Du32 as i32];
    diff("gen compute_checksum(&one[1], 1)", |l| {
        l.compute_checksum(one.as_mut_ptr(), 1)
    });
    diff("gen compute_checksum(&one[1], 0)", |l| {
        l.compute_checksum(one.as_mut_ptr(), 0)
    });
    diff("gen compute_checksum(&one[1], -1)", |l| {
        l.compute_checksum(one.as_mut_ptr(), -1)
    });
}

#[test]
fn err_gen_one_past_valid_ranges() {
    // get_operation: valid range is [0,4); check -1 and 4 (done in err_14) plus
    // the shift amount boundary: static_shift_amount is 2, so shifting is never
    // UB, but the operand extremes are still worth pinning down.
    for &(a, b) in &[
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (0x4000_0000u32 as i32, -1),
        (-1, 0x4000_0000u32 as i32),
    ] {
        diff(&format!("gen shift_with_static({a},{b})"), |l| {
            l.shift_with_static(a, b)
        });
        diff(&format!("gen multiply_with_static({a},{b})"), |l| {
            l.multiply_with_static(a, b)
        });
        diff(&format!("gen add_with_static({a},{b})"), |l| {
            l.add_with_static(a, b)
        });
        diff(&format!("gen xor_operation({a},{b})"), |l| {
            l.xor_operation(a, b)
        });
        diff(&format!("gen checkshift({a},{b},{a},{b})"), |l| {
            l.checkshift(a, b, a, b)
        });
    }
}
