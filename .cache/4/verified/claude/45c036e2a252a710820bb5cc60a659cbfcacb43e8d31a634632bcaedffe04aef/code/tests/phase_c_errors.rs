// Phase C — error-path differential tests, one test per row of ERRORS.md.
//
// Each test constructs the exact invalid input / condition the C code checks for
// and asserts that BOTH libraries reject it the same way: the same sentinel
// return value, and (for `charinbuf`) the same diagnostic bytes on stdout.
// "Both failed somehow" is never accepted — the concrete value is asserted.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const N: usize = 512;

// ===========================================================================
// Row 1 — is_string_empty(NULL) -> 1
// ===========================================================================

#[test]
fn err_01_is_string_empty_null() {
    let _g = gate();
    let (c, r) = apis();
    let cv = unsafe { (c.is_string_empty)(std::ptr::null()) };
    let rv = unsafe { (r.is_string_empty)(std::ptr::null()) };
    assert_eq!(cv, 1, "C is_string_empty(NULL) must return 1");
    assert_eq!(rv, 1, "Rust is_string_empty(NULL) must return 1");
    assert_eq!(cv, rv);
    diff_is_string_empty_raw(std::ptr::null(), "NULL");
}

// ===========================================================================
// Row 2 — is_string_empty("") -> 1
// ===========================================================================

#[test]
fn err_02_is_string_empty_empty() {
    let _g = gate();
    let (c, r) = apis();
    let s = b"\0";
    let p = s.as_ptr().cast::<c_char>();
    let cv = unsafe { (c.is_string_empty)(p) };
    let rv = unsafe { (r.is_string_empty)(p) };
    assert_eq!(cv, 1, "C is_string_empty(\"\") must return 1");
    assert_eq!(rv, 1, "Rust is_string_empty(\"\") must return 1");
    // A NUL first byte wins even when more bytes follow.
    diff_is_string_empty(b"\0trailing garbage\0");
}

// ===========================================================================
// Row 3 — is_string_empty(non-empty) -> 0, for every possible first byte
// ===========================================================================

#[test]
fn err_03_is_string_empty_nonempty_all_bytes() {
    let _g = gate();
    let (c, r) = apis();
    for b in 1u8..=255 {
        let s = [b, 0u8];
        let p = s.as_ptr().cast::<c_char>();
        let cv = unsafe { (c.is_string_empty)(p) };
        let rv = unsafe { (r.is_string_empty)(p) };
        assert_eq!(cv, 0, "C is_string_empty([{b:#04x}]) must return 0");
        assert_eq!(rv, 0, "Rust is_string_empty([{b:#04x}]) must return 0");
    }
}

// ===========================================================================
// Row 4 — find_char_in_buffer(NULL, ...) -> NULL
//         The null check must short-circuit before memchr reads anything, even
//         with an absurd size.
// ===========================================================================

#[test]
fn err_04_find_char_null_buffer() {
    let _g = gate();
    let (c, r) = apis();
    for size in [0usize, 1, 16, 4096, usize::MAX / 2, usize::MAX] {
        for target in [0u8, b'X', 0xFF] {
            let cp = unsafe { (c.find_char_in_buffer)(std::ptr::null(), size, target as c_char) };
            let rp = unsafe { (r.find_char_in_buffer)(std::ptr::null(), size, target as c_char) };
            assert!(
                cp.is_null(),
                "C find_char_in_buffer(NULL, {size}, {target:#04x}) must return NULL, got {cp:?}"
            );
            assert!(
                rp.is_null(),
                "Rust find_char_in_buffer(NULL, {size}, {target:#04x}) must return NULL, got {rp:?}"
            );
            diff_find_char_raw(
                std::ptr::null(),
                size,
                target,
                &format!("NULL, size={size}, target={target:#04x}"),
            );
        }
    }
}

// ===========================================================================
// Row 5 — target absent within `size` -> NULL
// ===========================================================================

#[test]
fn err_05_find_char_absent() {
    let _g = gate();
    let (c, r) = apis();
    let buf = b"abcdefghij";
    for target in [b'X', b'Z', 0u8, 0xFF, b'A'] {
        assert!(!buf.contains(&target), "test buffer must not contain target");
        let p = buf.as_ptr().cast::<c_char>();
        let cp = unsafe { (c.find_char_in_buffer)(p, buf.len(), target as c_char) };
        let rp = unsafe { (r.find_char_in_buffer)(p, buf.len(), target as c_char) };
        assert!(cp.is_null(), "C must return NULL for absent {target:#04x}");
        assert!(rp.is_null(), "Rust must return NULL for absent {target:#04x}");
        diff_find_char(buf, buf.len(), target);
    }

    // Randomized: guaranteed-absent targets over random buffers.
    let mut rng = Rng::new(0xE05_0000_0000_0005);
    for _ in 0..N {
        let len = rng.below(64);
        let buf: Vec<u8> = (0..len).map(|_| (rng.next_u8() % 26) + b'a').collect();
        let target = b'0' + (rng.next_u8() % 10); // never in 'a'..='z'
        diff_find_char(&buf, len, target);
    }
}

// ===========================================================================
// Row 6 — size == 0 rejects even a match at buffer[0]
// ===========================================================================

#[test]
fn err_06_find_char_zero_size() {
    let _g = gate();
    let (c, r) = apis();
    let buf = b"XYZ";
    let p = buf.as_ptr().cast::<c_char>();
    for target in [b'X', b'Y', b'Z', 0u8] {
        let cp = unsafe { (c.find_char_in_buffer)(p, 0, target as c_char) };
        let rp = unsafe { (r.find_char_in_buffer)(p, 0, target as c_char) };
        assert!(cp.is_null(), "C size=0 must return NULL for {target:#04x}");
        assert!(rp.is_null(), "Rust size=0 must return NULL for {target:#04x}");
    }
    // Every byte value, with a buffer that starts with it.
    for b in 0u8..=255 {
        let buf = [b, b, b];
        diff_find_char(&buf, 0, b);
    }
}

// ===========================================================================
// Row 7 — size shorter than the match position truncates the search
// ===========================================================================

#[test]
fn err_07_find_char_size_truncates_match() {
    let _g = gate();
    let (c, r) = apis();
    let buf = b"aaaaX";
    let p = buf.as_ptr().cast::<c_char>();
    // 'X' lives at index 4, so sizes 0..=4 must all miss and size 5 must hit.
    for size in 0..=4usize {
        let cp = unsafe { (c.find_char_in_buffer)(p, size, b'X' as c_char) };
        let rp = unsafe { (r.find_char_in_buffer)(p, size, b'X' as c_char) };
        assert!(cp.is_null(), "C size={size} must miss 'X' at index 4");
        assert!(rp.is_null(), "Rust size={size} must miss 'X' at index 4");
    }
    let cp = unsafe { (c.find_char_in_buffer)(p, 5, b'X' as c_char) };
    let rp = unsafe { (r.find_char_in_buffer)(p, 5, b'X' as c_char) };
    assert!(!cp.is_null() && !rp.is_null(), "size=5 must hit");
    assert_eq!(
        unsafe { cp.offset_from(p) },
        4,
        "C must report index 4"
    );
    assert_eq!(
        unsafe { rp.offset_from(p) },
        4,
        "Rust must report index 4"
    );

    // Randomized sweep across the boundary.
    let mut rng = Rng::new(0xE07_0000_0000_0007);
    for _ in 0..N {
        let len = 1 + rng.below(32);
        let pos = rng.below(len);
        let mut buf: Vec<u8> = vec![b'.'; len];
        buf[pos] = b'!';
        for size in [pos, pos + 1, len] {
            diff_find_char(&buf, size, b'!');
        }
    }
}

// ===========================================================================
// Row 8 — create_buffer(NULL) -> NULL
// ===========================================================================

#[test]
fn err_08_create_buffer_null() {
    let _g = gate();
    let (c, r) = apis();
    let cb = unsafe { (c.create_buffer)(std::ptr::null()) };
    let rb = unsafe { (r.create_buffer)(std::ptr::null()) };
    assert!(cb.is_null(), "C create_buffer(NULL) must return NULL");
    assert!(rb.is_null(), "Rust create_buffer(NULL) must return NULL");
    // Repeat: must be stable and must not allocate.
    for _ in 0..64 {
        let cb = unsafe { (c.create_buffer)(std::ptr::null()) };
        let rb = unsafe { (r.create_buffer)(std::ptr::null()) };
        assert!(cb.is_null() && rb.is_null());
    }
}

// ===========================================================================
// Rows 10-12 — validate_uint16_range rejections and the accepted boundaries
// ===========================================================================

#[test]
fn err_10_validate_negative() {
    let _g = gate();
    let (c, r) = apis();
    for v in [-1, -2, -100, -65535, -65536, i32::MIN + 1, i32::MIN] {
        let cv = (c.validate_uint16_range)(v);
        let rv = (r.validate_uint16_range)(v);
        assert_eq!(cv, 0, "C validate_uint16_range({v}) must return 0");
        assert_eq!(rv, 0, "Rust validate_uint16_range({v}) must return 0");
    }
    let mut rng = Rng::new(0xE10_0000_0000_0010);
    for _ in 0..N * 2 {
        let v = -1 - (rng.next_u32() % (i32::MAX as u32)) as i32;
        let cv = (c.validate_uint16_range)(v);
        let rv = (r.validate_uint16_range)(v);
        assert_eq!(cv, 0, "C validate_uint16_range({v})");
        assert_eq!(rv, 0, "Rust validate_uint16_range({v})");
    }
}

#[test]
fn err_11_validate_above_max() {
    let _g = gate();
    let (c, r) = apis();
    for v in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        let cv = (c.validate_uint16_range)(v);
        let rv = (r.validate_uint16_range)(v);
        assert_eq!(cv, 0, "C validate_uint16_range({v}) must return 0");
        assert_eq!(rv, 0, "Rust validate_uint16_range({v}) must return 0");
    }
    let mut rng = Rng::new(0xE11_0000_0000_0011);
    for _ in 0..N * 2 {
        let v = 65536 + (rng.next_u32() % (i32::MAX as u32 - 65536)) as i32;
        let cv = (c.validate_uint16_range)(v);
        let rv = (r.validate_uint16_range)(v);
        assert_eq!(cv, 0, "C validate_uint16_range({v})");
        assert_eq!(rv, 0, "Rust validate_uint16_range({v})");
    }
}

#[test]
fn err_12_validate_boundaries() {
    let _g = gate();
    let (c, r) = apis();
    // One step past each edge, and the edges themselves.
    let cases: [(i32, c_int); 6] = [
        (-1, 0),
        (0, 1),
        (1, 1),
        (65534, 1),
        (65535, 1),
        (65536, 0),
    ];
    for (v, expected) in cases {
        let cv = (c.validate_uint16_range)(v);
        let rv = (r.validate_uint16_range)(v);
        assert_eq!(cv, expected, "C validate_uint16_range({v})");
        assert_eq!(rv, expected, "Rust validate_uint16_range({v})");
    }
}

// ===========================================================================
// Rows 13-14 — apply_operation
// ===========================================================================

#[test]
fn err_13_apply_operation_null() {
    let _g = gate();
    let (c, r) = apis();
    for v in [0, 1, -1, 42, i32::MAX, i32::MIN, 65535, 65536] {
        let cv = unsafe { (c.apply_operation)(std::ptr::null(), v) };
        let rv = unsafe { (r.apply_operation)(std::ptr::null(), v) };
        assert_eq!(cv, -1, "C apply_operation(NULL, {v}) must return -1");
        assert_eq!(rv, -1, "Rust apply_operation(NULL, {v}) must return -1");
    }

    // A NULL callback must not disturb the hidden counter either.
    (c.reset_counter)(1234);
    (r.reset_counter)(1234);
    unsafe {
        (c.apply_operation)(std::ptr::null(), 99);
        (r.apply_operation)(std::ptr::null(), 99);
    }
    assert_eq!(c.peek_counter(), 1234, "C counter must be untouched");
    assert_eq!(r.peek_counter(), 1234, "Rust counter must be untouched");
}

extern "C" fn always_minus_one(_v: c_int) -> c_int {
    -1
}

#[test]
fn err_14_apply_operation_callee_returns_minus_one() {
    let _g = gate();
    let p = always_minus_one as *const c_void;
    // The -1 sentinel is indistinguishable from a legitimate -1: both libraries
    // must return -1 here for the same reason (the callee said so), proving the
    // Rust `Option<extern "C" fn>` niche did not turn a valid pointer into None.
    for v in [0, 1, -1, i32::MAX, i32::MIN] {
        diff_apply_operation_raw(p, p, v, "always_minus_one");
    }

    // And a callback that returns -1 must still be *called* — verified by using
    // the library's own decrement_counter to reach -1 through real state.
    let (c, r) = apis();
    (c.reset_counter)(0);
    (r.reset_counter)(0);
    let cv = unsafe { (c.apply_operation)(c.p_decrement, 1) };
    let rv = unsafe { (r.apply_operation)(r.p_decrement, 1) };
    assert_eq!(cv, -1, "C decrement from 0 must yield -1");
    assert_eq!(rv, -1, "Rust decrement from 0 must yield -1");
    assert_eq!(c.peek_counter(), -1, "C counter must actually be -1");
    assert_eq!(r.peek_counter(), -1, "Rust counter must actually be -1");
}

// ===========================================================================
// Row 15 — charinbuf: unknown mode -> "Invalid mode: %d" and -1
// ===========================================================================

#[test]
fn err_15_charinbuf_invalid_mode() {
    let _g = gate();
    let (c, r) = apis();
    for m in [-1, 5, 6, 7, 100, -100, i32::MIN, i32::MAX] {
        // Exact bytes and exact sentinel, not just "both non-zero".
        let (cv, cout) = capture(|| (c.charinbuf)(m, 0, 0, 0));
        let (rv, rout) = capture(|| (r.charinbuf)(m, 0, 0, 0));
        assert_eq!(cv, -1, "C charinbuf(mode={m}) must return -1");
        assert_eq!(rv, -1, "Rust charinbuf(mode={m}) must return -1");
        let expected = format!("Invalid mode: {m}\n").into_bytes();
        assert_eq!(
            cout, expected,
            "C stdout for mode={m} was \"{}\"",
            show(&cout)
        );
        assert_eq!(
            rout, expected,
            "Rust stdout for mode={m} was \"{}\"",
            show(&rout)
        );
    }
}

// ===========================================================================
// Row 16 — charinbuf mode 0 with an out-of-range value -> -1
// ===========================================================================

#[test]
fn err_16_charinbuf_mode0_out_of_range() {
    let _g = gate();
    let (c, r) = apis();
    for v in [-1, -2, 65536, 65537, i32::MIN, i32::MAX] {
        let (cv, cout) = capture(|| (c.charinbuf)(0, v, 0, 0));
        let (rv, rout) = capture(|| (r.charinbuf)(0, v, 0, 0));
        assert_eq!(cv, -1, "C charinbuf(0, {v}) must return -1");
        assert_eq!(rv, -1, "Rust charinbuf(0, {v}) must return -1");
        let expected = format!(
            "Mode 0: UINT16_MAX validation\n\
             Checking if value {v} is within uint16_t range...\n\
             Value {v} is out of range for uint16_t\n\
             UINT16_MAX constant value: 65535\n"
        )
        .into_bytes();
        assert_eq!(cout, expected, "C stdout was \"{}\"", show(&cout));
        assert_eq!(rout, expected, "Rust stdout was \"{}\"", show(&rout));
    }
}

// ===========================================================================
// Row 19 — charinbuf mode 1: the "check failed" branch is unreachable, so the
//          reachable side is pinned exactly.
// ===========================================================================

#[test]
fn err_19_charinbuf_mode1_dead_branch() {
    let _g = gate();
    let (c, r) = apis();
    let (cv, cout) = capture(|| (c.charinbuf)(1, 0, 0, 0));
    let (rv, rout) = capture(|| (r.charinbuf)(1, 0, 0, 0));

    let expected: Vec<u8> = concat!(
        "Mode 1: String empty check by dereference\n",
        "Test string is empty (checked with *string)\n",
        "Non-empty string correctly identified\n",
    )
    .as_bytes()
    .to_vec();

    assert_eq!(cv, 10, "C mode 1 must return 0 + 10");
    assert_eq!(rv, 10, "Rust mode 1 must return 0 + 10");
    assert_eq!(cout, expected, "C stdout was \"{}\"", show(&cout));
    assert_eq!(rout, expected, "Rust stdout was \"{}\"", show(&rout));

    // The dead branch would print this instead; neither may ever do so.
    assert!(
        !cout.windows(9).any(|w| w == b"check fai"),
        "C took the unreachable branch"
    );
    assert!(
        !rout.windows(9).any(|w| w == b"check fai"),
        "Rust took the unreachable branch"
    );
}

// ===========================================================================
// Row 20 — out-of-range "enum" values for `mode` across the FFI boundary.
//          `mode` is a plain int, so every one of the 2^32 values is a real
//          input; everything outside 0..=4 must take `default:`.
// ===========================================================================

#[test]
fn err_20_charinbuf_mode_exhaustive_boundaries() {
    let _g = gate();
    // One step past each end of the valid switch range, plus the extremes.
    for m in [-3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7] {
        diff_charinbuf_with_state(m, 7, 3, 2);
    }
    for m in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        diff_charinbuf_with_state(m, 0, 0, 0);
    }

    let mut rng = Rng::new(0xE20_0000_0000_0020);
    for _ in 0..N {
        let m = rng.next_i32();
        diff_charinbuf_with_state(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }

    // value one step past its documented range, for the mode that checks it.
    for v in [-1, 0, 65535, 65536] {
        diff_charinbuf_with_state(0, v, 0, 0);
    }
}

// ===========================================================================
// Row 22 — INT_MIN / INT_MAX in every int parameter
// ===========================================================================

#[test]
fn err_21_charinbuf_extreme_int_params() {
    let _g = gate();
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for mode in 0..=5 {
        for &v in &extremes {
            for &o1 in &extremes {
                for &o2 in &extremes {
                    diff_charinbuf_with_state(mode, v, o1, o2);
                }
            }
        }
    }
}

// ===========================================================================
// Row 24 — target == '\0' is an ACCEPT for memchr (not strchr semantics)
// ===========================================================================

#[test]
fn err_22_find_char_nul_target() {
    let _g = gate();
    let (c, r) = apis();
    let buf = b"ab\0cd";
    let p = buf.as_ptr().cast::<c_char>();
    // Inside the window -> hit at index 2.
    let cp = unsafe { (c.find_char_in_buffer)(p, buf.len(), 0) };
    let rp = unsafe { (r.find_char_in_buffer)(p, buf.len(), 0) };
    assert!(!cp.is_null(), "C must find the NUL (memchr, not strchr)");
    assert!(!rp.is_null(), "Rust must find the NUL (memchr, not strchr)");
    assert_eq!(unsafe { cp.offset_from(p) }, 2);
    assert_eq!(unsafe { rp.offset_from(p) }, 2);
    // Outside the window -> miss.
    for size in 0..=2usize {
        diff_find_char(buf, size, 0);
    }
    for size in 3..=buf.len() {
        diff_find_char(buf, size, 0);
    }
}

// ===========================================================================
// Row 25 — high-bit targets: C sign-extends char->int, Rust zero-extends;
//          memchr masks to unsigned char so both must agree on all 128 values.
// ===========================================================================

#[test]
fn err_23_find_char_high_bit_target() {
    let _g = gate();
    let (c, r) = apis();
    for b in 0x80u8..=0xFF {
        let buf = [0x00u8, b, 0x7F, b];
        let p = buf.as_ptr().cast::<c_char>();
        let cp = unsafe { (c.find_char_in_buffer)(p, buf.len(), b as c_char) };
        let rp = unsafe { (r.find_char_in_buffer)(p, buf.len(), b as c_char) };
        assert!(!cp.is_null(), "C must find {b:#04x}");
        assert!(!rp.is_null(), "Rust must find {b:#04x}");
        assert_eq!(
            unsafe { cp.offset_from(p) },
            1,
            "C wrong position for {b:#04x}"
        );
        assert_eq!(
            unsafe { rp.offset_from(p) },
            1,
            "Rust wrong position for {b:#04x}"
        );

        // The corresponding *positive* char must NOT match the high-bit byte
        // (i.e. no truncation-to-7-bits bug).
        let low = b & 0x7F;
        let only_high = [b, b, b];
        diff_find_char(&only_high, 3, low);
    }
}

// ===========================================================================
// Row 26 — create_buffer("") is the smallest accepted input (1-byte malloc)
// ===========================================================================

#[test]
fn err_24_create_buffer_empty_string() {
    let _g = gate();
    let (c, r) = apis();
    let s = b"\0";
    let p = s.as_ptr().cast::<c_char>();
    let cb = unsafe { (c.create_buffer)(p) };
    let rb = unsafe { (r.create_buffer)(p) };
    assert!(!cb.is_null(), "C create_buffer(\"\") must allocate");
    assert!(!rb.is_null(), "Rust create_buffer(\"\") must allocate");
    assert_eq!(unsafe { libc_strlen(cb) }, 0);
    assert_eq!(unsafe { libc_strlen(rb) }, 0);
    assert_eq!(unsafe { *cb }, 0, "C must write the NUL terminator");
    assert_eq!(unsafe { *rb }, 0, "Rust must write the NUL terminator");
    unsafe {
        libc_free(cb);
        libc_free(rb);
    }
    diff_create_buffer(b"\0");
}
