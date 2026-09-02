//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries: null
//! pointers, zero/oversized lengths, values one past a valid range, and
//! out-of-range "enum" values crossing the FFI boundary (`operation` is a
//! plain `int` in C, so any value is a real input).
//!
//! Each test asserts C and Rust return the SAME sentinel/error value AND print
//! the same diagnostics, not merely that both failed.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

// ===========================================================================
// Row 1 / Row 24 — allocation failure of the 24-byte state, and `confusion`'s
// resulting `-1`.
//
// Not triggerable from the public API: `create_state` requests a fixed 24
// bytes, and `confusion` always passes capacity 128. Recorded here so the row
// is accounted for rather than silently dropped. The reachable analogue (the
// SECOND malloc failing) is covered by rows 2-4.
// ===========================================================================

#[test]
fn row01_row24_state_malloc_failure_is_unreachable() {
    let p = pair();
    // Both implementations succeed for the sizes the API can request, so the
    // NULL-return branch cannot be observed. Assert that agreement.
    let (c_null, _) = capture(|| unsafe {
        let s = (p.c.create_state)(0, 128);
        let n = s.is_null();
        (p.c.destroy_state)(s);
        n
    });
    let (rs_null, _) = capture(|| unsafe {
        let s = (p.rs.create_state)(0, 128);
        let n = s.is_null();
        (p.rs.destroy_state)(s);
        n
    });
    assert_ret_eq("row01 24-byte malloc", c_null, rs_null);
    assert!(!c_null, "24-byte malloc is expected to succeed");

    // And `confusion` therefore never returns the -1 sentinel from that branch.
    let (rc, oc) = capture(|| unsafe { (p.c.confusion)(0, 0, 0, 0) });
    let (rr, or) = capture(|| unsafe { (p.rs.confusion)(0, 0, 0, 0) });
    assert_ret_eq("row24 confusion", rc, rr);
    assert_out_eq("row24 confusion", &oc, &or);
}

// ===========================================================================
// Rows 2, 3, 4, 5, 6 — `create_state` capacity boundaries
// ===========================================================================

/// Compares `create_state` for one capacity. `read_buffer` is false when the C
/// leaves the buffer uninitialized (capacity 0), where contents are
/// indeterminate and only NULL-ness is a defined observable.
fn compare_create(ctx: &str, initial: c_int, capacity: c_int, read_buffer: bool) {
    let p = pair();
    let ((c_null, c_snap), oc) = capture(|| unsafe {
        let s = (p.c.create_state)(initial, capacity);
        let snap = if s.is_null() {
            None
        } else {
            Some(p.c.snapshot(s, read_buffer))
        };
        (p.c.destroy_state)(s);
        (s.is_null(), snap)
    });
    let ((rs_null, rs_snap), or) = capture(|| unsafe {
        let s = (p.rs.create_state)(initial, capacity);
        let snap = if s.is_null() {
            None
        } else {
            Some(p.rs.snapshot(s, read_buffer))
        };
        (p.rs.destroy_state)(s);
        (s.is_null(), snap)
    });
    assert_ret_eq(&format!("{ctx} nullness"), c_null, rs_null);
    match (c_snap, rs_snap) {
        (Some(a), Some(b)) => assert_snap_eq(ctx, &a, &b),
        (None, None) => {}
        _ => unreachable!("nullness already compared"),
    }
    assert_out_eq(ctx, &oc, &or);
}

#[test]
fn row02_create_state_negative_capacity_returns_null() {
    for cap in [-1, -2, -16, -128, -1000, -65536, i32::MIN + 1] {
        compare_create(&format!("row02 cap={cap}"), 42, cap, true);
    }
    // The rejection must actually happen (not a happy path that agrees).
    let p = pair();
    let ((s, _), out) = capture(|| unsafe {
        let s = (p.c.create_state)(42, -1);
        (s.is_null(), ())
    });
    assert!(s, "negative capacity must fail the buffer allocation");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Error: Failed to allocate buffer\n",
        "expected the buffer-allocation error message"
    );
}

#[test]
fn row03_create_state_capacity_int_min() {
    compare_create("row03 cap=INT_MIN", 0, i32::MIN, true);
    compare_create("row03 cap=INT_MIN initial=INT_MAX", i32::MAX, i32::MIN, true);
}

#[test]
fn row04_create_state_capacity_int_max() {
    // May or may not succeed depending on overcommit; the requirement is that
    // C and Rust agree, including the printed diagnostics.
    compare_create("row04 cap=INT_MAX", 7, i32::MAX, true);
    compare_create("row04 cap=INT_MAX-1", 7, i32::MAX - 1, true);
}

#[test]
fn row05_create_state_capacity_zero() {
    // malloc(0) returns a non-NULL minimal block and snprintf(buf, 0, ..)
    // writes nothing, so the buffer stays uninitialized: compare only the
    // defined observables.
    for initial in [0, 1, -1, i32::MIN, i32::MAX] {
        compare_create(&format!("row05 cap=0 initial={initial}"), initial, 0, false);
    }
    let p = pair();
    let (c_null, _) = capture(|| unsafe {
        let s = (p.c.create_state)(0, 0);
        let n = s.is_null();
        (p.c.destroy_state)(s);
        n
    });
    assert!(!c_null, "glibc malloc(0) is expected to return non-NULL");
}

#[test]
fn row06_create_state_capacity_truncating() {
    for cap in 1..=17 {
        for initial in [0, 5, 12345, -1, -99999, i32::MIN, i32::MAX] {
            compare_create(&format!("row06 cap={cap} initial={initial}"), initial, cap, true);
        }
    }
}

// ===========================================================================
// Rows 7, 8 — destroy_state guards
// ===========================================================================

#[test]
fn row07_destroy_state_null_is_noop() {
    let p = pair();
    let ((), oc) = capture(|| unsafe { (p.c.destroy_state)(std::ptr::null_mut()) });
    let ((), or) = capture(|| unsafe { (p.rs.destroy_state)(std::ptr::null_mut()) });
    assert_out_eq("row07 destroy(NULL)", &oc, &or);
    assert!(oc.is_empty(), "destroy_state(NULL) must print nothing");
}

#[test]
fn row08_destroy_state_null_buffer() {
    let p = pair();
    let ((), oc) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, 0, None, 128);
        (p.c.destroy_state)(s);
    });
    let ((), or) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, 0, None, 128);
        (p.rs.destroy_state)(s);
    });
    assert_out_eq("row08 destroy(buffer=NULL)", &oc, &or);
    assert!(oc.is_empty());
}

// ===========================================================================
// Rows 9, 10 — process_buffer null guards (returns -1)
// ===========================================================================

#[test]
fn row09_process_buffer_null_state_returns_minus_one() {
    let p = pair();
    for target in [0i8, b'0' as i8, b':' as i8, -1i8, i8::MIN, i8::MAX] {
        let (rc, oc) = capture(|| unsafe { (p.c.process_buffer)(std::ptr::null_mut(), target) });
        let (rr, or) = capture(|| unsafe { (p.rs.process_buffer)(std::ptr::null_mut(), target) });
        assert_ret_eq(&format!("row09 target={target}"), rc, rr);
        assert_eq!(rc, -1, "C must return the -1 sentinel");
        assert_out_eq(&format!("row09 target={target}"), &oc, &or);
        assert_eq!(
            String::from_utf8_lossy(&oc),
            "Error: Null pointer in process_buffer\n"
        );
    }
}

#[test]
fn row10_process_buffer_null_buffer_returns_minus_one() {
    let p = pair();
    for target in [0i8, b'0' as i8, -1i8, i8::MIN, i8::MAX] {
        let (rc, oc) = capture(|| unsafe {
            let s = make_state(0x0000_7b05, 0, None, 128);
            let r = (p.c.process_buffer)(s, target);
            drop_state(s);
            r
        });
        let (rr, or) = capture(|| unsafe {
            let s = make_state(0x0000_7b05, 0, None, 128);
            let r = (p.rs.process_buffer)(s, target);
            drop_state(s);
            r
        });
        assert_ret_eq(&format!("row10 target={target}"), rc, rr);
        assert_eq!(rc, -1);
        assert_out_eq(&format!("row10 target={target}"), &oc, &or);
        assert_eq!(
            String::from_utf8_lossy(&oc),
            "Error: Null pointer in process_buffer\n"
        );
    }
}

// ===========================================================================
// Rows 11, 12, 13, 14 — process_buffer scan-termination conditions
// ===========================================================================

/// Runs `process_buffer` against a hand-built buffer holding arbitrary bytes —
/// content `create_state` can never produce.
fn compare_process_raw(ctx: &str, bytes: &[u8], target: u8) {
    let p = pair();
    let (rc, oc) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, 0, Some(bytes), bytes.len() as c_int + 1);
        let r = (p.c.process_buffer)(s, target as i8);
        drop_state(s);
        r
    });
    let (rr, or) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, 0, Some(bytes), bytes.len() as c_int + 1);
        let r = (p.rs.process_buffer)(s, target as i8);
        drop_state(s);
        r
    });
    assert_ret_eq(ctx, rc, rr);
    assert_out_eq(ctx, &oc, &or);
}

#[test]
fn row11_process_buffer_target_absent() {
    compare_process_raw("row11 absent", b"State:1:Mode:3", b'Z');
    compare_process_raw("row11 absent empty-ish", b"abc", b'z');
    let p = pair();
    let (rc, oc) = capture(|| unsafe {
        let s = (p.c.create_state)(1, 128);
        let r = (p.c.process_buffer)(s, b'Z' as i8);
        (p.c.destroy_state)(s);
        r
    });
    let (rr, or) = capture(|| unsafe {
        let s = (p.rs.create_state)(1, 128);
        let r = (p.rs.process_buffer)(s, b'Z' as i8);
        (p.rs.destroy_state)(s);
        r
    });
    assert_ret_eq("row11 via create_state", rc, rr);
    assert_eq!(rc, 0, "absent target must yield count 0");
    assert_out_eq("row11 via create_state", &oc, &or);
    assert!(oc.is_empty(), "no Operation: line when nothing is found");
}

#[test]
fn row12_process_buffer_nul_target_never_found() {
    compare_process_raw("row12 nul", b"State:1:Mode:3", 0);
    compare_process_raw("row12 nul on empty", b"", 0);
    let p = pair();
    let (rc, _) = capture(|| unsafe {
        let s = (p.c.create_state)(1, 128);
        let r = (p.c.process_buffer)(s, 0);
        (p.c.destroy_state)(s);
        r
    });
    assert_eq!(rc, 0, "'\\0' lies outside strlen's span");
}

#[test]
fn row13_process_buffer_high_bit_target() {
    // memchr compares as unsigned char: a negative `char` must not match an
    // ASCII byte via sign extension.
    for t in [0x80u8, 0xFF, 0xC0, 0xB0, 0xAA] {
        compare_process_raw(&format!("row13 target={t:#04x} ascii"), b"State:1:Mode:3", t);
    }
    // ...and must match when the byte really is present.
    compare_process_raw("row13 0xFF present", &[0x41, 0xFF, 0x42, 0xFF, 0x43], 0xFF);
    compare_process_raw("row13 0x80 present", &[0x80, 0x80, 0x80], 0x80);
}

#[test]
fn row14_process_buffer_empty_buffer() {
    compare_process_raw("row14 empty", b"", b'0');
    compare_process_raw("row14 empty nul", b"", 0);
    let p = pair();
    // capacity 1 -> snprintf writes just the terminator -> strlen == 0.
    let (rc, oc) = capture(|| unsafe {
        let s = (p.c.create_state)(5, 1);
        let r = (p.c.process_buffer)(s, b'5' as i8);
        (p.c.destroy_state)(s);
        r
    });
    let (rr, or) = capture(|| unsafe {
        let s = (p.rs.create_state)(5, 1);
        let r = (p.rs.process_buffer)(s, b'5' as i8);
        (p.rs.destroy_state)(s);
        r
    });
    assert_ret_eq("row14 cap=1", rc, rr);
    assert_eq!(rc, 0);
    assert_out_eq("row14 cap=1", &oc, &or);
}

/// Randomized arbitrary-content buffers: many occurrences, adjacent
/// occurrences, occurrences at the first and last position, and embedded high
/// bytes. This is the memchr loop's real stress test.
#[test]
fn process_buffer_randomized_arbitrary_buffers() {
    let mut rng = Rng::new(0x2001);
    for _ in 0..400 {
        let len = rng.below(40) as usize;
        // Draw from a small alphabet so repeats are common, plus raw bytes.
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            let b = match rng.below(3) {
                0 => b"ab:0"[rng.below(4) as usize],
                1 => rng.next_u8(),
                _ => 0xFF,
            };
            // A NUL would truncate the string; keep it out of the body.
            bytes.push(if b == 0 { b'.' } else { b });
        }
        for _ in 0..3 {
            let t = match rng.below(2) {
                0 if !bytes.is_empty() => bytes[rng.below(bytes.len() as u64) as usize],
                _ => rng.next_u8(),
            };
            compare_process_raw("randomized raw buffer", &bytes, t);
        }
    }
}

// ===========================================================================
// Rows 15, 16, 17, 18 — update_flags
// ===========================================================================

#[test]
fn row15_update_flags_null_state_is_silent_noop() {
    let p = pair();
    for param in [0, 1, -1, i32::MIN, i32::MAX, 0x1234_5678] {
        let ((), oc) = capture(|| unsafe { (p.c.update_flags)(std::ptr::null_mut(), param) });
        let ((), or) = capture(|| unsafe { (p.rs.update_flags)(std::ptr::null_mut(), param) });
        assert_out_eq(&format!("row15 param={param}"), &oc, &or);
        assert!(oc.is_empty(), "update_flags(NULL) must print nothing");
    }
}

/// Applies `update_flags` to a hand-built state with arbitrary starting
/// bit-field contents, then compares the FULL 32-bit storage unit. This is the
/// only way to observe whether the read-modify-write preserves `status` and
/// `reserved`, since `create_state` always initializes them to 15 and 0.
fn compare_update_raw(ctx: &str, flags_raw: u32, data_raw: u32, params: &[c_int]) {
    let p = pair();
    let (c_snap, oc) = capture(|| unsafe {
        let s = make_state(flags_raw, data_raw, Some(b"State:1:Mode:3"), 128);
        for &param in params {
            (p.c.update_flags)(s, param);
        }
        let snap = p.c.snapshot(s, true);
        drop_state(s);
        snap
    });
    let (rs_snap, or) = capture(|| unsafe {
        let s = make_state(flags_raw, data_raw, Some(b"State:1:Mode:3"), 128);
        for &param in params {
            (p.rs.update_flags)(s, param);
        }
        let snap = p.rs.snapshot(s, true);
        drop_state(s);
        snap
    });
    assert_snap_eq(ctx, &c_snap, &rs_snap);
    assert_out_eq(ctx, &oc, &or);
}

#[test]
fn row16_update_flags_negative_param_arithmetic_shift() {
    for param in [-1, -2, -8, -9, -16, -1000, -0x7FFF_FFFF] {
        compare_update_raw(&format!("row16 param={param}"), 0x0000_7b05, 0, &[param]);
    }
}

#[test]
fn row17_update_flags_param_int_min() {
    compare_update_raw("row17 INT_MIN", 0x0000_7b05, 0, &[i32::MIN]);
    compare_update_raw("row17 INT_MIN twice", 0x0000_7b05, 0, &[i32::MIN, i32::MIN]);
    compare_update_raw("row17 INT_MIN+1", 0x0000_7b05, 0, &[i32::MIN + 1]);
}

#[test]
fn row18_update_flags_counter_wraps_at_32() {
    // Start the counter at 31 so the very next call wraps it to 0.
    let flags_31 = (31u32 << 3) | 0x0000_7b05 & !0xF8;
    compare_update_raw("row18 counter=31 +1", flags_31, 0, &[0]);
    compare_update_raw("row18 counter=31 +2", flags_31, 0, &[0, 0]);
    let params: Vec<c_int> = (0..40).collect();
    compare_update_raw("row18 40 calls", 0x0000_7b05, 0, &params);
}

/// Randomized arbitrary starting bit-fields: catches mask errors that a
/// freshly created state cannot expose (non-zero `reserved`, `status != 15`).
#[test]
fn update_flags_preserves_unrelated_bitfields() {
    let mut rng = Rng::new(0x2002);
    for _ in 0..600 {
        let flags = rng.next_u64() as u32;
        let data = rng.next_u64() as u32;
        let n = 1 + rng.below(4) as usize;
        let params: Vec<c_int> = (0..n).map(|_| rng.interesting_i32()).collect();
        compare_update_raw("arbitrary starting flags", flags, data, &params);
    }
}

// ===========================================================================
// Rows 19, 20, 21, 22, 23 — confuse_types
// ===========================================================================

#[test]
fn row19_confuse_types_null_state_returns_zero() {
    let p = pair();
    for op in [-1, 0, 1, 2, 3, 4, i32::MIN, i32::MAX] {
        let (rc, oc) = capture(|| unsafe { (p.c.confuse_types)(std::ptr::null_mut(), op) });
        let (rr, or) = capture(|| unsafe { (p.rs.confuse_types)(std::ptr::null_mut(), op) });
        assert_ret_eq(&format!("row19 op={op}"), rc, rr);
        assert_eq!(rc, 0, "C returns 0 for a NULL state");
        assert_out_eq(&format!("row19 op={op}"), &oc, &or);
        assert!(oc.is_empty());
    }
}

fn compare_confuse_raw(ctx: &str, data_raw: u32, ops: &[c_int]) {
    let p = pair();
    let ((rets_c, snap_c), oc) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, data_raw, Some(b"State:1:Mode:3"), 128);
        let mut rets = Vec::new();
        for &op in ops {
            rets.push((p.c.confuse_types)(s, op));
        }
        let snap = p.c.snapshot(s, true);
        drop_state(s);
        (rets, snap)
    });
    let ((rets_r, snap_r), or) = capture(|| unsafe {
        let s = make_state(0x0000_7b05, data_raw, Some(b"State:1:Mode:3"), 128);
        let mut rets = Vec::new();
        for &op in ops {
            rets.push((p.rs.confuse_types)(s, op));
        }
        let snap = p.rs.snapshot(s, true);
        drop_state(s);
        (rets, snap)
    });
    assert_ret_eq(ctx, rets_c, rets_r);
    assert_snap_eq(ctx, &snap_c, &snap_r);
    assert_out_eq(ctx, &oc, &or);
}

/// `operation` is a plain `int` in C: every value with no matching `case` is a
/// real input that must return 0 and print nothing.
#[test]
fn row20_confuse_types_operation_out_of_range_positive() {
    for op in [4, 5, 6, 7, 8, 16, 100, 255, 256, 65536, i32::MAX, i32::MAX - 1] {
        compare_confuse_raw(&format!("row20 op={op}"), 0x4048_F5DB, &[op]);
        let p = pair();
        let (rc, oc) = capture(|| unsafe {
            let s = make_state(0x0000_7b05, 0x4048_F5DB, Some(b"x"), 8);
            let r = (p.c.confuse_types)(s, op);
            drop_state(s);
            r
        });
        assert_eq!(rc, 0, "op={op} must return 0");
        assert!(oc.is_empty(), "op={op} must print nothing");
    }
}

#[test]
fn row21_confuse_types_operation_negative() {
    for op in [-1, -2, -3, -4, -5, -100, i32::MIN, i32::MIN + 1] {
        compare_confuse_raw(&format!("row21 op={op}"), 0x4048_F5DB, &[op]);
        let p = pair();
        let (rc, oc) = capture(|| unsafe {
            let s = make_state(0x0000_7b05, 0x4048_F5DB, Some(b"x"), 8);
            let r = (p.c.confuse_types)(s, op);
            drop_state(s);
            r
        });
        assert_eq!(rc, 0, "op={op} must return 0");
        assert!(oc.is_empty(), "op={op} must print nothing");
    }
}

#[test]
fn row22_confuse_types_op1_float_specials() {
    // NaN (quiet/signalling, both signs), +-Inf, +-0, denormals, FLT_MAX,
    // and values whose *100 leaves int range -> cvttss2si "indefinite".
    const PATTERNS: [u32; 26] = [
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // smallest denormal
        0x007F_FFFF, // largest denormal
        0x0080_0000, // smallest normal
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x4048_F5DB, // 3.14159
        0x7F7F_FFFF, // FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
        0x7F80_0000, // +Inf
        0xFF80_0000, // -Inf
        0x7FC0_0000, // quiet NaN
        0xFFC0_0000, // -quiet NaN
        0x7F80_0001, // signalling NaN
        0xFF80_0001, // -signalling NaN
        0x4F00_0000, // 2^31 exactly -> *100 overflows
        0xCF00_0000, // -2^31
        0x4EFF_FFFF, // just below 2^31
        0x4B7F_FFFF, // 2^24-ish
        0x4CBE_BC1F, // ~1e8, *100 overflows int
        0x4C18_9680, // ~4e7
        0x477F_FF00, // 65535.0
        0x3C23_D70A, // 0.01 -> *100 == 1.0 (rounding sensitive)
        0x3D4C_CCCD, // 0.05
        0xBC23_D70A, // -0.01
    ];
    for pat in PATTERNS {
        compare_confuse_raw(&format!("row22 op1 data={pat:#010x}"), pat, &[1]);
        compare_confuse_raw(&format!("row22 op1x2 data={pat:#010x}"), pat, &[1, 1]);
    }
    // Randomized full-range bit patterns.
    let mut rng = Rng::new(0x2003);
    for _ in 0..1500 {
        let pat = rng.next_u64() as u32;
        compare_confuse_raw("row22 op1 random", pat, &[1]);
    }
}

#[test]
fn row23_confuse_types_op3_signed_bytes() {
    // char is signed on x86-64 Linux: bytes[0]+bytes[1] can be negative and
    // must sign-extend, not zero-extend.
    const PATTERNS: [u32; 12] = [
        0x0000_0000,
        0xFFFF_FFFF,
        0x0000_0080,
        0x0000_8080,
        0x8080_8080,
        0x7F7F_7F7F,
        0x0000_FF80,
        0x0000_80FF,
        0x0000_7F7F,
        0x0000_8180,
        0xFFFF_0000,
        0x4048_F5DB,
    ];
    for pat in PATTERNS {
        compare_confuse_raw(&format!("row23 op3 data={pat:#010x}"), pat, &[3]);
    }
    let mut rng = Rng::new(0x2004);
    for _ in 0..800 {
        let pat = rng.next_u64() as u32;
        compare_confuse_raw("row23 op3 random", pat, &[3]);
    }
}

/// Every `operation` value in a wide window around the valid range, on
/// randomized data — the cross-product of "out-of-range enum" with "arbitrary
/// payload".
#[test]
fn confuse_types_operation_window_randomized() {
    let mut rng = Rng::new(0x2005);
    for _ in 0..250 {
        let pat = rng.next_u64() as u32;
        let ops: Vec<c_int> = (-6..=10).collect();
        compare_confuse_raw("op window", pat, &ops);
    }
}

// ===========================================================================
// Rows 25, 26, 27 — `confusion` parameter reductions that select invalid modes
// ===========================================================================

fn compare_confusion(ctx: &str, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.confusion)(a, b, c, d) });
    let (rr, or) = capture(|| unsafe { (p.rs.confusion)(a, b, c, d) });
    assert_ret_eq(ctx, rc, rr);
    assert_out_eq(ctx, &oc, &or);
    rc
}

#[test]
fn row25_confusion_negative_param3_non_digit_search_char() {
    for p3 in [-1, -2, -3, -5, -9, -10, -11, -19, -20, -99, -1000] {
        for p4 in [0, 1, 2, 3] {
            compare_confusion(&format!("row25 p3={p3} p4={p4}"), 12345, 0, p3, p4);
        }
    }
    // The search character really is a non-digit, so nothing is ever found.
    let p = pair();
    let (_, out) = capture(|| unsafe { (p.c.confusion)(11111, 0, -1, 0) });
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("Operation: memchr_found"),
        "a non-digit search char must find nothing: {text}"
    );
}

#[test]
fn row26_confusion_param3_int_min() {
    // INT_MIN % 10 == -8  =>  search_char == '0' - 8 == '('
    compare_confusion("row26 p3=INT_MIN", 1, 0, i32::MIN, 0);
    for p4 in [0, 1, 2, 3, -1, 4] {
        compare_confusion(&format!("row26 p3=INT_MIN p4={p4}"), 88888, 7, i32::MIN, p4);
    }
    compare_confusion("row26 p3=INT_MIN+1", 1, 0, i32::MIN + 1, 0);
}

#[test]
fn row27_confusion_negative_param4_matches_no_switch_case() {
    // param4 % 4 in {-1,-2,-3} -> no `case` -> confusion_result == 0 and no
    // "Set as"/"Read as" line is printed at all.
    for p4 in [-1, -2, -3, -5, -6, -7, -9, -101, i32::MIN + 1] {
        let ctx = format!("row27 p4={p4}");
        compare_confusion(&ctx, 12345, 0, 5, p4);
        let p = pair();
        let (_, out) = capture(|| unsafe { (p.c.confusion)(12345, 0, 5, p4) });
        let text = String::from_utf8_lossy(&out);
        assert!(
            !text.contains("Set as") && !text.contains("Read as"),
            "p4={p4} (%4 = {}) must take no switch case: {text}",
            p4 % 4
        );
    }
    // INT_MIN % 4 == 0, so that one DOES take case 0.
    let p = pair();
    let (_, out) = capture(|| unsafe { (p.c.confusion)(12345, 0, 5, i32::MIN) });
    assert!(
        String::from_utf8_lossy(&out).contains("Set as int"),
        "INT_MIN % 4 == 0 selects case 0"
    );
    compare_confusion("row27 p4=INT_MIN", 12345, 0, 5, i32::MIN);
}

// ===========================================================================
// Rows 28, 29, 30 — extreme values, wrapping arithmetic, char narrowing
// ===========================================================================

#[test]
fn row28_confusion_param1_extremes() {
    for p1 in [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0,
        -1,
        1,
        1078530011,
        0x7F80_0000u32 as i32,
        0x7FC0_0000u32 as i32,
        0xFF80_0000u32 as i32,
        0x0000_0001,
        0x007F_FFFF,
    ] {
        for p4 in [0, 1, 2, 3] {
            compare_confusion(&format!("row28 p1={p1} p4={p4}"), p1, 0, 0, p4);
        }
    }
}

#[test]
fn row29_confusion_result_overflow_wraps() {
    // op 1 on a huge float yields INT_MIN, which then has counter*5 + mode*3
    // added to it: signed overflow that gcc wraps.
    for p2 in 0..64 {
        // p1 = +Inf bit pattern -> (int)(inf*100) == INT_MIN
        compare_confusion(
            &format!("row29 p2={p2}"),
            0x7F80_0000u32 as i32,
            p2,
            0,
            1,
        );
    }
    let mut rng = Rng::new(0x2006);
    for _ in 0..600 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        compare_confusion("row29 random overflow", p1, p2, 0, 1);
    }
}

#[test]
fn row30_confusion_search_char_narrowing() {
    // '0' + (param3 % 10) stays in range for |param3 % 10| <= 9, but the
    // narrowing conversion to `char` is exercised across the whole range.
    let mut rng = Rng::new(0x2007);
    for _ in 0..800 {
        let p3 = rng.interesting_i32();
        compare_confusion("row30 narrowing", 1234567, 0, p3, 0);
    }
    for p3 in -30..=30 {
        compare_confusion(&format!("row30 p3={p3}"), 1234567890, 0, p3, 3);
    }
}

// ===========================================================================
// Generic C-API boundaries (required even where not an ERRORS.md row)
// ===========================================================================

#[test]
fn generic_all_entry_points_with_null_pointers() {
    let p = pair();
    let n = std::ptr::null_mut::<c_void>();
    let (a, oa) = capture(|| unsafe { (p.c.process_buffer)(n, 0) });
    let (b, ob) = capture(|| unsafe { (p.rs.process_buffer)(n, 0) });
    assert_ret_eq("null process_buffer", a, b);
    assert_out_eq("null process_buffer", &oa, &ob);

    let (c, oc) = capture(|| unsafe { (p.c.confuse_types)(n, 0) });
    let (d, od) = capture(|| unsafe { (p.rs.confuse_types)(n, 0) });
    assert_ret_eq("null confuse_types", c, d);
    assert_out_eq("null confuse_types", &oc, &od);

    let ((), oe) = capture(|| unsafe { (p.c.update_flags)(n, 0) });
    let ((), of) = capture(|| unsafe { (p.rs.update_flags)(n, 0) });
    assert_out_eq("null update_flags", &oe, &of);

    let ((), og) = capture(|| unsafe { (p.c.destroy_state)(n) });
    let ((), oh) = capture(|| unsafe { (p.rs.destroy_state)(n) });
    assert_out_eq("null destroy_state", &og, &oh);
}

#[test]
fn generic_one_past_valid_ranges() {
    // capacity: 0 (zero length), -1 (one below), INT_MIN (extreme)
    compare_create("generic cap=0", 1, 0, false);
    compare_create("generic cap=-1", 1, -1, true);
    compare_create("generic cap=1", 1, 1, true);

    // operation: 3 (last valid), 4 (one past), -1 (one below)
    for op in [3, 4, -1] {
        compare_confuse_raw(&format!("generic op={op}"), 0x4048_F5DB, &[op]);
    }

    // mode field is 3 bits: param>>3 == 7 is the last in-range value, 8 wraps
    for param in [0x38, 0x3F, 0x40, 0x47, 0x48] {
        compare_update_raw(&format!("generic param={param:#x}"), 0x0000_7b05, 0, &[param]);
    }

    // target: full char range boundaries
    for t in [0u8, 1, 0x7F, 0x80, 0xFF] {
        compare_process_raw(&format!("generic target={t:#04x}"), b"State:1:Mode:3", t);
    }
}

/// Negative control: the harness must actually be able to see a difference.
/// If this test ever passed by comparing nothing, the whole suite is vacuous.
#[test]
fn harness_detects_divergence() {
    let p = pair();
    let (r1, o1) = capture(|| unsafe { (p.c.confusion)(1, 2, 3, 4) });
    let (r2, o2) = capture(|| unsafe { (p.c.confusion)(9, 8, 7, 6) });
    assert_ne!(o1, o2, "capture must observe differing stdout");
    assert!(!o1.is_empty() && !o2.is_empty());
    assert!(
        std::panic::catch_unwind(|| assert_out_eq("neg control", &o1, &o2)).is_err(),
        "assert_out_eq must reject differing output"
    );
    assert!(
        std::panic::catch_unwind(|| assert_ret_eq("neg control", r1, r2 + 1)).is_err(),
        "assert_ret_eq must reject differing returns"
    );
}
