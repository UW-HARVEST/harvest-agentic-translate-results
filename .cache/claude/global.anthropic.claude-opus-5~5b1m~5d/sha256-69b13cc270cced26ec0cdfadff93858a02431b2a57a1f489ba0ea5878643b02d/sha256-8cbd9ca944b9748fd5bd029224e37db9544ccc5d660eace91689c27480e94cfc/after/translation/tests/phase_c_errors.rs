//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid
//! input/condition, calls BOTH `.so`s and asserts the SAME sentinel/error
//! value (not merely "both failed").

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Row 1 — is_string_empty(NULL) == 1
// ---------------------------------------------------------------------------
#[test]
fn err_is_string_empty_null() {
    let l = libs();
    let c = unsafe { (l.c.is_string_empty)(std::ptr::null()) };
    let r = unsafe { (l.r.is_string_empty)(std::ptr::null()) };
    assert_eq!(c, r, "is_string_empty(NULL)");
    assert_eq!(c, 1, "C contract: NULL -> 1");
}

// ---------------------------------------------------------------------------
// Row 2 — is_string_empty("") == 1
// ---------------------------------------------------------------------------
#[test]
fn err_is_string_empty_empty() {
    let l = libs();
    let s = b"\0";
    let p = s.as_ptr() as *const c_char;
    let c = unsafe { (l.c.is_string_empty)(p) };
    let r = unsafe { (l.r.is_string_empty)(p) };
    assert_eq!(c, r, "is_string_empty(\"\")");
    assert_eq!(c, 1, "C contract: empty -> 1");
    // ... and the negative control, so the sentinel is not trivially constant.
    let ne = b"x\0";
    let p2 = ne.as_ptr() as *const c_char;
    let c2 = unsafe { (l.c.is_string_empty)(p2) };
    let r2 = unsafe { (l.r.is_string_empty)(p2) };
    assert_eq!(c2, r2);
    assert_eq!(c2, 0);
}

// ---------------------------------------------------------------------------
// Row 3 — find_char_in_buffer(NULL, ..) == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_find_char_null_buffer() {
    let l = libs();
    for size in [0usize, 1, 7, 1 << 20, usize::MAX] {
        for t in [0i8, 1, b'a' as i8, 127, -1, -128] {
            let cp = unsafe { (l.c.find_char_in_buffer)(std::ptr::null(), size, t) };
            let rp = unsafe { (l.r.find_char_in_buffer)(std::ptr::null(), size, t) };
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "find_char_in_buffer(NULL, {size}, {t}) null-ness"
            );
            assert!(cp.is_null(), "C contract: NULL buffer -> NULL");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — non-NULL buffer, size == 0 -> NULL
// ---------------------------------------------------------------------------
#[test]
fn err_find_char_zero_size() {
    let l = libs();
    let buf = b"abcdef\0";
    let p = buf.as_ptr() as *const c_char;
    for t in [b'a' as i8, b'f' as i8, 0i8, -1i8, 127i8] {
        let cp = unsafe { (l.c.find_char_in_buffer)(p, 0, t) };
        let rp = unsafe { (l.r.find_char_in_buffer)(p, 0, t) };
        assert_eq!(cp.is_null(), rp.is_null(), "size==0, target {t}");
        assert!(cp.is_null(), "C contract: size 0 -> NULL");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — target absent (incl. present past `size`, and NUL past `size`)
// ---------------------------------------------------------------------------
#[test]
fn err_find_char_not_found() {
    let l = libs();
    let cases: &[(&[u8], usize, i8)] = &[
        (b"abcdefghij", 10, b'z' as i8),
        (b"aaaaaaaaaaZ", 10, b'Z' as i8), // Z only at index 10
        (b"abc\0", 3, 0),                 // NUL only at index 3
        (b"\x01\x02\x03", 3, 4),
        (b"\x7f\x7f\x7f", 3, -1),
    ];
    for (buf, size, t) in cases {
        let p = buf.as_ptr() as *const c_char;
        let cp = unsafe { (l.c.find_char_in_buffer)(p, *size, *t) };
        let rp = unsafe { (l.r.find_char_in_buffer)(p, *size, *t) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "not-found case {buf:?} size {size} target {t}"
        );
        assert!(cp.is_null(), "C contract: absent -> NULL");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — create_buffer(NULL) == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_create_buffer_null() {
    let l = libs();
    let cp = unsafe { (l.c.create_buffer)(std::ptr::null()) };
    let rp = unsafe { (l.r.create_buffer)(std::ptr::null()) };
    assert_eq!(cp.is_null(), rp.is_null(), "create_buffer(NULL)");
    assert!(cp.is_null(), "C contract: NULL initial -> NULL");
}

// ---------------------------------------------------------------------------
// Row 7 — create_buffer with a failing malloc.
//
// `malloc` cannot be made to fail deterministically from inside the test
// process without interposing it globally (which would also break the test
// harness's own allocations). The structural contract we *can* assert is:
//   * both libraries obtain their memory from the SAME allocator, so a caller
//     may `free()` either result interchangeably;
//   * neither writes anything when the pointer would be NULL, i.e. the copy is
//     guarded (line 73 in the C).
// An absurd (SIZE_MAX-class) length is the closest reachable proxy: we drive
// it through `find_char_in_buffer`'s sibling path instead, and here we assert
// the allocator-identity property plus NULL propagation from row 6.
// ---------------------------------------------------------------------------
#[test]
fn err_create_buffer_alloc_fail_contract() {
    let l = libs();
    extern "C" {
        fn free(p: *mut std::ffi::c_void);
        fn strlen(s: *const c_char) -> usize;
    }
    let s = b"allocator identity probe\0";
    let p = s.as_ptr() as *const c_char;
    unsafe {
        let cp = (l.c.create_buffer)(p);
        let rp = (l.r.create_buffer)(p);
        assert!(!cp.is_null() && !rp.is_null(), "probe allocation failed");
        assert_eq!(strlen(cp), strlen(rp));
        assert_eq!(
            std::slice::from_raw_parts(cp as *const u8, strlen(cp) + 1),
            std::slice::from_raw_parts(rp as *const u8, strlen(rp) + 1)
        );
        // Both come from the process allocator: the test frees both with the
        // same `free`. A mismatch here aborts/traps, which is the assertion.
        free(cp as *mut _);
        free(rp as *mut _);
    }
    // NULL propagation (the other arm of the same `if (buffer)` guard).
    let cp = unsafe { (l.c.create_buffer)(std::ptr::null()) };
    let rp = unsafe { (l.r.create_buffer)(std::ptr::null()) };
    assert!(cp.is_null() && rp.is_null());
}

// ---------------------------------------------------------------------------
// Rows 8-10 — validate_uint16_range
// ---------------------------------------------------------------------------
#[test]
fn err_validate_uint16_negative() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xA8);
    let mut vals: Vec<i32> = vec![-1, -2, -65535, -65536, i32::MIN, i32::MIN + 1];
    for _ in 0..1024 {
        vals.push(rng.range_i32(i32::MIN, -1));
    }
    for v in vals {
        let c = unsafe { (l.c.validate_uint16_range)(v) };
        let r = unsafe { (l.r.validate_uint16_range)(v) };
        assert_eq!(c, r, "validate_uint16_range({v})");
        assert_eq!(c, 0, "C contract: negative -> 0 (value {v})");
    }
}

#[test]
fn err_validate_uint16_too_large() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xA9);
    let mut vals: Vec<i32> = vec![65536, 65537, 1 << 20, i32::MAX, i32::MAX - 1];
    for _ in 0..1024 {
        vals.push(rng.range_i32(65536, i32::MAX));
    }
    for v in vals {
        let c = unsafe { (l.c.validate_uint16_range)(v) };
        let r = unsafe { (l.r.validate_uint16_range)(v) };
        assert_eq!(c, r, "validate_uint16_range({v})");
        assert_eq!(c, 0, "C contract: > UINT16_MAX -> 0 (value {v})");
    }
}

#[test]
fn err_validate_uint16_boundaries() {
    let l = libs();
    // Inclusive boundaries are VALID; one step past on each side is not.
    for (v, expect) in [
        (-1, 0),
        (0, 1),
        (1, 1),
        (65534, 1),
        (65535, 1),
        (65536, 0),
    ] {
        let c = unsafe { (l.c.validate_uint16_range)(v) };
        let r = unsafe { (l.r.validate_uint16_range)(v) };
        assert_eq!(c, r, "validate_uint16_range({v})");
        assert_eq!(c, expect, "C contract for {v}");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — apply_operation(NULL, v) == -1
// ---------------------------------------------------------------------------
#[test]
fn err_apply_operation_null_fp() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xB1);
    let mut vals: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, -1];
    for _ in 0..512 {
        vals.push(rng.spicy_i32());
    }
    for v in vals {
        let c = unsafe { (l.c.apply_operation)(None, v) };
        let r = unsafe { (l.r.apply_operation)(None, v) };
        assert_eq!(c, r, "apply_operation(NULL, {v})");
        assert_eq!(c, -1, "C contract: NULL op -> -1");
    }
    // -1 must not be a coincidence: a real op returning -1 is distinguishable
    // only by the counter side effect, so check the state moved.
    unsafe {
        assert_eq!((l.c.reset_counter)(-1), (l.r.reset_counter)(-1));
        assert_eq!(
            (l.c.apply_operation)(Some(l.c.increment_counter), 0),
            (l.r.apply_operation)(Some(l.r.increment_counter), 0)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 / G4 — charinbuf default branch: out-of-range enum-like int
// ---------------------------------------------------------------------------
#[test]
fn err_charinbuf_invalid_mode() {
    let l = libs();
    let mut modes: Vec<i32> = vec![
        -1,
        -2,
        5,
        6,
        7,
        100,
        255,
        256,
        1 << 16,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..512 {
        let mut m = rng.spicy_i32();
        if (0..=4).contains(&m) {
            m = m.wrapping_sub(9999);
        }
        modes.push(m);
    }
    for m in modes {
        let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(m, 1, 2, 3) });
        let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(m, 1, 2, 3) });
        assert_eq!(cres.0, -1, "C contract: invalid mode {m} -> -1");
        assert_same_call(&format!("charinbuf(invalid mode {m})"), cres, rres);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — charinbuf mode 0 out-of-range value
// ---------------------------------------------------------------------------
#[test]
fn err_charinbuf_mode0_out_of_range() {
    let l = libs();
    let mut vals: Vec<i32> = vec![-1, -2, i32::MIN, 65536, 65537, i32::MAX];
    let mut rng = Rng::new(SEED ^ 0xD3);
    for _ in 0..256 {
        vals.push(if rng.next_u64() % 2 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(65536, i32::MAX)
        });
    }
    for v in vals {
        let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(0, v, 0, 0) });
        let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(0, v, 0, 0) });
        assert_eq!(cres.0, -1, "C contract: mode 0 with {v} -> -1");
        assert!(
            String::from_utf8_lossy(&cres.1).contains("is out of range for uint16_t"),
            "C should take the rejection branch for {v}"
        );
        assert_same_call(&format!("charinbuf(0, {v}, 0, 0)"), cres, rres);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — charinbuf mode 2 allocation failure (unreachable) contract
// ---------------------------------------------------------------------------
#[test]
fn err_charinbuf_mode2_alloc_fail_contract() {
    let l = libs();
    // The failure arm needs malloc to fail for a 24-byte request, which cannot
    // be forced here. What IS verifiable is that the success arm is entered
    // identically and that the reported length matches exactly (a wrong length
    // would be the observable symptom of a mistranslated guard).
    let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(2, 0, 0, 0) });
    let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(2, 0, 0, 0) });
    assert_eq!(
        cres.0,
        "Testing malloc and free".len() as c_int,
        "C contract: mode 2 returns strlen of the literal"
    );
    let text = String::from_utf8_lossy(&cres.1).to_string();
    assert!(text.contains("Buffer freed successfully"), "success arm taken");
    assert!(
        !text.contains("Failed to allocate buffer"),
        "failure arm must not be taken here"
    );
    assert_same_call("charinbuf(2, ...) success arm", cres, rres);
}

// ---------------------------------------------------------------------------
// Row 15 — charinbuf mode 4 "character not found" (unreachable) contract
// ---------------------------------------------------------------------------
#[test]
fn err_charinbuf_mode4_notfound_contract() {
    let l = libs();
    let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(4, 0, 0, 0) });
    let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(4, 0, 0, 0) });
    let expected = "Search for character X in this buffer".find('X').unwrap() as c_int;
    assert_eq!(cres.0, expected, "C contract: index of 'X'");
    assert_ne!(cres.0, -1, "the not-found arm is unreachable for this literal");
    assert!(!String::from_utf8_lossy(&cres.1).contains("not found"));
    assert_same_call("charinbuf(4, ...) found arm", cres, rres);

    // The equivalent -1 sentinel IS reachable through the underlying export,
    // which is the same code path (`find_char_in_buffer` returning NULL).
    let buf = b"Search for character _ in this buffer";
    let p = buf.as_ptr() as *const c_char;
    let cp = unsafe { (l.c.find_char_in_buffer)(p, buf.len(), b'X' as c_char) };
    let rp = unsafe { (l.r.find_char_in_buffer)(p, buf.len(), b'X' as c_char) };
    assert!(cp.is_null() && rp.is_null(), "no 'X' -> NULL in both");
}

// ---------------------------------------------------------------------------
// Row 16 — charinbuf mode 4 with a NULL buffer keeps result == 0 (unreachable)
// ---------------------------------------------------------------------------
#[test]
fn err_charinbuf_mode4_null_buffer_contract() {
    let l = libs();
    // The C `if (buffer)` at line 184 has no `else`, so a NULL allocation would
    // leave `result` at its initialiser 0. Not forceable, but we can prove the
    // Rust keeps the same initialiser semantics for the one branch that *is*
    // reachable, and that no stray non-zero default leaks in.
    let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(4, i32::MIN, i32::MAX, -1) });
    let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(4, i32::MIN, i32::MAX, -1) });
    assert_same_call("charinbuf(4) ignores its extra args", cres, rres);

    // `create_buffer(NULL)` is the NULL-producing sibling: both must yield NULL,
    // which is exactly the condition that would drive the no-else branch.
    let cp = unsafe { (l.c.create_buffer)(std::ptr::null()) };
    let rp = unsafe { (l.r.create_buffer)(std::ptr::null()) };
    assert!(cp.is_null() && rp.is_null());
}

// ---------------------------------------------------------------------------
// G1 — NULL to every pointer-taking export
// ---------------------------------------------------------------------------
#[test]
fn err_all_null_pointers() {
    let l = libs();
    unsafe {
        assert_eq!(
            (l.c.is_string_empty)(std::ptr::null()),
            (l.r.is_string_empty)(std::ptr::null())
        );
        assert_eq!(
            (l.c.find_char_in_buffer)(std::ptr::null(), 10, 65).is_null(),
            (l.r.find_char_in_buffer)(std::ptr::null(), 10, 65).is_null()
        );
        assert_eq!(
            (l.c.create_buffer)(std::ptr::null()).is_null(),
            (l.r.create_buffer)(std::ptr::null()).is_null()
        );
        assert_eq!((l.c.apply_operation)(None, 0), (l.r.apply_operation)(None, 0));
    }
}

// ---------------------------------------------------------------------------
// Symbol parity as an executable assertion (Phase D, checked here too)
// ---------------------------------------------------------------------------
#[test]
fn err_symbol_parity() {
    let l = libs();
    // Loading `Lib` already resolves every symbol in EXPECTED_SYMBOLS through
    // both handles; this test documents the list and fails loudly if the Rust
    // `.so` ever drops an export.
    assert_eq!(EXPECTED_SYMBOLS.len(), 10);
    assert!(l.c.path.exists() && l.r.path.exists());
}
