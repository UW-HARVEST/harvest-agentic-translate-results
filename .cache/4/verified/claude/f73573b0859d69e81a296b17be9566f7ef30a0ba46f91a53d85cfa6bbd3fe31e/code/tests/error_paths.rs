//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Every test constructs the exact invalid
//! input, calls **both** implementations through their `.so` exports, and
//! asserts that they return the *same* sentinel (`-1` / `0`) — not merely that
//! both "failed somehow".
//!
//! Rows 1–25 and 29 exercise the `static` C helpers, so they need
//! `--features internal_test_api` (C side: `tests/cshim/shim.c`).  Rows 26, 27,
//! 28, 30 and 31 go through the shipped `memchra2` export and always run.

mod common;

use std::ffi::c_int;

use common::{c_lib, rust_lib, sym, Rng, SEED};

// ---------------------------------------------------------------------------
// Public surface (always available)
// ---------------------------------------------------------------------------

type FMemchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn memchra2_pair() -> (FMemchra2, FMemchra2) {
    unsafe {
        (
            sym::<FMemchra2>(c_lib(), "memchra2"),
            sym::<FMemchra2>(rust_lib(), "memchra2"),
        )
    }
}

#[track_caller]
fn same(label: &str, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let (fc, fr) = memchra2_pair();
    let rc = unsafe { fc(a, b, c, d) };
    let rr = unsafe { fr(a, b, c, d) };
    assert_eq!(
        rc, rr,
        "{label}: memchra2({a},{b},{c},{d}) C={rc} Rust={rr} (a bits {:#010x})",
        a as u32
    );
    rc
}

/// Row 26 — `f <= 0.0f` (a == 0, a with the sign bit set): float branch rejected.
#[test]
fn err26_memchra2_float_nonpositive() {
    let mut rng = Rng::new(SEED ^ 26);
    same("err26-zero", 0, 0, 0, 0);
    same("err26-negzero", 0x8000_0000u32 as i32, 1, 2, 3);
    for _ in 0..2000 {
        // any negative int has the float sign bit set → f < 0 → branch skipped
        let a = (rng.next_u32() | 0x8000_0000) as i32;
        same("err26-neg", a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 27 — `f >= 1000.0f` (incl. +inf): float branch rejected.
#[test]
fn err27_memchra2_float_too_big() {
    let mut rng = Rng::new(SEED ^ 27);
    for a in [0x447A_0000u32, 0x447A_0001, 0x7F7F_FFFF, 0x7F80_0000] {
        same("err27-fixed", a as i32, -1, 0, 1);
    }
    let lo = 0x447A_0000u32;
    let hi = 0x7F80_0000u32;
    for _ in 0..2000 {
        let a = (lo + rng.next_u32() % (hi - lo + 1)) as i32;
        same("err27-rand", a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 28 — `f` is NaN: both comparisons are false, branch rejected.
#[test]
fn err28_memchra2_float_nan() {
    let nans: [u32; 6] = [
        0x7FC0_0000,
        0x7FFF_FFFF,
        0x7F80_0001,
        0xFFC0_0000,
        0xFFFF_FFFF,
        0xFF80_0001,
    ];
    let mut rng = Rng::new(SEED ^ 28);
    for &a in &nans {
        for _ in 0..200 {
            same(
                "err28",
                a as i32,
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
            );
        }
    }
}

/// Row 30 — extreme/boundary arguments (longest `%d` expansion, no truncation).
#[test]
fn err30_memchra2_extreme_args() {
    let vals = [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1];
    for &v in &vals {
        same("err30-a", v, 0, 0, 0);
        same("err30-b", 0, v, 0, 0);
        same("err30-c", 0, 0, v, 0);
        same("err30-d", 0, 0, 0, v);
        same("err30-all", v, v, v, v);
    }
    // all-INT_MIN → 51-byte formatted buffer, the longest possible
    same("err30-longest", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
}

/// Row 31 — there is no argument validation at all: every 32-bit pattern is a
/// valid input (the analogue of "out-of-range enum value" for this API), and the
/// function never rejects.  Both implementations must agree on all of them.
#[test]
fn err31_memchra2_no_rejection() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..10_000 {
        // deliberately nonsensical / "out of range" bit patterns
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        same("err31", a, b, c, d);
    }
    // values one step past every documented-looking boundary
    for &v in &[
        i32::MIN,
        i32::MIN + 1,
        -256,
        -255,
        -129,
        -128,
        -1,
        0,
        1,
        127,
        128,
        255,
        256,
        0x3F7F_FFFF,
        0x3F80_0000,
        0x4479_FFFF,
        0x447A_0000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        same("err31-boundary", v, v, v, v);
        same("err31-boundary-a", v, 1, 2, 3);
    }
}

// ---------------------------------------------------------------------------
// Static helpers (feature `internal_test_api`)
// ---------------------------------------------------------------------------

#[cfg(feature = "internal_test_api")]
mod helpers {
    use std::ffi::{c_char, c_int, c_uchar};
    use std::ptr;

    use super::common::{c_shim, pair, Rng, SEED};

    type FMemchra = unsafe extern "C" fn(*const c_char, c_int, usize) -> c_int;
    type FProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
    type FProcessStrings = unsafe extern "C" fn(*mut *mut c_char, c_int, *const c_char) -> c_int;
    type FSafeSum = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
    type FInterpret = unsafe extern "C" fn(*mut c_uchar, usize) -> c_int;
    type FCount = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
    type FComplex = unsafe extern "C" fn(*mut c_int, usize) -> c_int;

    fn to_cchar(bytes: &[u8]) -> Vec<c_char> {
        bytes.iter().map(|&b| b as c_char).collect()
    }

    /// Row 1 — `process_buffer(NULL, len)` → -1 for every `len`.
    #[test]
    fn err01_process_buffer_null() {
        let (fc, fr) = unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") };
        for len in [0usize, 1, 4, 64, usize::MAX] {
            let rc = unsafe { fc(ptr::null_mut(), len) };
            let rr = unsafe { fr(ptr::null_mut(), len) };
            assert_eq!(rc, -1, "C must reject NULL buffer with -1 (len={len})");
            assert_eq!(rc, rr, "process_buffer(NULL, {len}) C={rc} Rust={rr}");
        }
    }

    /// Row 2 — `*buffer == '\0'` → -1.
    #[test]
    fn err02_process_buffer_empty() {
        let (fc, fr) = unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") };
        let mut empty: Vec<c_char> = vec![0, b'a' as c_char, b'b' as c_char, 0];
        for len in [0usize, 1, 2, 4] {
            let rc = unsafe { fc(empty.as_mut_ptr(), len) };
            let rr = unsafe { fr(empty.as_mut_ptr(), len) };
            assert_eq!(rc, -1, "C must reject an empty string with -1 (len={len})");
            assert_eq!(rc, rr, "process_buffer(\"\", {len}) C={rc} Rust={rr}");
        }
    }

    /// Row 3 — `len == 0` with a non-empty buffer → 0 (guard passes, loop skipped).
    #[test]
    fn err03_process_buffer_zero_len() {
        let (fc, fr) = unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") };
        let mut buf = to_cchar(b"abc\0");
        let rc = unsafe { fc(buf.as_mut_ptr(), 0) };
        let rr = unsafe { fr(buf.as_mut_ptr(), 0) };
        assert_eq!(rc, 0, "C returns 0 for len == 0");
        assert_eq!(rc, rr, "process_buffer(\"abc\", 0) C={rc} Rust={rr}");
    }

    /// Row 4 — interior NUL stops the loop early.
    #[test]
    fn err04_process_buffer_interior_nul() {
        let (fc, fr) = unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") };
        let mut buf = to_cchar(b"ab\0cd\0");
        for len in 0..=buf.len() {
            let rc = unsafe { fc(buf.as_mut_ptr(), len) };
            let rr = unsafe { fr(buf.as_mut_ptr(), len) };
            assert_eq!(rc, rr, "process_buffer(\"ab\\0cd\", {len}) C={rc} Rust={rr}");
        }
        // 'a' + 'b' == 97 + 98
        let rc = unsafe { fc(buf.as_mut_ptr(), buf.len()) };
        assert_eq!(rc, 97 + 98, "loop must stop at the interior NUL");
    }

    /// Row 5 — `process_strings(NULL, count, target)` → 0.
    #[test]
    fn err05_process_strings_null_array() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let target = to_cchar(b"test\0");
        for count in [-1i32, 0, 1, 4, i32::MAX] {
            let rc = unsafe { fc(ptr::null_mut(), count, target.as_ptr()) };
            let rr = unsafe { fr(ptr::null_mut(), count, target.as_ptr()) };
            assert_eq!(rc, 0, "C must reject a NULL array with 0 (count={count})");
            assert_eq!(rc, rr, "process_strings(NULL, {count}) C={rc} Rust={rr}");
        }
    }

    /// Row 6 — `count == 0` → 0.
    #[test]
    fn err06_process_strings_count_zero() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let s = to_cchar(b"test1\0");
        let mut ptrs: Vec<*mut c_char> = vec![s.as_ptr() as *mut c_char];
        let target = to_cchar(b"test\0");
        let rc = unsafe { fc(ptrs.as_mut_ptr(), 0, target.as_ptr()) };
        let rr = unsafe { fr(ptrs.as_mut_ptr(), 0, target.as_ptr()) };
        assert_eq!(rc, 0, "C returns 0 for count == 0");
        assert_eq!(rc, rr, "process_strings(count=0) C={rc} Rust={rr}");
    }

    /// Row 7 — negative `count` → 0.
    #[test]
    fn err07_process_strings_count_negative() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let s = to_cchar(b"test1\0");
        let mut ptrs: Vec<*mut c_char> = vec![s.as_ptr() as *mut c_char];
        let target = to_cchar(b"test\0");
        for count in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
            let rc = unsafe { fc(ptrs.as_mut_ptr(), count, target.as_ptr()) };
            let rr = unsafe { fr(ptrs.as_mut_ptr(), count, target.as_ptr()) };
            assert_eq!(rc, 0, "C returns 0 for count={count}");
            assert_eq!(rc, rr, "process_strings(count={count}) C={rc} Rust={rr}");
        }
    }

    /// Row 8 — a NULL element is skipped, the rest still counted.
    #[test]
    fn err08_process_strings_null_element() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let a = to_cchar(b"test1\0");
        let b = to_cchar(b"test2\0");
        let mut ptrs: Vec<*mut c_char> = vec![
            ptr::null_mut(),
            a.as_ptr() as *mut c_char,
            ptr::null_mut(),
            b.as_ptr() as *mut c_char,
        ];
        let target = to_cchar(b"test\0");
        let rc = unsafe { fc(ptrs.as_mut_ptr(), 4, target.as_ptr()) };
        let rr = unsafe { fr(ptrs.as_mut_ptr(), 4, target.as_ptr()) };
        assert_eq!(rc, 2, "the two NULL elements must be skipped, 2 matches remain");
        assert_eq!(rc, rr, "process_strings(with NULL elements) C={rc} Rust={rr}");
    }

    /// Row 9 — an empty-string element is skipped.
    #[test]
    fn err09_process_strings_empty_element() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let empty = to_cchar(b"\0");
        let a = to_cchar(b"test1\0");
        let mut ptrs: Vec<*mut c_char> = vec![
            empty.as_ptr() as *mut c_char,
            a.as_ptr() as *mut c_char,
            empty.as_ptr() as *mut c_char,
        ];
        // even with an empty target (which matches everything) the empty element
        // is skipped by the `**i == '\0'` guard
        for target in [&b"test\0"[..], &b"\0"[..]] {
            let t = to_cchar(target);
            let rc = unsafe { fc(ptrs.as_mut_ptr(), 3, t.as_ptr()) };
            let rr = unsafe { fr(ptrs.as_mut_ptr(), 3, t.as_ptr()) };
            assert_eq!(rc, 1, "only the non-empty element may be counted");
            assert_eq!(rc, rr, "process_strings(with empty elements) C={rc} Rust={rr}");
        }
    }

    /// Row 10 — empty target: `strncmp(..., 0) == 0`, so every live element matches.
    #[test]
    fn err10_process_strings_empty_target() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let a = to_cchar(b"zzz\0");
        let b = to_cchar(b"qqq\0");
        let mut ptrs: Vec<*mut c_char> =
            vec![a.as_ptr() as *mut c_char, b.as_ptr() as *mut c_char];
        let t = to_cchar(b"\0");
        let rc = unsafe { fc(ptrs.as_mut_ptr(), 2, t.as_ptr()) };
        let rr = unsafe { fr(ptrs.as_mut_ptr(), 2, t.as_ptr()) };
        assert_eq!(rc, 2, "an empty target matches every live element");
        assert_eq!(rc, rr, "process_strings(empty target) C={rc} Rust={rr}");
    }

    /// Row 11 — element shorter than target: no match.
    #[test]
    fn err11_process_strings_short_element() {
        let (fc, fr) = unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") };
        let a = to_cchar(b"te\0");
        let b = to_cchar(b"t\0");
        let c = to_cchar(b"test\0");
        let mut ptrs: Vec<*mut c_char> = vec![
            a.as_ptr() as *mut c_char,
            b.as_ptr() as *mut c_char,
            c.as_ptr() as *mut c_char,
        ];
        let t = to_cchar(b"test\0");
        let rc = unsafe { fc(ptrs.as_mut_ptr(), 3, t.as_ptr()) };
        let rr = unsafe { fr(ptrs.as_mut_ptr(), 3, t.as_ptr()) };
        assert_eq!(rc, 1, "only the full-length element matches");
        assert_eq!(rc, rr, "process_strings(short elements) C={rc} Rust={rr}");
    }

    /// Row 12 — `safe_sum_array(NULL, size)` → 0.
    #[test]
    fn err12_safe_sum_null() {
        let (fc, fr) = unsafe { pair::<FSafeSum>(c_shim(), "itest_safe_sum_array") };
        for size in [0usize, 1, 4, 1024, usize::MAX] {
            let rc = unsafe { fc(ptr::null_mut(), size) };
            let rr = unsafe { fr(ptr::null_mut(), size) };
            assert_eq!(rc, 0, "C must reject a NULL array with 0 (size={size})");
            assert_eq!(rc, rr, "safe_sum_array(NULL, {size}) C={rc} Rust={rr}");
        }
    }

    /// Row 13 — `size == 0` → 0.
    #[test]
    fn err13_safe_sum_zero_size() {
        let (fc, fr) = unsafe { pair::<FSafeSum>(c_shim(), "itest_safe_sum_array") };
        let mut arr: Vec<c_int> = vec![1, 2, 3, 4];
        let rc = unsafe { fc(arr.as_mut_ptr(), 0) };
        let rr = unsafe { fr(arr.as_mut_ptr(), 0) };
        assert_eq!(rc, 0, "C returns 0 for size == 0");
        assert_eq!(rc, rr, "safe_sum_array(size=0) C={rc} Rust={rr}");
    }

    /// Row 14 — `interpret_as_int(NULL, len)` → 0.
    #[test]
    fn err14_interpret_null() {
        let (fc, fr) = unsafe { pair::<FInterpret>(c_shim(), "itest_interpret_as_int") };
        for len in [0usize, 1, 3, 4, 8, usize::MAX] {
            let rc = unsafe { fc(ptr::null_mut(), len) };
            let rr = unsafe { fr(ptr::null_mut(), len) };
            assert_eq!(rc, 0, "C must reject NULL bytes with 0 (len={len})");
            assert_eq!(rc, rr, "interpret_as_int(NULL, {len}) C={rc} Rust={rr}");
        }
    }

    /// Row 15 — `len < sizeof(int)` → 0.
    #[test]
    fn err15_interpret_short_len() {
        let (fc, fr) = unsafe { pair::<FInterpret>(c_shim(), "itest_interpret_as_int") };
        let mut bytes: Vec<c_uchar> = vec![0xAA, 0xBB, 0xCC, 0xDD];
        for len in 0..4usize {
            let rc = unsafe { fc(bytes.as_mut_ptr(), len) };
            let rr = unsafe { fr(bytes.as_mut_ptr(), len) };
            assert_eq!(rc, 0, "C returns 0 for len={len} < sizeof(int)");
            assert_eq!(rc, rr, "interpret_as_int(len={len}) C={rc} Rust={rr}");
        }
    }

    /// Row 16 — `len == sizeof(int)`: one step inside the valid range.
    #[test]
    fn err16_interpret_len_boundary() {
        let (fc, fr) = unsafe { pair::<FInterpret>(c_shim(), "itest_interpret_as_int") };
        let mut bytes: Vec<c_uchar> = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let rc = unsafe { fc(bytes.as_mut_ptr(), 4) };
        let rr = unsafe { fr(bytes.as_mut_ptr(), 4) };
        assert_ne!(rc, 0, "len == 4 must not be rejected");
        assert_eq!(rc, rr, "interpret_as_int(len=4) C={rc} Rust={rr}");
    }

    /// Row 17 — `count_occurrences(NULL, ch)` → 0.
    #[test]
    fn err17_count_null() {
        let (fc, fr) = unsafe { pair::<FCount>(c_shim(), "itest_count_occurrences") };
        for ch in [0i8, b'-' as i8, -1, 127, -128] {
            let rc = unsafe { fc(ptr::null(), ch) };
            let rr = unsafe { fr(ptr::null(), ch) };
            assert_eq!(rc, 0, "C must reject NULL text with 0 (ch={ch})");
            assert_eq!(rc, rr, "count_occurrences(NULL, {ch}) C={rc} Rust={rr}");
        }
    }

    /// Row 18 — empty text → 0.
    #[test]
    fn err18_count_empty() {
        let (fc, fr) = unsafe { pair::<FCount>(c_shim(), "itest_count_occurrences") };
        let text = to_cchar(b"\0");
        for ch in [0i8, b'-' as i8, -1, 127, -128] {
            let rc = unsafe { fc(text.as_ptr(), ch) };
            let rr = unsafe { fr(text.as_ptr(), ch) };
            assert_eq!(rc, 0, "C returns 0 for an empty string (ch={ch})");
            assert_eq!(rc, rr, "count_occurrences(\"\", {ch}) C={rc} Rust={rr}");
        }
    }

    /// Row 19 — needle is the NUL terminator: never found inside [0, strlen).
    #[test]
    fn err19_count_nul_needle() {
        let (fc, fr) = unsafe { pair::<FCount>(c_shim(), "itest_count_occurrences") };
        for text in [&b"abc\0"[..], &b"a\0"[..], &b"----\0"[..]] {
            let t = to_cchar(text);
            let rc = unsafe { fc(t.as_ptr(), 0) };
            let rr = unsafe { fr(t.as_ptr(), 0) };
            assert_eq!(rc, 0, "a NUL needle is never counted");
            assert_eq!(rc, rr, "count_occurrences(text, 0) C={rc} Rust={rr}");
        }
    }

    /// Row 20 — `complex_iteration(NULL, count)` → -1.
    #[test]
    fn err20_complex_null() {
        let (fc, fr) = unsafe { pair::<FComplex>(c_shim(), "itest_complex_iteration") };
        for count in [0usize, 1, 4, 1024, usize::MAX] {
            let rc = unsafe { fc(ptr::null_mut(), count) };
            let rr = unsafe { fr(ptr::null_mut(), count) };
            assert_eq!(rc, -1, "C must reject NULL data with -1 (count={count})");
            assert_eq!(rc, rr, "complex_iteration(NULL, {count}) C={rc} Rust={rr}");
        }
    }

    /// Row 21 — `count == 0` → -1.
    #[test]
    fn err21_complex_zero_count() {
        let (fc, fr) = unsafe { pair::<FComplex>(c_shim(), "itest_complex_iteration") };
        let mut data: Vec<c_int> = vec![1, 2, 3, 4];
        let rc = unsafe { fc(data.as_mut_ptr(), 0) };
        let rr = unsafe { fr(data.as_mut_ptr(), 0) };
        assert_eq!(rc, -1, "C returns -1 for count == 0");
        assert_eq!(rc, rr, "complex_iteration(count=0) C={rc} Rust={rr}");
    }

    /// Row 22 — the `-1` sentinel is unambiguous: valid inputs always yield a
    /// value in [0, 255].
    #[test]
    fn err22_complex_result_range() {
        let (fc, fr) = unsafe { pair::<FComplex>(c_shim(), "itest_complex_iteration") };
        let mut rng = Rng::new(SEED ^ 122);
        for _ in 0..2000 {
            let n = 1 + rng.below(16);
            let mut data_c: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
            let mut data_r = data_c.clone();
            let rc = unsafe { fc(data_c.as_mut_ptr(), n) };
            let rr = unsafe { fr(data_r.as_mut_ptr(), n) };
            assert!((0..=255).contains(&rc), "C result {rc} out of [0,255]");
            assert_eq!(rc, rr, "complex_iteration C={rc} Rust={rr}");
        }
    }

    /// Row 23 — `memchra(str, c, 0)` → 0 (loop never runs).
    #[test]
    fn err23_memchra_zero_n() {
        let (fc, fr) = unsafe { pair::<FMemchra>(c_shim(), "itest_memchra") };
        let s = to_cchar(b"aaaa\0");
        for c in [b'a' as c_int, 0, -1, i32::MAX, i32::MIN] {
            let rc = unsafe { fc(s.as_ptr(), c, 0) };
            let rr = unsafe { fr(s.as_ptr(), c, 0) };
            assert_eq!(rc, 0, "C returns 0 for n == 0 (c={c})");
            assert_eq!(rc, rr, "memchra(.., {c}, 0) C={rc} Rust={rr}");
        }
        // NULL str with n == 0 is never dereferenced by the C either
        for c in [b'a' as c_int, 0] {
            let rc = unsafe { fc(ptr::null(), c, 0) };
            let rr = unsafe { fr(ptr::null(), c, 0) };
            assert_eq!(rc, 0);
            assert_eq!(rc, rr, "memchra(NULL, {c}, 0) C={rc} Rust={rr}");
        }
    }

    /// Row 24 — needle absent → 0.
    #[test]
    fn err24_memchra_absent() {
        let (fc, fr) = unsafe { pair::<FMemchra>(c_shim(), "itest_memchra") };
        let s = to_cchar(b"abcdef");
        for c in [b'z' as c_int, b'A' as c_int, 0x80, -100] {
            let rc = unsafe { fc(s.as_ptr(), c, 6) };
            let rr = unsafe { fr(s.as_ptr(), c, 6) };
            assert_eq!(rc, 0, "needle {c} is absent → 0");
            assert_eq!(rc, rr, "memchra(.., {c}, 6) C={rc} Rust={rr}");
        }
    }

    /// Row 25 — `c` outside the `char` range: `(char)c` narrows, so `0x141`
    /// matches `'A'` and `-1` matches `0xFF`.
    #[test]
    fn err25_memchra_out_of_char_range() {
        let (fc, fr) = unsafe { pair::<FMemchra>(c_shim(), "itest_memchra") };
        let raw: [u8; 6] = [b'A', 0x41, 0xFF, 0x00, 0x80, b'B'];
        let s: Vec<c_char> = raw.iter().map(|&b| b as c_char).collect();
        let cases: [c_int; 10] = [
            0x141,     // (char)0x141 == 'A'
            0x1FF,     // (char)0x1FF == (char)0xFF
            -1,        // (char)-1 == 0xFF
            0x100,     // (char)0x100 == '\0'
            i32::MIN,  // (char)INT_MIN == '\0'
            i32::MAX,  // (char)INT_MAX == (char)0xFF
            0x80,      // (char)0x80 == -128
            -128,      // 0x80 byte
            0x42,      // 'B'
            0x1_0041,  // (char)0x10041 == 'A'
        ];
        for c in cases {
            let rc = unsafe { fc(s.as_ptr(), c, raw.len()) };
            let rr = unsafe { fr(s.as_ptr(), c, raw.len()) };
            assert_eq!(rc, rr, "memchra(.., {c:#x}, {}) C={rc} Rust={rr}", raw.len());
        }
        // spot-check the documented truncation semantics against the C itself
        let rc = unsafe { fc(s.as_ptr(), 0x141, raw.len()) };
        assert_eq!(rc, 2, "(char)0x141 must behave like 'A'");
        let rc = unsafe { fc(s.as_ptr(), 0x100, raw.len()) };
        assert_eq!(rc, 1, "(char)0x100 must behave like '\\0'");
    }

    /// Row 29 — `memchra2`'s `buf_sum > 0` guard: the formatted buffer is always
    /// printable ASCII, so `process_buffer` never returns a non-positive value
    /// for any argument combination, in either implementation.
    #[test]
    fn err29_memchra2_bufsum_positive() {
        let (fc, fr) = unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") };
        let mut rng = Rng::new(SEED ^ 129);
        let check = |a: c_int, b: c_int, c: c_int, d: c_int| {
            let s = format!("test{a}-{b}-{c}-{d}");
            let mut buf_c = to_cchar(s.as_bytes());
            buf_c.push(0);
            let mut buf_r = buf_c.clone();
            let len = s.len();
            let rc = unsafe { fc(buf_c.as_mut_ptr(), len) };
            let rr = unsafe { fr(buf_r.as_mut_ptr(), len) };
            assert!(rc > 0, "buf_sum must be > 0 for {s:?}, got {rc}");
            assert_eq!(rc, rr, "process_buffer({s:?}) C={rc} Rust={rr}");
        };
        check(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
        check(0, 0, 0, 0);
        for _ in 0..2000 {
            check(
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            );
        }
    }
}
