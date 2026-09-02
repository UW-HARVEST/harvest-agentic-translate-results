//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI-boundary rows G1..G7.
//! Every test asserts the two libraries return the *same* sentinel, not merely
//! that both failed.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const ITERS: usize = 200;

// ===========================================================================
// Row 1 / G1 — is_string_empty(NULL) == 1
// ===========================================================================

#[test]
fn err_01_is_string_empty_null() {
    let p = pair();
    let a = unsafe { (p.c.is_string_empty)(std::ptr::null()) };
    let b = unsafe { (p.r.is_string_empty)(std::ptr::null()) };
    assert_eq!(a, 1, "C: NULL must yield 1");
    assert_eq!(b, 1, "Rust: NULL must yield 1");
    assert_eq!(a, b);
}

// ===========================================================================
// Row 2 — is_string_empty("") == 1 (same sentinel as NULL: conflated in C)
// ===========================================================================

#[test]
fn err_02_is_string_empty_empty() {
    let p = pair();
    let s = cstring(b"");
    let ptr = s.as_ptr() as *const c_char;
    let a = unsafe { (p.c.is_string_empty)(ptr) };
    let b = unsafe { (p.r.is_string_empty)(ptr) };
    assert_eq!(a, 1);
    assert_eq!(b, 1);
    // The C conflates NULL and "" — both must give the identical sentinel.
    assert_eq!(a, unsafe { (p.c.is_string_empty)(std::ptr::null()) });
    assert_eq!(b, unsafe { (p.r.is_string_empty)(std::ptr::null()) });
}

// ===========================================================================
// Row 3 / G1 — find_char_in_buffer(NULL, ..) == NULL, even for size != 0
// ===========================================================================

#[test]
fn err_03_find_char_null_buffer() {
    let p = pair();
    let mut rng = Rng::new(0xC003);
    for size in [0usize, 1, 7, 4096, usize::MAX] {
        for target in [0u8, 1, b'X', 0x80, 0xFF] {
            let t = target as i8 as c_char;
            let a = unsafe { (p.c.find_char_in_buffer)(std::ptr::null(), size, t) };
            let b = unsafe { (p.r.find_char_in_buffer)(std::ptr::null(), size, t) };
            assert!(a.is_null(), "C must return NULL (size={size})");
            assert!(b.is_null(), "Rust must return NULL (size={size})");
        }
    }
    for _ in 0..ITERS {
        let size = rng.next_u64() as usize;
        let t = rng.next_u32() as u8 as i8 as c_char;
        let a = unsafe { (p.c.find_char_in_buffer)(std::ptr::null(), size, t) };
        let b = unsafe { (p.r.find_char_in_buffer)(std::ptr::null(), size, t) };
        assert_eq!(a.is_null(), b.is_null());
        assert!(a.is_null());
    }
}

// ===========================================================================
// Row 4 — target absent -> NULL
// ===========================================================================

#[test]
fn err_04_find_char_absent() {
    let p = pair();
    let mut rng = Rng::new(0xC004);
    for _ in 0..(ITERS * 4) {
        let len = (rng.below(64) + 1) as usize;
        // Build a buffer from one half of the byte space, search the other half.
        let buf: Vec<u8> = (0..len).map(|_| 0x01 + (rng.next_u32() as u8 % 0x40)).collect();
        let target = 0x90u8 + (rng.next_u32() as u8 % 0x40);
        assert!(!buf.contains(&target));
        let base = buf.as_ptr() as *const c_char;
        let t = target as i8 as c_char;
        let a = unsafe { (p.c.find_char_in_buffer)(base, len, t) };
        let b = unsafe { (p.r.find_char_in_buffer)(base, len, t) };
        assert!(a.is_null(), "C must return NULL for absent target 0x{target:02x}");
        assert!(b.is_null(), "Rust must return NULL for absent target 0x{target:02x}");
    }
}

// ===========================================================================
// Row 5 / G2 — size == 0 -> NULL even when byte 0 matches
// ===========================================================================

#[test]
fn err_05_find_char_zero_size() {
    let p = pair();
    for target in [0u8, b'A', 0xFF] {
        let buf = vec![target; 8];
        let base = buf.as_ptr() as *const c_char;
        let t = target as i8 as c_char;
        let a = unsafe { (p.c.find_char_in_buffer)(base, 0, t) };
        let b = unsafe { (p.r.find_char_in_buffer)(base, 0, t) };
        assert!(a.is_null(), "C: size 0 must return NULL");
        assert!(b.is_null(), "Rust: size 0 must return NULL");
    }
}

// ===========================================================================
// Row 6 / G1 — create_buffer(NULL) == NULL
// ===========================================================================

#[test]
fn err_06_create_buffer_null() {
    let p = pair();
    for _ in 0..8 {
        let a = unsafe { (p.c.create_buffer)(std::ptr::null()) };
        let b = unsafe { (p.r.create_buffer)(std::ptr::null()) };
        assert!(a.is_null(), "C: NULL in -> NULL out");
        assert!(b.is_null(), "Rust: NULL in -> NULL out");
    }
}

// ===========================================================================
// Row 7 — create_buffer with a failing malloc returns NULL unchanged
// ===========================================================================

#[test]
fn err_07_create_buffer_oom_path() {
    // A real OOM cannot be forced in-process without tearing down the harness
    // (both `.so`s share this process' libc allocator, and so does the test
    // harness itself). What *is* checkable, and is checked here, is that the
    // only NULL-returning paths are the ones the C has: a NULL argument, and a
    // failing `malloc`. Under a working allocator both libraries must therefore
    // return non-NULL for every size we can ask for, including a very large
    // one, and NULL only for a NULL argument.
    let p = pair();
    for len in [0usize, 1, 1024, 1 << 20] {
        let src = cstring(&vec![b'z'; len]);
        let sp = src.as_ptr() as *const c_char;
        let a = unsafe { (p.c.create_buffer)(sp) };
        let b = unsafe { (p.r.create_buffer)(sp) };
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "create_buffer nullness must agree for len={len}"
        );
        assert!(!a.is_null(), "allocator unexpectedly failed for len={len}");
        assert_eq!(unsafe { c_strlen(a) }, len);
        assert_eq!(unsafe { c_strlen(b) }, len);
        unsafe {
            c_free(a);
            c_free(b);
        }
    }
    // And the NULL-argument sentinel is identical (row 6 overlap, asserted here
    // so this row stands alone).
    assert!(unsafe { (p.c.create_buffer)(std::ptr::null()) }.is_null());
    assert!(unsafe { (p.r.create_buffer)(std::ptr::null()) }.is_null());
}

// ===========================================================================
// Rows 8, 9 / G4 — validate_uint16_range rejections
// ===========================================================================

#[test]
fn err_08_validate_negative() {
    let p = pair();
    let mut rng = Rng::new(0xC008);
    let mut cases: Vec<c_int> = vec![-1, -2, -65535, -65536, -65537, i32::MIN, i32::MIN + 1];
    for _ in 0..ITERS {
        cases.push(rng.range_i32(i32::MIN, -1));
    }
    for v in cases {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.r.validate_uint16_range)(v) };
        assert_eq!(a, 0, "C: validate_uint16_range({v}) must be 0");
        assert_eq!(b, 0, "Rust: validate_uint16_range({v}) must be 0");
    }
}

#[test]
fn err_09_validate_above_max() {
    let p = pair();
    let mut rng = Rng::new(0xC009);
    let mut cases: Vec<c_int> = vec![65536, 65537, 131071, i32::MAX, i32::MAX - 1];
    for _ in 0..ITERS {
        cases.push(rng.range_i32(65536, i32::MAX));
    }
    for v in cases {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.r.validate_uint16_range)(v) };
        assert_eq!(a, 0, "C: validate_uint16_range({v}) must be 0");
        assert_eq!(b, 0, "Rust: validate_uint16_range({v}) must be 0");
    }
}

// ===========================================================================
// Row 10 / G1 — apply_operation(NULL, v) == -1 and leaves the counter alone
// ===========================================================================

#[test]
fn err_10_apply_operation_null_op() {
    let p = pair();
    let mut rng = Rng::new(0xC010);
    let _g = guard();
    for _ in 0..ITERS {
        let seed = rng.interesting_i32();
        p.c.call_mut(MutOp::Reset, seed);
        p.r.call_mut(MutOp::Reset, seed);

        let v = rng.interesting_i32();
        let a = unsafe { (p.c.apply_operation)(std::ptr::null::<c_void>(), v) };
        let b = unsafe { (p.r.apply_operation)(std::ptr::null::<c_void>(), v) };
        assert_eq!(a, -1, "C: NULL op must yield -1");
        assert_eq!(b, -1, "Rust: NULL op must yield -1");

        // The callback must not have run: the counter is still `seed`.
        let after_c = p.c.call_mut(MutOp::Increment, 0);
        let after_r = p.r.call_mut(MutOp::Increment, 0);
        assert_eq!(after_c, seed, "C counter must be untouched");
        assert_eq!(after_r, seed, "Rust counter must be untouched");
    }
}

// ===========================================================================
// Row 11 — charinbuf mode 0 with an out-of-range value
// ===========================================================================

#[test]
fn err_11_charinbuf_mode0_out_of_range() {
    let mut rng = Rng::new(0xC011);
    let mut cases: Vec<c_int> = vec![-1, i32::MIN, i32::MIN + 1, 65536, 65537, i32::MAX];
    for _ in 0..ITERS {
        cases.push(if rng.below(2) == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(65536, i32::MAX)
        });
    }
    for v in cases {
        let (rc, out) = diff_charinbuf(0, v, 0, 0);
        assert_eq!(rc, -1, "mode 0 value={v} must return -1");
        let s = show(&out);
        assert!(
            s.contains(&format!("Value {v} is out of range for uint16_t")),
            "missing rejection message for {v}: {s:?}"
        );
        assert!(!s.contains("is valid (0 <= value"));
    }
}

// ===========================================================================
// Rows 12, 13, 14 — branches that a working allocator / the fixed literals
// make unreachable. Asserted as *never taken*, identically in both libraries.
// ===========================================================================

#[test]
fn err_12_charinbuf_mode2_alloc_fail_unreached() {
    // `create_buffer("Testing malloc and free")` cannot fail here, so the
    // `Failed to allocate buffer` / `-1` branch must never be observed — in
    // either library. A divergence would show up as one library taking it.
    for _ in 0..32 {
        let (rc, out) = diff_charinbuf(2, 0, 0, 0);
        let s = show(&out);
        assert_ne!(rc, -1, "mode 2 must not report allocation failure");
        assert!(!s.contains("Failed to allocate buffer"));
        assert!(s.contains("Buffer freed successfully"));
    }
}

#[test]
fn err_13_charinbuf_mode4_notfound_unreached() {
    // The literal always contains 'X', so `find_char_in_buffer` cannot miss and
    // the `-1` / "not found" branch must never be observed. The *reachable*
    // form of this rejection is covered directly by `err_04_find_char_absent`.
    for _ in 0..32 {
        let (rc, out) = diff_charinbuf(4, 0, 0, 0);
        let s = show(&out);
        assert_ne!(rc, -1, "mode 4 must not report a miss");
        assert!(!s.contains("Character 'X' not found"));
        assert!(s.contains("Found 'X' at position: 21"));
    }
}

#[test]
fn err_14_charinbuf_mode4_alloc_fail_unreached() {
    // Note the asymmetry with mode 2: mode 4 has NO `else`, so a failing
    // allocation would return the initial `result = 0` and print only the
    // header. Assert neither library produces that shape.
    for _ in 0..32 {
        let (rc, out) = diff_charinbuf(4, 0, 0, 0);
        let s = show(&out);
        assert_ne!(rc, 0, "mode 4 with a working allocator must not return 0");
        assert!(
            s.lines().count() > 1,
            "mode 4 must print more than just the header: {s:?}"
        );
    }
}

// ===========================================================================
// Row 15 / G5 — charinbuf default branch
// ===========================================================================

#[test]
fn err_15_charinbuf_invalid_mode() {
    for mode in [-1, 5, 6, 100, -100] {
        let (rc, out) = diff_charinbuf(mode, 0, 0, 0);
        assert_eq!(rc, -1, "mode {mode} must return -1");
        assert_eq!(
            show(&out),
            format!("Invalid mode: {mode}\n"),
            "default branch output for mode {mode}"
        );
    }
}

#[test]
fn err_16_charinbuf_mode_boundaries() {
    // One step past each end of the valid `switch` range, in both directions.
    for (mode, expect_default) in [(-2, true), (-1, true), (0, false), (4, false), (5, true), (6, true)] {
        let (rc, out) = diff_charinbuf(mode, 0, 0, 0);
        let s = show(&out);
        assert_eq!(
            s.starts_with("Invalid mode:"),
            expect_default,
            "mode {mode} took the wrong branch: {s:?}"
        );
        if expect_default {
            assert_eq!(rc, -1);
        }
    }
    // Extreme int32 values: a C `switch` on an `int` accepts any bit pattern.
    for mode in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        let (rc, out) = diff_charinbuf(mode, 0, 0, 0);
        assert_eq!(rc, -1);
        assert_eq!(show(&out), format!("Invalid mode: {mode}\n"));
    }
}

#[test]
fn err_17_charinbuf_mode_fuzz() {
    // Out-of-range "enum" values crossing the FFI boundary: `mode` is a plain
    // C `int`, so every 32-bit pattern is a real input.
    let mut rng = Rng::new(0xC017);
    let _g = guard();
    for _ in 0..(ITERS * 4) {
        let mode = rng.interesting_i32();
        let (rc, out) = diff_charinbuf_locked(mode, rng.next_i32(), rng.next_i32(), rng.next_i32());
        if !(0..=4).contains(&mode) {
            assert_eq!(rc, -1, "mode {mode} must return -1");
            assert_eq!(show(&out), format!("Invalid mode: {mode}\n"));
        }
    }
}

// ===========================================================================
// G3 — oversized `size` for find_char_in_buffer
// ===========================================================================

#[test]
fn err_18_find_char_oversized_size() {
    let p = pair();
    // (a) `size` past the end of the data but the match is inside: memchr stops
    //     at the first hit, so no out-of-bounds read happens.
    let mut buf = vec![b'.'; 4096];
    buf[10] = b'!';
    let base = buf.as_ptr() as *const c_char;
    for size in [11usize, 12, 4096, 4096 + 1000] {
        let a = unsafe { (p.c.find_char_in_buffer)(base, size, b'!' as c_char) };
        let b = unsafe { (p.r.find_char_in_buffer)(base, size, b'!' as c_char) };
        assert!(!a.is_null() && !b.is_null());
        let oa = unsafe { a.offset_from(base as *mut c_char) };
        let ob = unsafe { b.offset_from(base as *mut c_char) };
        assert_eq!(oa, ob);
        assert_eq!(oa, 10, "size={size}");
    }

    // (b) `size == SIZE_MAX` with the target filling the whole buffer, so the
    //     very first (possibly vectorised) load already contains a match.
    let full = vec![0xC5u8; 4096];
    let fbase = full.as_ptr() as *const c_char;
    let t = 0xC5u8 as i8 as c_char;
    let a = unsafe { (p.c.find_char_in_buffer)(fbase, usize::MAX, t) };
    let b = unsafe { (p.r.find_char_in_buffer)(fbase, usize::MAX, t) };
    assert!(!a.is_null() && !b.is_null());
    assert_eq!(
        unsafe { a.offset_from(fbase as *mut c_char) },
        unsafe { b.offset_from(fbase as *mut c_char) }
    );
    assert_eq!(unsafe { a.offset_from(fbase as *mut c_char) }, 0);
}

// ===========================================================================
// G4 — one step past every documented range
// ===========================================================================

#[test]
fn err_19_one_past_range() {
    let p = pair();
    // validate_uint16_range: -1 / 0 / 65535 / 65536
    for (v, expect) in [(-1, 0), (0, 1), (65535, 1), (65536, 0)] {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.r.validate_uint16_range)(v) };
        assert_eq!(a, expect, "C validate({v})");
        assert_eq!(b, expect, "Rust validate({v})");
    }
    // charinbuf mode 0 at the same four points: the return value flips between
    // `value` and -1 exactly at the boundary.
    for (v, expect) in [(-1, -1), (0, 0), (65535, 65535), (65536, -1)] {
        let (rc, _) = diff_charinbuf(0, v, 0, 0);
        assert_eq!(rc, expect, "charinbuf(0, {v})");
    }
    // charinbuf mode range: -1 / 0 / 4 / 5
    assert_eq!(diff_charinbuf(-1, 0, 0, 0).0, -1);
    assert_eq!(diff_charinbuf(5, 0, 0, 0).0, -1);
    diff_charinbuf(0, 0, 0, 0);
    diff_charinbuf(4, 0, 0, 0);
}

// ===========================================================================
// G6 — signed `char` target values (high bit set)
// ===========================================================================

#[test]
fn err_20_find_char_signed_target() {
    let p = pair();
    // For every byte value, a buffer containing exactly that byte once.
    for tb in 0u8..=255 {
        let mut buf = vec![0x2Au8; 16]; // '*' filler
        let pos = (tb as usize) % 16;
        buf[pos] = tb;
        let base = buf.as_ptr() as *const c_char;
        let t = tb as i8 as c_char;
        let a = unsafe { (p.c.find_char_in_buffer)(base, 16, t) };
        let b = unsafe { (p.r.find_char_in_buffer)(base, 16, t) };
        assert_eq!(a.is_null(), b.is_null(), "target 0x{tb:02x} nullness");
        if !a.is_null() {
            let oa = unsafe { a.offset_from(base as *mut c_char) };
            let ob = unsafe { b.offset_from(base as *mut c_char) };
            assert_eq!(oa, ob, "target 0x{tb:02x} offset: C={oa} Rust={ob}");
            let expect = buf.iter().position(|&x| x == tb).unwrap() as isize;
            assert_eq!(oa, expect, "target 0x{tb:02x} vs model");
        }
    }
    // A target absent from the buffer, for every high-bit value.
    for tb in 0x80u8..=0xFF {
        let buf = vec![0x2Au8; 16];
        let base = buf.as_ptr() as *const c_char;
        let t = tb as i8 as c_char;
        assert!(unsafe { (p.c.find_char_in_buffer)(base, 16, t) }.is_null());
        assert!(unsafe { (p.r.find_char_in_buffer)(base, 16, t) }.is_null());
    }
}

// ===========================================================================
// G7 — signed-overflow arithmetic in the counter mutators
// ===========================================================================

#[test]
fn err_21_counter_overflow() {
    let p = pair();
    let _g = guard();
    // INT_MAX + 1
    assert_eq!(p.c.call_mut(MutOp::Reset, i32::MAX), i32::MAX);
    assert_eq!(p.r.call_mut(MutOp::Reset, i32::MAX), i32::MAX);
    let a = p.c.call_mut(MutOp::Increment, 1);
    let b = p.r.call_mut(MutOp::Increment, 1);
    assert_eq!(a, b, "INT_MAX + 1: C={a} Rust={b}");
    assert_eq!(a, i32::MIN);

    // INT_MIN - 1
    p.c.call_mut(MutOp::Reset, i32::MIN);
    p.r.call_mut(MutOp::Reset, i32::MIN);
    let a = p.c.call_mut(MutOp::Decrement, 1);
    let b = p.r.call_mut(MutOp::Decrement, 1);
    assert_eq!(a, b, "INT_MIN - 1: C={a} Rust={b}");
    assert_eq!(a, i32::MAX);

    // INT_MIN * -1
    p.c.call_mut(MutOp::Reset, i32::MIN);
    p.r.call_mut(MutOp::Reset, i32::MIN);
    let a = p.c.call_mut(MutOp::Multiply, -1);
    let b = p.r.call_mut(MutOp::Multiply, -1);
    assert_eq!(a, b, "INT_MIN * -1: C={a} Rust={b}");
    assert_eq!(a, i32::MIN);

    // Large-magnitude multiplies, both signs.
    let mut rng = Rng::new(0xC021);
    for _ in 0..(ITERS * 4) {
        let start = rng.interesting_i32();
        p.c.call_mut(MutOp::Reset, start);
        p.r.call_mut(MutOp::Reset, start);
        let v = rng.interesting_i32();
        let op = MutOp::from_u32(rng.next_u32());
        let a = p.c.call_mut(op, v);
        let b = p.r.call_mut(op, v);
        assert_eq!(a, b, "{op:?}: {start} op {v} -> C={a} Rust={b}");
    }
    // And the same overflow through the indirect entry point.
    p.c.call_mut(MutOp::Reset, i32::MAX);
    p.r.call_mut(MutOp::Reset, i32::MAX);
    let a = unsafe { (p.c.apply_operation)(p.c.op_ptr(MutOp::Increment), 1) };
    let b = unsafe { (p.r.apply_operation)(p.r.op_ptr(MutOp::Increment), 1) };
    assert_eq!(a, b);
    assert_eq!(a, i32::MIN);
}
