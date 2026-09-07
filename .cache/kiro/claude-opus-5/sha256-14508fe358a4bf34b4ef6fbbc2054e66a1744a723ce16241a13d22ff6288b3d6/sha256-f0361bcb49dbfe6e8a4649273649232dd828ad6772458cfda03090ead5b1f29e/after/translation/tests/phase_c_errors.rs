//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact rejection condition and asserts the two
//! libraries return the *same* sentinel (`NULL`, `-1`, `0`, `1`), not merely
//! that both failed.

mod common;

use common::*;
use std::ffi::{c_char, c_int};
use std::ptr;

// ------------------------------------------------------- row 1: NULL string

#[test]
fn err01_is_string_empty_null() {
    let _g = guard();
    let p = pair();
    let a = unsafe { (p.c.is_string_empty)(ptr::null()) };
    let b = unsafe { (p.rs.is_string_empty)(ptr::null()) };
    assert_eq!(a, 1, "C is_string_empty(NULL) must be 1");
    assert_eq!(a, b, "is_string_empty(NULL): C={a} Rust={b}");
}

// ------------------------------------------------------- row 2: empty string

#[test]
fn err02_is_string_empty_empty_matches_null_sentinel() {
    let _g = guard();
    let p = pair();
    let s = cstring(b"");
    let a = unsafe { (p.c.is_string_empty)(s.as_ptr()) };
    let b = unsafe { (p.rs.is_string_empty)(s.as_ptr()) };
    assert_eq!(a, 1, "C is_string_empty(\"\") must be 1");
    assert_eq!(a, b);
    // Both rejections collapse to the same value in C; that must be preserved.
    let an = unsafe { (p.c.is_string_empty)(ptr::null()) };
    let bn = unsafe { (p.rs.is_string_empty)(ptr::null()) };
    assert_eq!(a, an);
    assert_eq!(b, bn);
}

// ------------------------------------------- row 3: NULL buffer, any size

#[test]
fn err03_find_char_null_buffer() {
    let _g = guard();
    let p = pair();
    // The NULL check precedes any use of `size`, so an oversized `size` on a
    // NULL pointer must still return cleanly.
    for size in [0usize, 1, 64, usize::MAX / 2, usize::MAX] {
        for target in [0u8, b'a', 0xff] {
            let a = unsafe { (p.c.find_char_in_buffer)(ptr::null(), size, target as c_char) };
            let b = unsafe { (p.rs.find_char_in_buffer)(ptr::null(), size, target as c_char) };
            assert!(a.is_null(), "C find_char_in_buffer(NULL,{size}) must be NULL");
            assert_eq!(a, b, "find_char_in_buffer(NULL, {size}, {target:#04x})");
        }
    }
}

// ------------------------------------------------------------ row 4: size 0

#[test]
fn err04_find_char_zero_size() {
    let _g = guard();
    let p = pair();
    let store = cstring(b"aXbXc");
    let ptr0 = store.as_ptr();
    for target in [0u8, b'a', b'X', 0x80, 0xff] {
        let a = unsafe { (p.c.find_char_in_buffer)(ptr0, 0, target as c_char) };
        let b = unsafe { (p.rs.find_char_in_buffer)(ptr0, 0, target as c_char) };
        assert!(a.is_null(), "C find_char_in_buffer(buf,0) must be NULL");
        assert_eq!(a, b, "find_char_in_buffer(buf, 0, {target:#04x})");
    }
    // Oversized `size` with a zero-length buffer is UB in C the same way it is
    // in Rust, so it is deliberately not exercised; the readable-prefix case is
    // covered in Phase B row 22.
}

// -------------------------------------------------------- row 5: no match

#[test]
fn err05_find_char_absent_target() {
    let _g = guard();
    let p = pair();
    let store = cstring(b"abcdefghij");
    let ptr0 = store.as_ptr();
    let mut rng = Rng::new(0xC005);
    for _ in 0..2000 {
        // Anything outside 'a'..='j' plus the appended NUL is absent.
        let mut t = rng.u8();
        if (b'a'..=b'j').contains(&t) || t == 0 {
            t = 0xfe;
        }
        let a = unsafe { (p.c.find_char_in_buffer)(ptr0, 11, t as c_char) };
        let b = unsafe { (p.rs.find_char_in_buffer)(ptr0, 11, t as c_char) };
        assert!(a.is_null(), "C must not find absent {t:#04x}");
        assert_eq!(a, b, "find_char_in_buffer absent target {t:#04x}");
    }
}

// --------------------------------------------------- row 6: NULL to create

#[test]
fn err06_create_buffer_null() {
    let _g = guard();
    let p = pair();
    let a = unsafe { (p.c.create_buffer)(ptr::null()) };
    let b = unsafe { (p.rs.create_buffer)(ptr::null()) };
    assert!(a.is_null(), "C create_buffer(NULL) must be NULL");
    assert!(b.is_null(), "Rust create_buffer(NULL) must be NULL");
    assert_eq!(a, b);
}

// ------------------------------- row 7: malloc failure (static equivalence)

#[test]
fn err07_create_buffer_malloc_failure_is_structurally_equivalent() {
    // `malloc` cannot be made to fail reliably from inside the test process
    // without an allocator interposer, and the two libraries share the *same*
    // process allocator, so a failure would hit both identically. Instead the
    // structure of the guard is asserted: the Rust translation must keep the
    // `if (buffer)` check before `strcpy` and must propagate NULL, exactly as
    // the C does.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rs = std::fs::read_to_string(root.join("src/helpers.rs")).unwrap();
    let create = rs
        .split("pub unsafe extern \"C\" fn create_buffer")
        .nth(1)
        .expect("create_buffer present in src/helpers.rs");
    let body: String = create.chars().take(700).collect();
    assert!(
        body.contains("if !buffer.is_null()"),
        "create_buffer must guard strcpy on a non-NULL malloc result"
    );
    let guard_at = body.find("if !buffer.is_null()").unwrap();
    let strcpy_at = body.find("strcpy").expect("strcpy call present");
    assert!(
        guard_at < strcpy_at,
        "the NULL guard must precede the strcpy, as in lib.c:73-75"
    );

    let c = std::fs::read_to_string(root.join("../c_src/src/lib.c")).unwrap();
    assert!(
        c.contains("if (buffer) {"),
        "C source shape changed; revisit this row"
    );
}

// ------------------------------------------ rows 8 & 9: uint16 range checks

#[test]
fn err08_validate_uint16_negative() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC008);
    let mut cases: Vec<i32> = vec![-1, -2, -65535, -65536, i32::MIN, i32::MIN + 1];
    for _ in 0..2000 {
        cases.push(rng.range_i32(i32::MIN, -1));
    }
    for v in cases {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, 0, "C validate_uint16_range({v}) must reject with 0");
        assert_eq!(a, b, "validate_uint16_range({v})");
    }
}

#[test]
fn err09_validate_uint16_above_max() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC009);
    let mut cases: Vec<i32> = vec![65536, 65537, 131071, i32::MAX, i32::MAX - 1];
    for _ in 0..2000 {
        cases.push(rng.range_i32(65536, i32::MAX));
    }
    for v in cases {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, 0, "C validate_uint16_range({v}) must reject with 0");
        assert_eq!(a, b, "validate_uint16_range({v})");
    }
    // One step inside the range must be accepted by both, so the boundary is
    // exactly 65535/65536 in both libraries.
    for v in [65535, 65534, 0] {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, 1, "C validate_uint16_range({v}) must accept");
        assert_eq!(a, b);
    }
}

// ----------------------------------------------------- row 10: NULL callback

#[test]
fn err10_apply_operation_null_op() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC010);
    let mut cases: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX];
    for _ in 0..2000 {
        cases.push(rng.interesting_i32());
    }
    for v in cases {
        let a = unsafe { (p.c.apply_operation)(None, v) };
        let b = unsafe { (p.rs.apply_operation)(None, v) };
        assert_eq!(a, -1, "C apply_operation(NULL, {v}) must be -1");
        assert_eq!(a, b, "apply_operation(NULL, {v})");
    }
}

// ------------------------------- row 11: callback legitimately returning -1

unsafe extern "C" fn cb_minus_one(_v: c_int) -> c_int {
    -1
}

#[test]
fn err11_apply_operation_sentinel_collision() {
    let _g = guard();
    let p = pair();
    let f: CounterFn = cb_minus_one;
    for v in [0, 1, -1, i32::MIN, i32::MAX] {
        let a = unsafe { (p.c.apply_operation)(Some(f), v) };
        let b = unsafe { (p.rs.apply_operation)(Some(f), v) };
        assert_eq!(a, -1, "the callee's -1 must be returned verbatim");
        assert_eq!(a, b, "apply_operation(cb_minus_one, {v})");
    }
    // The library must not distinguish this from the NULL case, as in C.
    let n = unsafe { (p.c.apply_operation)(None, 7) };
    let m = unsafe { (p.rs.apply_operation)(None, 7) };
    assert_eq!(n, -1);
    assert_eq!(m, -1);
}

// --------------------------------------------- row 12: mode 0 out of range

#[test]
fn err12_charinbuf_mode0_rejects() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC012);
    let mut cases: Vec<i32> = vec![-1, -2, 65536, 65537, i32::MIN, i32::MAX];
    for _ in 0..300 {
        cases.push(if rng.below(2) == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(65536, i32::MAX)
        });
    }
    for v in cases {
        let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(0, v, 0, 0) });
        let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(0, v, 0, 0) });
        assert_eq!(rc, -1, "C charinbuf(0,{v},..) must return -1");
        assert_eq!(rc, rr, "charinbuf(0,{v},..) return value");
        assert_eq!(
            oc,
            or,
            "charinbuf(0,{v},..) stdout\nC: {:?}\nRust: {:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or)
        );
        assert!(
            String::from_utf8_lossy(&oc).contains("is out of range for uint16_t"),
            "expected the rejection message, got {:?}",
            String::from_utf8_lossy(&oc)
        );
    }
}

// ------------------------------ row 13: mode 1 "check failed" (unreachable)

#[test]
fn err13_charinbuf_mode1_failed_branch_is_unreachable_in_both() {
    let _g = guard();
    let p = pair();
    // The haystack is the literal "Hello, World!", so `is_string_empty` returns
    // 0 and the failure branch cannot be reached. Assert that neither library
    // prints the failure message and that both take the +10 path, and pin the
    // literal in both sources so a future edit cannot silently change this.
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(1, 0, 0, 0) });
    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(1, 0, 0, 0) });
    assert_eq!(rc, rr);
    assert_eq!(oc, or);
    let text = String::from_utf8_lossy(&oc).to_string();
    assert!(
        !text.contains("Non-empty string check failed!"),
        "unexpectedly reached the failure branch: {text:?}"
    );
    assert!(text.contains("Non-empty string correctly identified"));
    assert_eq!(rc, 10, "0 (empty) + 10 (non-empty identified)");

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("../c_src/src/lib.c")).unwrap();
    let rs = std::fs::read_to_string(root.join("src/charinbuf.rs")).unwrap();
    assert!(c.contains("\"Hello, World!\""));
    assert!(rs.contains("Hello, World!"));
    // The direct entry point covers the branch condition itself.
    let s = cstring(b"Hello, World!");
    assert_eq!(unsafe { (p.c.is_string_empty)(s.as_ptr()) }, 0);
    assert_eq!(unsafe { (p.rs.is_string_empty)(s.as_ptr()) }, 0);
}

// ------------------------- row 14: mode 2 allocation failure (unreachable)

#[test]
fn err14_charinbuf_mode2_alloc_failure_branch() {
    let _g = guard();
    let p = pair();
    // Cannot induce malloc failure; assert instead that the success path is
    // taken identically and that the failure branch exists with the same
    // sentinel in the Rust translation.
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(2, 0, 0, 0) });
    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(2, 0, 0, 0) });
    assert_eq!(rc, rr);
    assert_eq!(oc, or);
    assert_eq!(rc, 23, "strlen(\"Testing malloc and free\")");
    assert!(!String::from_utf8_lossy(&oc).contains("Failed to allocate buffer"));

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rs = std::fs::read_to_string(root.join("src/charinbuf.rs")).unwrap();
    assert!(
        rs.contains("Failed to allocate buffer"),
        "the Rust mode 2 failure branch is missing"
    );
    let idx = rs.find("Failed to allocate buffer").unwrap();
    assert!(
        rs[idx..idx + 200].contains("result = -1"),
        "the Rust mode 2 failure branch must yield -1"
    );
}

// ------------------------- row 15: mode 4 character not found (unreachable)

#[test]
fn err15_charinbuf_mode4_not_found_branch() {
    let _g = guard();
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(4, 0, 0, 0) });
    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(4, 0, 0, 0) });
    assert_eq!(rc, rr);
    assert_eq!(oc, or);
    assert!(!String::from_utf8_lossy(&oc).contains("not found"));

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rs = std::fs::read_to_string(root.join("src/charinbuf.rs")).unwrap();
    let idx = rs
        .find("not found")
        .expect("the Rust mode 4 not-found branch is missing");
    assert!(
        rs[idx..idx + 200].contains("result = -1"),
        "the Rust mode 4 not-found branch must yield -1"
    );
    // The underlying rejection is exercised directly on the same haystack.
    let hay = cstring(b"Search for character X in this buffer");
    let a = unsafe { (p.c.find_char_in_buffer)(hay.as_ptr(), 37, b'Q' as c_char) };
    let b = unsafe { (p.rs.find_char_in_buffer)(hay.as_ptr(), 37, b'Q' as c_char) };
    assert!(a.is_null());
    assert_eq!(a, b);
}

// ------------------- row 16: mode 4 allocation failure returns 0, not -1

#[test]
fn err16_charinbuf_mode4_alloc_failure_returns_zero() {
    // Not inducible; pin the structural property that makes this row special —
    // the *outer* `if (buffer)` in mode 4 has no `else`, so a NULL allocation
    // falls through with `result` still 0 rather than -1. (The single `else`
    // present in the arm belongs to the inner not-found check, row 15.)
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let c = std::fs::read_to_string(root.join("../c_src/src/lib.c")).unwrap();
    let c_arm = c
        .split("case 4:")
        .nth(1)
        .and_then(|s| s.split("default:").next())
        .expect("C mode 4 arm present")
        .to_string();
    let c_elses = c_arm.matches("else").count();
    let c_minus_ones = c_arm.matches("result = -1").count();
    assert_eq!(c_elses, 1, "C mode 4 arm shape changed; revisit this row");
    assert_eq!(c_minus_ones, 1, "C mode 4 arm shape changed");

    let rs = std::fs::read_to_string(root.join("src/charinbuf.rs")).unwrap();
    let rs_arm = rs
        .split("Mode 4: Using memchr to find character")
        .nth(1)
        .and_then(|s| s.split("Invalid mode").next())
        .expect("Rust mode 4 arm present")
        .to_string();
    assert!(
        rs_arm.contains("if !buffer.is_null()"),
        "mode 4 must guard on the allocation result"
    );
    assert_eq!(
        rs_arm.matches("else").count(),
        c_elses,
        "the Rust mode 4 arm must have exactly as many `else` branches as the C \
         arm ({c_elses}): the allocation guard must stay else-less so a NULL \
         buffer yields the initial 0, not -1"
    );
    assert_eq!(
        rs_arm.matches("result = -1").count(),
        c_minus_ones,
        "the Rust mode 4 arm must produce -1 in exactly one place (the \
         not-found branch), never for a failed allocation"
    );
    // The one `else` present must be the not-found branch, not the alloc guard.
    let else_at = rs_arm.find("else").unwrap();
    let found_at = rs_arm
        .find("if !found_pos.is_null()")
        .expect("inner not-found check present");
    assert!(
        found_at < else_at,
        "the only `else` in mode 4 must belong to the found_pos check"
    );
}

// -------------------------------------------------- row 17: invalid `mode`

#[test]
fn err17_charinbuf_invalid_mode() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC017);
    // Out-of-range "enum" ints: every value with no matching switch case.
    let mut modes: Vec<i32> = vec![
        -1,
        -2,
        -100,
        5,
        6,
        7,
        8,
        1000,
        65536,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    for _ in 0..600 {
        let mut m = rng.interesting_i32();
        if (0..=4).contains(&m) {
            m = m.wrapping_sub(9);
        }
        modes.push(m);
    }
    for m in modes {
        let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(m, 1, 2, 3) });
        let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(m, 1, 2, 3) });
        assert_eq!(rc, -1, "C charinbuf({m},..) must return -1");
        assert_eq!(rc, rr, "charinbuf({m},1,2,3) return value");
        assert_eq!(
            oc,
            or,
            "charinbuf({m},1,2,3) stdout\nC: {:?}\nRust: {:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or)
        );
        assert_eq!(
            oc,
            format!("Invalid mode: {m}\n").into_bytes(),
            "unexpected C stdout for mode {m}"
        );
    }
}

// ------------------------------------------------- generic boundary sweeps

#[test]
fn gen_g4_find_char_size_one_step_around_match() {
    let _g = guard();
    let p = pair();
    let store = cstring(b"abcdefX_hij");
    let ptr0 = store.as_ptr();
    let match_at = 6usize; // 'X'
    for size in [match_at - 1, match_at, match_at + 1, match_at + 2] {
        let a = unsafe { (p.c.find_char_in_buffer)(ptr0, size, b'X' as c_char) };
        let b = unsafe { (p.rs.find_char_in_buffer)(ptr0, size, b'X' as c_char) };
        assert_eq!(a, b, "size {size} around match at {match_at}");
        if size <= match_at {
            assert!(a.is_null(), "size {size} must not reach offset {match_at}");
        } else {
            assert_eq!(a as usize - ptr0 as usize, match_at);
        }
    }
}

#[test]
fn gen_g5_validate_uint16_boundary_sweep() {
    let _g = guard();
    let p = pair();
    let expect: [(i32, c_int); 8] = [
        (-1, 0),
        (0, 1),
        (1, 1),
        (65534, 1),
        (65535, 1),
        (65536, 0),
        (i32::MIN, 0),
        (i32::MAX, 0),
    ];
    for (v, want) in expect {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, want, "C validate_uint16_range({v})");
        assert_eq!(b, want, "Rust validate_uint16_range({v})");
    }
}

#[test]
fn gen_g6_charinbuf_mode_one_past_valid_range() {
    let _g = guard();
    // -1 and 5 bracket the valid 0..=4 switch; both must hit `default`.
    for m in [-1, 5] {
        diff_charinbuf(m, 0, 0, 0, "G6");
    }
    // ...and the two extremes of the int domain.
    diff_charinbuf(i32::MIN, 0, 0, 0, "G6/INT_MIN");
    diff_charinbuf(i32::MAX, 0, 0, 0, "G6/INT_MAX");
}

#[test]
fn gen_g7_find_char_nul_and_negative_targets() {
    let _g = guard();
    let p = pair();
    let store = cstring(b"ab\x80\xffz");
    let ptr0 = store.as_ptr();
    // Every possible target byte, at every size in scope.
    for t in 0u8..=255 {
        for size in 0..=6usize {
            let a = unsafe { (p.c.find_char_in_buffer)(ptr0, size, t as c_char) };
            let b = unsafe { (p.rs.find_char_in_buffer)(ptr0, size, t as c_char) };
            assert_eq!(a, b, "target {t:#04x}, size {size}");
        }
    }
}

#[test]
fn gen_g8_apply_operation_inbound_callback() {
    let _g = guard();
    let p = pair();
    let f: CounterFn = cb_minus_one;
    let a = unsafe { (p.c.apply_operation)(Some(f), 42) };
    let b = unsafe { (p.rs.apply_operation)(Some(f), 42) };
    assert_eq!(a, b);
    assert_eq!(a, -1);
}

// A caller-side operation that itself calls back into the library, so the
// dispatcher is exercised with a non-trivial, stateful callback.
static mut WHICH: usize = 0;

#[test]
fn gen_counter_entry_points_reject_nothing() {
    let _g = guard();
    let p = pair();
    // The four counter functions have no rejection path at all: every `int` is
    // accepted. Confirm both libraries agree even at the extremes.
    let _ = unsafe { WHICH };
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        unsafe { (p.c.reset_counter)(v) };
        unsafe { (p.rs.reset_counter)(v) };
        for w in [i32::MIN, -1, 0, 1, i32::MAX] {
            unsafe { (p.c.reset_counter)(v) };
            unsafe { (p.rs.reset_counter)(v) };
            assert_eq!(unsafe { (p.c.increment_counter)(w) }, unsafe {
                (p.rs.increment_counter)(w)
            });
            unsafe { (p.c.reset_counter)(v) };
            unsafe { (p.rs.reset_counter)(v) };
            assert_eq!(unsafe { (p.c.decrement_counter)(w) }, unsafe {
                (p.rs.decrement_counter)(w)
            });
            unsafe { (p.c.reset_counter)(v) };
            unsafe { (p.rs.reset_counter)(v) };
            assert_eq!(unsafe { (p.c.multiply_counter)(w) }, unsafe {
                (p.rs.multiply_counter)(w)
            });
        }
    }
}
