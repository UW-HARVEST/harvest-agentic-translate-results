// Meta-tests: prove the harness itself is not trivially passing.
//
// A differential test that compares two empty buffers, or that silently loads
// the same library twice, passes vacuously. These tests fail if either happens.
mod common;

use common::*;
use std::ffi::c_char;

#[test]
fn harness_loads_two_distinct_libraries() {
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
    // Distinct .so files => distinct function addresses for every symbol.
    assert_ne!(
        p.c.modeselect as usize, p.rs.modeselect as usize,
        "C and Rust modeselect resolved to the same address: the same library was loaded twice"
    );
    assert_ne!(p.c.classify_mode as usize, p.rs.classify_mode as usize);
    assert_ne!(p.c.apply_multiplier as usize, p.rs.apply_multiplier as usize);
    assert_ne!(p.c.convert_time_factor as usize, p.rs.convert_time_factor as usize);
    assert_ne!(
        p.c.convert_negative_overflow as usize,
        p.rs.convert_negative_overflow as usize
    );
    assert_ne!(p.c.get_modified_time as usize, p.rs.get_modified_time as usize);
    assert_ne!(p.c.hash_time_value as usize, p.rs.hash_time_value as usize);
}

#[test]
fn harness_stdout_capture_is_not_vacuous() {
    let p = pair();
    // SAFETY: mode_selector 0 is in range; plain scalar C ABI call.
    let (rc, oc) = capture_forked_i32(|| unsafe { (p.c.modeselect)(2, 7, 3, 11) });
    let (rr, or) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(2, 7, 3, 11) });

    assert!(!oc.is_empty(), "captured nothing from the C library");
    assert!(!or.is_empty(), "captured nothing from the Rust library");

    // All 8 printf call sites must be present, so a partial capture cannot pass.
    let s = String::from_utf8_lossy(&oc).to_string();
    for needle in [
        "Selected mode: turbo (0x30)",
        "Complexity level: 3, Multiplier: 0x",
        "Modified time: ",
        ", Hash: 0x",
        "Converting double 1.10e+09 to int (may overflow)...",
        "Result 1: ",
        "Converting double -7.00e+07 to int (may underflow)...",
        "Result 2: ",
        "Final result: ",
    ] {
        assert!(s.contains(needle), "C stdout missing {needle:?}; got:\n{s}");
    }
    assert_eq!(s.lines().count(), 9, "expected 8 printf calls (9 lines incl. the blank one)");

    eq_bytes("meta", "modeselect(2,7,3,11)", &oc, &or);
    eq_int("meta", "modeselect(2,7,3,11)", rc, rr);
}

#[test]
fn harness_detects_a_deliberate_difference() {
    // Sanity: the comparison helpers really do fail on divergence.
    let a = b"abc";
    let b = b"abd";
    let r = std::panic::catch_unwind(|| eq_bytes("meta", "x", a, b));
    assert!(r.is_err(), "eq_bytes failed to detect differing bytes");
    let r = std::panic::catch_unwind(|| eq_int("meta", "x", 1, 2));
    assert!(r.is_err(), "eq_int failed to detect differing ints");
}

#[test]
fn harness_c_library_is_the_expected_ground_truth() {
    // Pin a handful of C return values literally, so a broken dlopen that
    // returned zeros everywhere could not pass the differential suite.
    let p = pair();
    let std_ = cstr(b"standard");
    // SAFETY: NUL-terminated buffer; plain scalar C ABI calls.
    unsafe {
        assert_eq!((p.c.classify_mode)(std_.as_ptr() as *const c_char), 0x10);
        assert_eq!((p.c.apply_multiplier)(0xA0, 4), 0xA0 + 0xFF + 0xAB + 0x7E + 0x1C + 0x05);
        assert_eq!((p.c.apply_multiplier)(0xA0, 0), 0xA0 + 0x05);
        assert_eq!((p.c.apply_multiplier)(0, 99), 0xDEAD);
        assert_eq!((p.c.convert_time_factor)(1.0), i32::MIN);
        assert_eq!((p.c.convert_time_factor)(0.0), 0);
        assert_eq!((p.c.convert_negative_overflow)(1.0), i32::MIN);
        assert_eq!((p.c.hash_time_value)(0) & !0x7FFF_FFFF, 0);
    }
}
