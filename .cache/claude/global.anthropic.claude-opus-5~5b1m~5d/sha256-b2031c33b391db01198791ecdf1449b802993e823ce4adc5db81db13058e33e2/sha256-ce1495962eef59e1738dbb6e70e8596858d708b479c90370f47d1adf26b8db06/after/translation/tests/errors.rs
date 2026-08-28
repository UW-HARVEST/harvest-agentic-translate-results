//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. Rows that concern the `static` helpers of
//! `lib.c` are driven through the `harness_*` C-ABI wrappers: the C side comes
//! from `tests/c_harness/harness.c` (which textually `#include`s the unmodified
//! `c_src/src/lib.c`), the Rust side from the `test_internals`-gated exports of
//! the Rust `cdylib`. Both are reached with `libloading` + `dlsym` only.
//!
//! Run with:  `cargo test --features test_internals`

mod common;

use common::{assert_same, Rng};
use std::ffi::c_char;

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

#[track_caller]
#[allow(dead_code)] // only used by the `test_internals` rows
fn same<T: PartialEq + std::fmt::Debug>(c: T, r: T, what: &str) -> T {
    assert_eq!(c, r, "C/Rust divergence on {what}");
    c
}

// ===========================================================================
// Rows driven through the internals harness
// ===========================================================================
#[cfg(feature = "test_internals")]
mod internals_rows {
    use super::*;
    use common::internals;
    use std::ffi::c_int;

    /// Row 1 — `process_buffer(NULL, len)` → -1
    #[test]
    fn err_row01_process_buffer_null() {
        let (cf, rf) = internals().process_buffer;
        for len in [0usize, 1, 3, 4, 64, 1 << 16] {
            let cv = unsafe { cf(std::ptr::null_mut(), len) };
            let rv = unsafe { rf(std::ptr::null_mut(), len) };
            same(cv, rv, &format!("process_buffer(NULL, {len})"));
            assert_eq!(cv, -1, "C sentinel for NULL buffer must be -1");
        }
    }

    /// Row 2 — `process_buffer(ptr_to_empty_string, len)` → -1
    #[test]
    fn err_row02_process_buffer_empty() {
        let (cf, rf) = internals().process_buffer;
        for len in [0usize, 1, 2, 8] {
            let mut buf: [c_char; 16] = [0; 16]; // "" (first byte NUL)
            let cv = unsafe { cf(buf.as_mut_ptr(), len) };
            let rv = unsafe { rf(buf.as_mut_ptr(), len) };
            same(cv, rv, &format!("process_buffer(\"\", {len})"));
            assert_eq!(cv, -1, "C sentinel for empty buffer must be -1");
        }
    }

    /// Row 3 — `len == 0` with a valid non-empty buffer → 0
    #[test]
    fn err_row03_process_buffer_zero_len() {
        let (cf, rf) = internals().process_buffer;
        let mut buf: Vec<c_char> = b"abc\0".iter().map(|&b| b as c_char).collect();
        let cv = unsafe { cf(buf.as_mut_ptr(), 0) };
        let rv = unsafe { rf(buf.as_mut_ptr(), 0) };
        same(cv, rv, "process_buffer(\"abc\", 0)");
        assert_eq!(cv, 0, "zero-length scan must contribute nothing");
    }

    /// Row 4 — embedded NUL stops the loop early; also oversized `len`
    #[test]
    fn err_row04_process_buffer_embedded_nul() {
        let (cf, rf) = internals().process_buffer;
        let mut buf: Vec<c_char> = b"ab\0cd\0\0\0"
            .iter()
            .map(|&b| b as c_char)
            .chain(std::iter::repeat(0).take(4096))
            .collect();
        for len in [1usize, 2, 3, 5, 8, 4096] {
            let cv = unsafe { cf(buf.as_mut_ptr(), len) };
            let rv = unsafe { rf(buf.as_mut_ptr(), len) };
            same(cv, rv, &format!("process_buffer(\"ab\\0cd\", {len})"));
        }
        // 'a'(97) + 'b'(98) = 195 regardless of oversized len
        let cv = unsafe { cf(buf.as_mut_ptr(), 4096) };
        assert_eq!(cv, 195);
    }

    /// Row 5 — the `if (buf_sum > 0)` guard: construct `buf_sum <= 0`
    #[test]
    fn err_row05_bufsum_guard() {
        let (csn, rsn) = internals().snprintf_fmt;
        let (cpb, rpb) = internals().process_buffer;
        // snprintf with size 1 writes only the NUL -> empty buffer
        let mut cbuf: [c_char; 64] = [0x7F; 64];
        let mut rbuf: [c_char; 64] = [0x7F; 64];
        let cn = unsafe { csn(cbuf.as_mut_ptr(), 1, 1, 2, 3, 4) };
        let rn = unsafe { rsn(rbuf.as_mut_ptr(), 1, 1, 2, 3, 4) };
        same(cn, rn, "snprintf_fmt(size=1) return");
        assert_eq!(cbuf[0], 0, "size-1 snprintf must NUL-terminate");
        let cv = unsafe { cpb(cbuf.as_mut_ptr(), 0) };
        let rv = unsafe { rpb(rbuf.as_mut_ptr(), 0) };
        same(cv, rv, "process_buffer on empty snprintf output");
        assert!(cv <= 0, "buf_sum must be <= 0 so the guard skips it");
    }

    /// Row 6 — `process_strings(NULL, count, target)` → 0
    #[test]
    fn err_row06_process_strings_null() {
        let (cf, rf) = internals().process_strings;
        let target = b"test\0";
        for count in [INT_MIN, -1, 0, 1, 4, INT_MAX] {
            let cv = unsafe {
                cf(
                    std::ptr::null(),
                    count,
                    target.as_ptr() as *const c_char,
                )
            };
            let rv = unsafe {
                rf(
                    std::ptr::null(),
                    count,
                    target.as_ptr() as *const c_char,
                )
            };
            same(cv, rv, &format!("process_strings(NULL, {count})"));
            assert_eq!(cv, 0, "C sentinel for NULL array must be 0");
        }
    }

    /// Row 7 — `count <= 0` → 0 (array untouched)
    #[test]
    fn err_row07_process_strings_nonpositive_count() {
        let (cf, rf) = internals().process_strings;
        let s0 = b"test1\0";
        let s1 = b"test2\0";
        let arr: [*const c_char; 2] = [s0.as_ptr() as *const c_char, s1.as_ptr() as *const c_char];
        let target = b"test\0";
        for count in [INT_MIN, INT_MIN + 1, -2, -1, 0] {
            let cv =
                unsafe { cf(arr.as_ptr(), count, target.as_ptr() as *const c_char) };
            let rv =
                unsafe { rf(arr.as_ptr(), count, target.as_ptr() as *const c_char) };
            same(cv, rv, &format!("process_strings(arr, {count})"));
            assert_eq!(cv, 0, "non-positive count must yield 0");
        }
    }

    /// Row 8 — NULL element is skipped
    #[test]
    fn err_row08_process_strings_null_element() {
        let (cf, rf) = internals().process_strings;
        let a = b"test1\0";
        let b = b"testing\0";
        let target = b"test\0";
        let arr: [*const c_char; 4] = [
            a.as_ptr() as *const c_char,
            std::ptr::null(),
            b.as_ptr() as *const c_char,
            std::ptr::null(),
        ];
        let cv = unsafe { cf(arr.as_ptr(), 4, target.as_ptr() as *const c_char) };
        let rv = unsafe { rf(arr.as_ptr(), 4, target.as_ptr() as *const c_char) };
        same(cv, rv, "process_strings with NULL elements");
        assert_eq!(cv, 2, "only the two non-NULL matching entries count");
        // all-NULL array
        let all_null: [*const c_char; 3] = [std::ptr::null(); 3];
        let cv = unsafe { cf(all_null.as_ptr(), 3, target.as_ptr() as *const c_char) };
        let rv = unsafe { rf(all_null.as_ptr(), 3, target.as_ptr() as *const c_char) };
        same(cv, rv, "process_strings all-NULL");
        assert_eq!(cv, 0);
    }

    /// Row 9 — empty-string element is skipped
    #[test]
    fn err_row09_process_strings_empty_element() {
        let (cf, rf) = internals().process_strings;
        let empty = b"\0";
        let hit = b"test9\0";
        let target = b"test\0";
        let arr: [*const c_char; 3] = [
            empty.as_ptr() as *const c_char,
            hit.as_ptr() as *const c_char,
            empty.as_ptr() as *const c_char,
        ];
        let cv = unsafe { cf(arr.as_ptr(), 3, target.as_ptr() as *const c_char) };
        let rv = unsafe { rf(arr.as_ptr(), 3, target.as_ptr() as *const c_char) };
        same(cv, rv, "process_strings with empty elements");
        assert_eq!(cv, 1);
    }

    /// Row 10 — `strncmp` mismatch is not counted
    #[test]
    fn err_row10_process_strings_mismatch() {
        let (cf, rf) = internals().process_strings;
        let target = b"test\0";
        let candidates: [&[u8]; 8] = [
            b"other\0", b"tes\0", b"te\0", b"t\0", b"Test\0", b"tesT\0", b"ztest\0", b"testX\0",
        ];
        let ptrs: Vec<*const c_char> =
            candidates.iter().map(|s| s.as_ptr() as *const c_char).collect();
        let cv = unsafe { cf(ptrs.as_ptr(), ptrs.len() as c_int, target.as_ptr() as *const c_char) };
        let rv = unsafe { rf(ptrs.as_ptr(), ptrs.len() as c_int, target.as_ptr() as *const c_char) };
        same(cv, rv, "process_strings mismatches");
        assert_eq!(cv, 1, "only \"testX\" starts with \"test\"");
        // each candidate on its own
        for s in candidates {
            let one: [*const c_char; 1] = [s.as_ptr() as *const c_char];
            let cv = unsafe { cf(one.as_ptr(), 1, target.as_ptr() as *const c_char) };
            let rv = unsafe { rf(one.as_ptr(), 1, target.as_ptr() as *const c_char) };
            same(cv, rv, "process_strings single candidate");
        }
    }

    /// Row 11 — empty target: `strncmp(x, "", 0) == 0` matches everything non-empty
    #[test]
    fn err_row11_process_strings_empty_target() {
        let (cf, rf) = internals().process_strings;
        let target = b"\0";
        let a = b"anything\0";
        let b = b"\0";
        let arr: [*const c_char; 4] = [
            a.as_ptr() as *const c_char,
            b.as_ptr() as *const c_char,
            std::ptr::null(),
            a.as_ptr() as *const c_char,
        ];
        let cv = unsafe { cf(arr.as_ptr(), 4, target.as_ptr() as *const c_char) };
        let rv = unsafe { rf(arr.as_ptr(), 4, target.as_ptr() as *const c_char) };
        same(cv, rv, "process_strings empty target");
        assert_eq!(cv, 2, "the two non-empty, non-NULL entries match");
    }

    /// Row 12 — `safe_sum_array(NULL, size)` → 0
    #[test]
    fn err_row12_safe_sum_null() {
        let (cf, rf) = internals().safe_sum_array;
        for size in [0usize, 1, 4, 1 << 20] {
            let cv = unsafe { cf(std::ptr::null(), size) };
            let rv = unsafe { rf(std::ptr::null(), size) };
            same(cv, rv, &format!("safe_sum_array(NULL, {size})"));
            assert_eq!(cv, 0);
        }
    }

    /// Row 13 — `size == 0` → 0
    #[test]
    fn err_row13_safe_sum_zero_size() {
        let (cf, rf) = internals().safe_sum_array;
        let arr: [c_int; 4] = [1, 2, 3, 4];
        let cv = unsafe { cf(arr.as_ptr(), 0) };
        let rv = unsafe { rf(arr.as_ptr(), 0) };
        same(cv, rv, "safe_sum_array(arr, 0)");
        assert_eq!(cv, 0);
    }

    /// Row 14 — signed overflow wraps identically
    #[test]
    fn err_row14_safe_sum_overflow() {
        let (cf, rf) = internals().safe_sum_array;
        let mut rng = Rng::new(0x14);
        let cases: Vec<Vec<c_int>> = vec![
            vec![INT_MAX, 1],
            vec![INT_MAX, INT_MAX],
            vec![INT_MIN, -1],
            vec![INT_MIN, INT_MIN],
            vec![INT_MAX, INT_MAX, INT_MAX, INT_MAX],
            vec![INT_MIN, INT_MIN, INT_MIN, INT_MIN],
        ];
        for case in &cases {
            let cv = unsafe { cf(case.as_ptr(), case.len()) };
            let rv = unsafe { rf(case.as_ptr(), case.len()) };
            same(cv, rv, &format!("safe_sum_array overflow {case:?}"));
        }
        for _ in 0..2000 {
            let n = rng.range_i64(1, 16) as usize;
            let v: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
            let cv = unsafe { cf(v.as_ptr(), n) };
            let rv = unsafe { rf(v.as_ptr(), n) };
            same(cv, rv, "safe_sum_array random");
        }
    }

    /// Row 15 — `interpret_as_int(NULL, len)` → 0
    #[test]
    fn err_row15_interpret_null() {
        let (cf, rf) = internals().interpret_as_int;
        for len in [0usize, 1, 3, 4, 8, 1 << 20] {
            let cv = unsafe { cf(std::ptr::null(), len) };
            let rv = unsafe { rf(std::ptr::null(), len) };
            same(cv, rv, &format!("interpret_as_int(NULL, {len})"));
            assert_eq!(cv, 0);
        }
    }

    /// Row 16 — `len < sizeof(int)` → 0; `len >= 4` reads 4 little-endian bytes
    #[test]
    fn err_row16_interpret_short_len() {
        let (cf, rf) = internals().interpret_as_int;
        let buf: Vec<u8> = (0u8..64).collect();
        for len in [0usize, 1, 2, 3] {
            let cv = unsafe { cf(buf.as_ptr(), len) };
            let rv = unsafe { rf(buf.as_ptr(), len) };
            same(cv, rv, &format!("interpret_as_int(buf, {len})"));
            assert_eq!(cv, 0, "len < sizeof(int) must return 0");
        }
        for len in [4usize, 5, 8, 64] {
            let cv = unsafe { cf(buf.as_ptr(), len) };
            let rv = unsafe { rf(buf.as_ptr(), len) };
            same(cv, rv, &format!("interpret_as_int(buf, {len})"));
            assert_eq!(cv, 0x03020100);
        }
        // randomized byte patterns
        let mut rng = Rng::new(0x16);
        for _ in 0..2000 {
            let bytes: [u8; 8] = std::array::from_fn(|_| (rng.next_u32() & 0xFF) as u8);
            for len in [3usize, 4, 5, 8] {
                let cv = unsafe { cf(bytes.as_ptr(), len) };
                let rv = unsafe { rf(bytes.as_ptr(), len) };
                same(cv, rv, "interpret_as_int random");
            }
        }
    }

    /// Row 17 — `count_occurrences(NULL, ch)` → 0
    #[test]
    fn err_row17_count_occ_null() {
        let (cf, rf) = internals().count_occurrences;
        for ch in [0i32, 1, 45, 127, -1, -128] {
            let cv = unsafe { cf(std::ptr::null(), ch as c_char) };
            let rv = unsafe { rf(std::ptr::null(), ch as c_char) };
            same(cv, rv, &format!("count_occurrences(NULL, {ch})"));
            assert_eq!(cv, 0);
        }
    }

    /// Row 18 — empty text → 0 (NOT -1)
    #[test]
    fn err_row18_count_occ_empty() {
        let (cf, rf) = internals().count_occurrences;
        let empty = b"\0";
        for ch in [0i32, 45, 65, -1] {
            let cv = unsafe { cf(empty.as_ptr() as *const c_char, ch as c_char) };
            let rv = unsafe { rf(empty.as_ptr() as *const c_char, ch as c_char) };
            same(cv, rv, &format!("count_occurrences(\"\", {ch})"));
            assert_eq!(cv, 0, "empty string sentinel is 0, not -1");
        }
        // non-empty sanity + randomized text
        let mut rng = Rng::new(0x18);
        for _ in 0..500 {
            let n = rng.range_i64(1, 40) as usize;
            let mut s: Vec<u8> = (0..n)
                .map(|_| {
                    let v = (rng.next_u32() % 96) as u8 + 32; // printable, never NUL
                    v
                })
                .collect();
            s.push(0);
            for ch in [45i32, 32, 122, 65] {
                let cv = unsafe { cf(s.as_ptr() as *const c_char, ch as c_char) };
                let rv = unsafe { rf(s.as_ptr() as *const c_char, ch as c_char) };
                same(cv, rv, "count_occurrences random");
            }
        }
    }

    /// Row 19 — `memchra(str, c, 0)` → 0
    #[test]
    fn err_row19_memchra_zero_n() {
        let (cf, rf) = internals().memchra;
        let s = b"----\0";
        let cv = unsafe { cf(s.as_ptr() as *const c_char, b'-' as c_int, 0) };
        let rv = unsafe { rf(s.as_ptr() as *const c_char, b'-' as c_int, 0) };
        same(cv, rv, "memchra(s, '-', 0)");
        assert_eq!(cv, 0);
        // NULL with n == 0 is also fine in C (loop never dereferences)
        let cv = unsafe { cf(std::ptr::null(), b'-' as c_int, 0) };
        let rv = unsafe { rf(std::ptr::null(), b'-' as c_int, 0) };
        same(cv, rv, "memchra(NULL, '-', 0)");
        assert_eq!(cv, 0);
    }

    /// Row 20 — needle truncated to `char`: only the low 8 bits matter
    #[test]
    fn err_row20_memchra_char_truncation() {
        let (cf, rf) = internals().memchra;
        let s = b"a-b-c-\x80\xFF\0";
        let n = 9usize;
        let needles: [c_int; 14] = [
            b'-' as c_int,
            b'-' as c_int + 0x100,
            b'-' as c_int + 0x7F00,
            -0xD3, // (char)(-211) == 0x2D == '-'
            0x2D,
            0xFF,
            -1,
            0x80,
            -128,
            0,
            256,
            INT_MIN,
            INT_MAX,
            0x1234_5600 + 0x2D,
        ];
        for &c in &needles {
            let cv = unsafe { cf(s.as_ptr() as *const c_char, c, n) };
            let rv = unsafe { rf(s.as_ptr() as *const c_char, c, n) };
            same(cv, rv, &format!("memchra(s, {c}, {n})"));
        }
        // randomized needles over the whole int range
        let mut rng = Rng::new(0x20);
        for _ in 0..5000 {
            let c = rng.next_i32();
            let cv = unsafe { cf(s.as_ptr() as *const c_char, c, n) };
            let rv = unsafe { rf(s.as_ptr() as *const c_char, c, n) };
            same(cv, rv, "memchra random needle");
        }
    }

    /// Row 21 — `complex_iteration(NULL, count)` → -1
    #[test]
    fn err_row21_complex_iter_null() {
        let (cf, rf) = internals().complex_iteration;
        for count in [0usize, 1, 4, 1 << 20] {
            let cv = unsafe { cf(std::ptr::null(), count) };
            let rv = unsafe { rf(std::ptr::null(), count) };
            same(cv, rv, &format!("complex_iteration(NULL, {count})"));
            assert_eq!(cv, -1, "C sentinel must be -1");
        }
    }

    /// Row 22 — `count == 0` → -1 (NOT 0)
    #[test]
    fn err_row22_complex_iter_zero_count() {
        let (cf, rf) = internals().complex_iteration;
        let arr: [c_int; 4] = [1, 2, 3, 4];
        let cv = unsafe { cf(arr.as_ptr(), 0) };
        let rv = unsafe { rf(arr.as_ptr(), 0) };
        same(cv, rv, "complex_iteration(arr, 0)");
        assert_eq!(cv, -1, "zero count sentinel is -1, not 0");
        // randomized non-empty
        let mut rng = Rng::new(0x22);
        for _ in 0..2000 {
            let n = rng.range_i64(1, 16) as usize;
            let v: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
            let cv = unsafe { cf(v.as_ptr(), n) };
            let rv = unsafe { rf(v.as_ptr(), n) };
            same(cv, rv, "complex_iteration random");
        }
    }

    /// Row 29 — `snprintf` truncation bound, including `size == 0`
    #[test]
    fn err_row29_snprintf_bound() {
        let (cf, rf) = internals().snprintf_fmt;
        let mut rng = Rng::new(0x29);
        let quads: Vec<(i32, i32, i32, i32)> = vec![
            (0, 0, 0, 0),
            (INT_MIN, INT_MIN, INT_MIN, INT_MIN),
            (INT_MAX, INT_MAX, INT_MAX, INT_MAX),
            (-1, -22, -333, -4444),
        ]
        .into_iter()
        .chain((0..64).map(|_| {
            (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            )
        }))
        .collect();

        for &(a, b, c, d) in &quads {
            for size in [0usize, 1, 2, 3, 5, 11, 12, 32, 51, 52, 63, 64, 128] {
                let mut cbuf = vec![0x7Fu8; 200];
                let mut rbuf = vec![0x7Fu8; 200];
                let cn = unsafe { cf(cbuf.as_mut_ptr() as *mut c_char, size, a, b, c, d) };
                let rn = unsafe { rf(rbuf.as_mut_ptr() as *mut c_char, size, a, b, c, d) };
                same(cn, rn, &format!("snprintf return size={size} args={a},{b},{c},{d}"));
                assert_eq!(
                    cbuf, rbuf,
                    "snprintf buffer bytes differ for size={size} args={a},{b},{c},{d}"
                );
            }
            // NULL destination with size 0 (legal for C snprintf)
            let cn = unsafe { cf(std::ptr::null_mut(), 0, a, b, c, d) };
            let rn = unsafe { rf(std::ptr::null_mut(), 0, a, b, c, d) };
            same(cn, rn, "snprintf(NULL, 0, ...)");
            assert!(cn <= 51, "max formatted length is 51, got {cn}");
        }
    }

    /// `int_to_float_bits` — the raw type pun, over the whole `int` domain
    #[test]
    fn err_row_extra_int_to_float_bits_bitwise() {
        let (cf, rf) = internals().int_to_float_bits;
        let mut rng = Rng::new(0xF10A7);
        let pinned: [i32; 14] = [
            0,
            1,
            -1,
            INT_MIN,
            INT_MAX,
            0x0080_0000,
            0x3F80_0000,
            0x447A_0000,
            0x447A_0000u32.wrapping_sub(1) as i32,
            0x7F80_0000,
            0x7F80_0001,
            0x7FC0_0000,
            0xFF80_0000u32 as i32,
            0xFFFF_FFFFu32 as i32,
        ];
        for &v in &pinned {
            let cv = unsafe { cf(v) };
            let rv = unsafe { rf(v) };
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "int_to_float_bits({v}) differs bitwise: C={cv:?} Rust={rv:?}"
            );
        }
        for _ in 0..200_000 {
            let v = rng.next_i32();
            let cv = unsafe { cf(v) };
            let rv = unsafe { rf(v) };
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "int_to_float_bits({v}) differs bitwise"
            );
        }
    }
}

// ===========================================================================
// Rows reachable through the public `memchra2` entry point
// ===========================================================================

/// Row 23 — float guard rejects non-positive `f`
#[test]
fn err_row23_float_guard_non_positive() {
    let mut rng = Rng::new(0x23);
    // +0.0 and -0.0
    assert_same(0, 0, 0, 0);
    assert_same(0x8000_0000u32 as i32, 0, 0, 0);
    for _ in 0..2000 {
        // any negative int is a negative float bit pattern
        let a = rng.range_i32(INT_MIN, -1);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 24 — float guard rejects `f >= 1000.0f`
#[test]
fn err_row24_float_guard_ge_1000() {
    let mut rng = Rng::new(0x24);
    assert_same(0x447A_0000u32 as i32, 0, 0, 0); // exactly 1000.0 -> excluded
    assert_same(0x447A_0001u32 as i32, 0, 0, 0);
    for _ in 0..2000 {
        let a = rng.range_u32_as_i32(0x447A_0000, 0x7F7F_FFFF);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 25 — `+inf`
#[test]
fn err_row25_float_guard_pos_inf() {
    let mut rng = Rng::new(0x25);
    let a = f32::INFINITY.to_bits() as i32;
    assert_same(a, 0, 0, 0);
    for _ in 0..500 {
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 26 — `-inf`
#[test]
fn err_row26_float_guard_neg_inf() {
    let mut rng = Rng::new(0x26);
    let a = f32::NEG_INFINITY.to_bits() as i32;
    assert_eq!(a, -8388608);
    assert_same(a, 0, 0, 0);
    for _ in 0..500 {
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 27 — NaN (both comparisons false)
#[test]
fn err_row27_float_guard_nan() {
    let mut rng = Rng::new(0x27);
    for bits in [
        0x7F80_0001u32,
        0x7FBF_FFFF,
        0x7FC0_0000,
        0x7FFF_FFFF,
        0xFF80_0001,
        0xFFC0_0000,
        0xFFFF_FFFF,
    ] {
        let a = bits as i32;
        assert_same(a, 0, 0, 0);
        for _ in 0..100 {
            assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}

/// Row 28 — `(int)f` on subnormals / `f < 1` truncates to 0
#[test]
fn err_row28_float_trunc_subnormal() {
    let mut rng = Rng::new(0x28);
    for a in [1i32, 2, 0x7F_FFFF, 0x0080_0000, 0x3F7F_FFFF] {
        assert_same(a, 0, 0, 0);
    }
    for _ in 0..2000 {
        let a = rng.range_u32_as_i32(1, 0x3F7F_FFFF);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Row 30 — INT_MIN arguments (longest `%d` field)
#[test]
fn err_row30_int_min_args() {
    assert_same(INT_MIN, INT_MIN, INT_MIN, INT_MIN);
    for pos in 0..4 {
        let mut v = [0i32; 4];
        v[pos] = INT_MIN;
        assert_same(v[0], v[1], v[2], v[3]);
        let mut v = [1i32; 4];
        v[pos] = INT_MIN;
        assert_same(v[0], v[1], v[2], v[3]);
    }
}

/// Row 31 — INT_MAX arguments
#[test]
fn err_row31_int_max_args() {
    assert_same(INT_MAX, INT_MAX, INT_MAX, INT_MAX);
    for pos in 0..4 {
        let mut v = [0i32; 4];
        v[pos] = INT_MAX;
        assert_same(v[0], v[1], v[2], v[3]);
        let mut v = [-1i32; 4];
        v[pos] = INT_MAX;
        assert_same(v[0], v[1], v[2], v[3]);
    }
}

/// Row 32 — `result` overflow wraps identically
#[test]
fn err_row32_result_overflow() {
    let mut rng = Rng::new(0x32);
    for _ in 0..5000 {
        // near the extremes so `sum` and `result` wrap
        let pick = |rng: &mut Rng| match rng.next_u32() % 4 {
            0 => INT_MAX - (rng.next_u32() % 64) as i32,
            1 => INT_MIN + (rng.next_u32() % 64) as i32,
            2 => (rng.next_u32() % 64) as i32,
            _ => rng.next_i32(),
        };
        let a = pick(&mut rng);
        let b = pick(&mut rng);
        let c = pick(&mut rng);
        let d = pick(&mut rng);
        assert_same(a, b, c, d);
    }
}

/// Row 33 — every one of the 2^32 `int` values is in-domain: no rejection path.
/// This is the analogue of "out-of-range enum value" for an API whose only
/// parameters are plain `int`s (there are no enums in `lib.h`): values with no
/// "valid variant" simply must behave identically in both libraries.
#[test]
fn err_row33_full_int_domain_no_rejection() {
    let mut rng = Rng::new(0x33);
    // bit-pattern classes that would be "invalid variants" for an enum-like arg
    let weird: [i32; 16] = [
        INT_MIN,
        INT_MIN + 1,
        -0x4000_0000,
        -0x0100_0000,
        -65536,
        -256,
        -128,
        -1,
        0,
        1,
        127,
        128,
        255,
        0x0100_0000,
        0x4000_0000,
        INT_MAX,
    ];
    for &v in &weird {
        // exercise each parameter position with the odd value
        for pos in 0..4 {
            let mut q = [0i32; 4];
            q[pos] = v;
            assert_same(q[0], q[1], q[2], q[3]);
            let mut q = [7i32; 4];
            q[pos] = v;
            assert_same(q[0], q[1], q[2], q[3]);
        }
    }
    for _ in 0..20_000 {
        assert_same(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

/// Row 34 — degenerate all-zero input is accepted, not rejected
#[test]
fn err_row34_all_zero() {
    let v = assert_same(0, 0, 0, 0);
    // pinned so an accidental change of any guard shows up
    println!("memchra2(0,0,0,0) = {v}");
}

/// Guard: with the default feature set the internals harness is unavailable, so
/// make it obvious that Phase C must be run with `--features test_internals`.
#[cfg(not(feature = "test_internals"))]
#[test]
fn phase_c_internals_require_feature() {
    eprintln!(
        "NOTE: ERRORS.md rows 1-22 and 29 exercise the `static` helpers of lib.c \
         and require `cargo test --features test_internals`."
    );
}

// keep `c_char` import used in both feature configurations
#[allow(dead_code)]
fn _c_char_used(_: *const c_char) {}
