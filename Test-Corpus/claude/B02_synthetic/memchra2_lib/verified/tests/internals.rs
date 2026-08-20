//! Phase B — valid-path differential tests for the **low-level** entry points
//! (rows 16–40 of `CONFIGS.md`).
//!
//! The eight helpers are `static` in the C translation unit, so the C side is
//! reached through `tests/cshim/shim.c` (which `#include`s `c_src/src/lib.c`
//! verbatim) and the Rust side through the `itest_*` exports of feature
//! `internal_test_api`.  Both are always called via `dlsym`.
#![cfg(feature = "internal_test_api")]

mod common;

use std::ffi::{c_char, c_int, c_uchar};

use common::{c_shim, pair, Rng, SEED};

type FMemchra = unsafe extern "C" fn(*const c_char, c_int, usize) -> c_int;
type FProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
type FIntToFloat = unsafe extern "C" fn(c_int) -> f32;
type FProcessStrings = unsafe extern "C" fn(*mut *mut c_char, c_int, *const c_char) -> c_int;
type FSafeSum = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
type FInterpret = unsafe extern "C" fn(*mut c_uchar, usize) -> c_int;
type FCount = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type FComplex = unsafe extern "C" fn(*mut c_int, usize) -> c_int;
type FMemchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn memchra_pair() -> (FMemchra, FMemchra) {
    unsafe { pair::<FMemchra>(c_shim(), "itest_memchra") }
}
fn process_buffer_pair() -> (FProcessBuffer, FProcessBuffer) {
    unsafe { pair::<FProcessBuffer>(c_shim(), "itest_process_buffer") }
}
fn int_to_float_pair() -> (FIntToFloat, FIntToFloat) {
    unsafe { pair::<FIntToFloat>(c_shim(), "itest_int_to_float_bits") }
}
fn process_strings_pair() -> (FProcessStrings, FProcessStrings) {
    unsafe { pair::<FProcessStrings>(c_shim(), "itest_process_strings") }
}
fn safe_sum_pair() -> (FSafeSum, FSafeSum) {
    unsafe { pair::<FSafeSum>(c_shim(), "itest_safe_sum_array") }
}
fn interpret_pair() -> (FInterpret, FInterpret) {
    unsafe { pair::<FInterpret>(c_shim(), "itest_interpret_as_int") }
}
fn count_pair() -> (FCount, FCount) {
    unsafe { pair::<FCount>(c_shim(), "itest_count_occurrences") }
}
fn complex_pair() -> (FComplex, FComplex) {
    unsafe { pair::<FComplex>(c_shim(), "itest_complex_iteration") }
}

fn to_cchar(bytes: &[u8]) -> Vec<c_char> {
    bytes.iter().map(|&b| b as c_char).collect()
}

// ---------------------------------------------------------------------------
// memchra
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_memchra(label: &str, buf: &[u8], c: c_int, n: usize) {
    let (fc, fr) = memchra_pair();
    let data = to_cchar(buf);
    let p = data.as_ptr();
    let rc = unsafe { fc(p, c, n) };
    let rr = unsafe { fr(p, c, n) };
    assert_eq!(rc, rr, "{label}: memchra({buf:?}, c={c}, n={n}) C={rc} Rust={rr}");
}

/// Row 16 — n ∈ {0, 1, strlen-1, strlen, strlen+k}.
#[test]
fn cfg16_memchra_lengths() {
    let buf = b"test-1-22-333\0extra-tail\0";
    let strlen = 13usize;
    for n in [0usize, 1, strlen - 1, strlen, strlen + 1, strlen + 5, buf.len()] {
        for c in [b'-' as c_int, b't' as c_int, 0, b'z' as c_int] {
            chk_memchra("cfg16", buf, c, n);
        }
    }
}

/// Row 17 — needle absent / once / many / every byte matching.
#[test]
fn cfg17_memchra_needle_density() {
    let cases: [&[u8]; 6] = [
        b"",
        b"a",
        b"aaaaaaaa",
        b"abababab",
        b"xyz",
        b"----------------",
    ];
    for buf in cases {
        for c in [b'a' as c_int, b'-' as c_int, b'x' as c_int, b'q' as c_int] {
            chk_memchra("cfg17", buf, c, buf.len());
        }
    }
}

/// Row 18 — `c` outside the `char` range, plus `c == 0` over embedded NULs.
#[test]
fn cfg18_memchra_needle_values() {
    let buf: [u8; 12] = [b'A', 0, b'A', 0xC1, 0x41, 0xFF, 0x00, b'z', 0x80, 0x7F, 0x41, 0];
    let needles: [c_int; 14] = [
        0,
        0x41,
        0x141,
        256,
        -1,
        255,
        0xFF,
        -256,
        i32::MIN,
        i32::MAX,
        0x1_0000_0000u64 as i32,
        0x80,
        -128,
        0x7F,
    ];
    for &c in &needles {
        for n in 0..=buf.len() {
            chk_memchra("cfg18", &buf, c, n);
        }
    }
}

/// Row 19 — randomized buffers × randomized needles.
#[test]
fn cfg19_memchra_random() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..4000 {
        let len = rng.below(257);
        let buf: Vec<u8> = (0..len).map(|_| rng.next_u8()).collect();
        let n = rng.below(len + 1);
        let c = match rng.below(3) {
            0 => rng.next_u8() as c_int,
            1 => rng.next_i32(),
            _ => (rng.next_u8() as c_int) | 0x100,
        };
        chk_memchra("cfg19", &buf, c, n);
    }
}

// ---------------------------------------------------------------------------
// process_buffer
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_process_buffer(label: &str, buf: &[u8], len: usize) {
    let (fc, fr) = process_buffer_pair();
    let mut data_c = to_cchar(buf);
    let mut data_r = to_cchar(buf);
    let rc = unsafe { fc(data_c.as_mut_ptr(), len) };
    let rr = unsafe { fr(data_r.as_mut_ptr(), len) };
    assert_eq!(rc, rr, "{label}: process_buffer({buf:?}, len={len}) C={rc} Rust={rr}");
    assert_eq!(data_c, data_r, "{label}: buffer mutated differently");
}

/// Row 20 — len < strlen, len == strlen, len > strlen (interior NUL).
#[test]
fn cfg20_process_buffer_lengths() {
    let buf = b"test1-2-3\0tail\0";
    for len in 0..=buf.len() {
        chk_process_buffer("cfg20", buf, len);
    }
    let no_nul = b"abcdef";
    for len in 0..=no_nul.len() {
        chk_process_buffer("cfg20-nonul", no_nul, len);
    }
}

/// Row 21 — bytes >= 0x80 (signed `char` sign extension → negative sum).
#[test]
fn cfg21_process_buffer_high_bytes() {
    let cases: [&[u8]; 5] = [
        &[0x80, 0x80, 0x80, 0x80],
        &[0xFF],
        &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        &[0x7F, 0x80, 0x81, 0xFE, 0xFF, 0x01],
        &[0xC1, 0xC2, b'a', 0xF0, 0x9F, 0x92, 0xA9],
    ];
    for buf in cases {
        for len in 0..=buf.len() {
            chk_process_buffer("cfg21", buf, len);
        }
    }
}

/// Row 22 — long buffer whose signed sum wraps past INT_MAX.
#[test]
fn cfg22_process_buffer_overflow() {
    // 0x7F per byte, 20 million bytes ≈ 2.54e9 > INT_MAX → wraps.
    let big: Vec<u8> = std::iter::repeat(0x7Fu8).take(20_000_000).collect();
    chk_process_buffer("cfg22-pos", &big, big.len());
    // 0x80 per byte → -128 each → wraps in the negative direction.
    let big_neg: Vec<u8> = std::iter::repeat(0x80u8).take(20_000_000).collect();
    chk_process_buffer("cfg22-neg", &big_neg, big_neg.len());
}

/// Row 23 — randomized non-empty buffers × randomized len.
#[test]
fn cfg23_process_buffer_random() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..4000 {
        let len = 1 + rng.below(128);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.next_u8()).collect();
        // keep it non-empty at index 0 most of the time, but sometimes make the
        // first byte a NUL to hit the `*buffer == '\0'` guard from the valid side
        if rng.below(8) == 0 {
            buf[0] = 0;
        } else if buf[0] == 0 {
            buf[0] = b'x';
        }
        // Terminate the buffer so that `len` values beyond the payload still
        // stop at a NUL instead of reading out of bounds (which would be UB in
        // the C and therefore not comparable).
        buf.push(0);
        let n = rng.below(buf.len() + 1);
        chk_process_buffer("cfg23", &buf, n);
    }
}

// ---------------------------------------------------------------------------
// int_to_float_bits
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_int_to_float(label: &str, v: c_int) {
    let (fc, fr) = int_to_float_pair();
    let rc = unsafe { fc(v) };
    let rr = unsafe { fr(v) };
    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "{label}: int_to_float_bits({v:#010x}) C={:#010x} Rust={:#010x}",
        rc.to_bits(),
        rr.to_bits()
    );
    // also compare the branch decision memchra2 makes on the result
    let dc = rc > 0.0 && rc < 1000.0;
    let dr = rr > 0.0 && rr < 1000.0;
    assert_eq!(dc, dr, "{label}: range predicate differs for {v:#010x}");
    if dc {
        assert_eq!(rc as c_int, rr as c_int, "{label}: (int)f differs for {v:#010x}");
    }
}

/// Row 24 — exact bit patterns + random bits.
#[test]
fn cfg24_int_to_float_bits() {
    let fixed: [u32; 16] = [
        0,
        1,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0x3F80_0000,
        0x447A_0000,
        0x4479_FFFF,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0x7F80_0001,
        0x0080_0000,
        0x007F_FFFF,
        0x4000_0000,
        0xC000_0000,
    ];
    for &b in &fixed {
        chk_int_to_float("cfg24-fixed", b as c_int);
    }
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..20_000 {
        chk_int_to_float("cfg24-rand", rng.next_i32());
    }
    for _ in 0..20_000 {
        chk_int_to_float("cfg24-interesting", rng.next_i32_interesting());
    }
}

// ---------------------------------------------------------------------------
// process_strings
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_process_strings(label: &str, elems: &[Option<&[u8]>], count: c_int, target: &[u8]) {
    let (fc, fr) = process_strings_pair();

    // Keep the backing storage alive for the duration of the calls.
    let storage: Vec<Option<Vec<c_char>>> = elems
        .iter()
        .map(|e| e.map(|bytes| to_cchar(bytes)))
        .collect();
    let mut ptrs: Vec<*mut c_char> = storage
        .iter()
        .map(|s| match s {
            Some(v) => v.as_ptr() as *mut c_char,
            None => std::ptr::null_mut(),
        })
        .collect();
    let tgt = to_cchar(target);

    let rc = unsafe { fc(ptrs.as_mut_ptr(), count, tgt.as_ptr()) };
    let rr = unsafe { fr(ptrs.as_mut_ptr(), count, tgt.as_ptr()) };
    assert_eq!(
        rc, rr,
        "{label}: process_strings({elems:?}, count={count}, target={target:?}) C={rc} Rust={rr}"
    );
}

/// Row 25 — count ∈ {1,2,3,4,8}, all / none / some matching.
#[test]
fn cfg25_process_strings_counts() {
    let all: [Option<&[u8]>; 8] = [
        Some(b"test1\0"),
        Some(b"test2\0"),
        Some(b"testing\0"),
        Some(b"other\0"),
        Some(b"te\0"),
        Some(b"test\0"),
        Some(b"TEST\0"),
        Some(b"ztest\0"),
    ];
    for count in [1i32, 2, 3, 4, 5, 6, 7, 8] {
        for target in [&b"test\0"[..], &b"other\0"[..], &b"z\0"[..], &b"testing\0"[..]] {
            chk_process_strings("cfg25", &all, count, target);
        }
    }
    let none: [Option<&[u8]>; 3] = [Some(b"aaa\0"), Some(b"bbb\0"), Some(b"ccc\0")];
    chk_process_strings("cfg25-none", &none, 3, b"test\0");
    let every: [Option<&[u8]>; 3] = [Some(b"testa\0"), Some(b"testb\0"), Some(b"test\0")];
    chk_process_strings("cfg25-every", &every, 3, b"test\0");
}

/// Row 26 — target shapes: empty, equal, longer, shorter, embedded NUL.
#[test]
fn cfg26_process_strings_targets() {
    let elems: [Option<&[u8]>; 5] = [
        Some(b"abc\0"),
        Some(b"abcdef\0"),
        Some(b"ab\0"),
        Some(b"a\0"),
        Some(b"zzzz\0"),
    ];
    let targets: [&[u8]; 8] = [
        b"\0",            // strlen == 0 → strncmp(...,0) == 0 → all match
        b"abc\0",
        b"abcdef\0",
        b"abcdefghij\0",
        b"a\0",
        b"ab\0cd\0",      // embedded NUL → strlen is 2
        b"z\0",
        b"\0abc\0",       // leading NUL → strlen 0
    ];
    for t in targets {
        for count in 1..=5i32 {
            chk_process_strings("cfg26", &elems, count, t);
        }
    }
}

/// Row 27 — NULL and empty elements interleaved with matches.
#[test]
fn cfg27_process_strings_holes() {
    let elems: [Option<&[u8]>; 8] = [
        None,
        Some(b"\0"),
        Some(b"test1\0"),
        None,
        Some(b"\0"),
        Some(b"test2\0"),
        Some(b"nope\0"),
        None,
    ];
    for count in 1..=8i32 {
        for t in [&b"test\0"[..], &b"\0"[..], &b"nope\0"[..]] {
            chk_process_strings("cfg27", &elems, count, t);
        }
    }
}

/// Row 28 — randomized element sets (with holes) × randomized targets.
#[test]
fn cfg28_process_strings_random() {
    let mut rng = Rng::new(SEED ^ 28);
    let alphabet = b"abtes\0";
    for _ in 0..2000 {
        let n = 1 + rng.below(8);
        let mut owned: Vec<Option<Vec<u8>>> = Vec::with_capacity(n);
        for _ in 0..n {
            match rng.below(6) {
                0 => owned.push(None),
                1 => owned.push(Some(vec![0])),
                _ => {
                    let len = rng.below(6);
                    let mut s: Vec<u8> = (0..len)
                        .map(|_| alphabet[rng.below(alphabet.len() - 1)])
                        .collect();
                    s.push(0);
                    owned.push(Some(s));
                }
            }
        }
        let refs: Vec<Option<&[u8]>> = owned.iter().map(|o| o.as_deref()).collect();

        let tlen = rng.below(5);
        let mut target: Vec<u8> = (0..tlen)
            .map(|_| alphabet[rng.below(alphabet.len() - 1)])
            .collect();
        target.push(0);

        let count = rng.below(n) as c_int + 1;
        chk_process_strings("cfg28", &refs, count, &target);
    }
}

// ---------------------------------------------------------------------------
// safe_sum_array
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_safe_sum(label: &str, arr: &[c_int], size: usize) {
    let (fc, fr) = safe_sum_pair();
    let mut a_c = arr.to_vec();
    let mut a_r = arr.to_vec();
    let rc = unsafe { fc(a_c.as_mut_ptr(), size) };
    let rr = unsafe { fr(a_r.as_mut_ptr(), size) };
    assert_eq!(rc, rr, "{label}: safe_sum_array(len={}, size={size}) C={rc} Rust={rr}", arr.len());
    assert_eq!(a_c, a_r, "{label}: array mutated differently");
}

/// Row 29 — sizes and value distributions.
#[test]
fn cfg29_safe_sum_shapes() {
    let mut rng = Rng::new(SEED ^ 29);
    for size in [1usize, 2, 4, 17, 1000] {
        let pos: Vec<c_int> = (0..size).map(|_| (rng.next_u32() % 1000) as c_int).collect();
        let neg: Vec<c_int> = (0..size).map(|_| -((rng.next_u32() % 1000) as c_int)).collect();
        let mix: Vec<c_int> = (0..size).map(|_| rng.next_i32()).collect();
        chk_safe_sum("cfg29-pos", &pos, size);
        chk_safe_sum("cfg29-neg", &neg, size);
        chk_safe_sum("cfg29-mix", &mix, size);
        // size smaller than the allocation
        chk_safe_sum("cfg29-partial", &mix, size / 2);
    }
}

/// Row 30 — signed wraparound.
#[test]
fn cfg30_safe_sum_overflow() {
    let cases: [&[c_int]; 6] = [
        &[i32::MAX, i32::MAX],
        &[i32::MIN, i32::MIN],
        &[i32::MAX, 1],
        &[i32::MIN, -1],
        &[i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        &[i32::MIN, i32::MAX, i32::MIN, i32::MAX],
    ];
    for c in cases {
        chk_safe_sum("cfg30", c, c.len());
    }
    let many: Vec<c_int> = std::iter::repeat(i32::MAX).take(64).collect();
    chk_safe_sum("cfg30-many", &many, many.len());
}

/// Row 31 — randomized arrays over the full int range.
#[test]
fn cfg31_safe_sum_random() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..3000 {
        let n = 1 + rng.below(64);
        let arr: Vec<c_int> = (0..n).map(|_| rng.next_i32_interesting()).collect();
        let size = rng.below(n + 1);
        chk_safe_sum("cfg31", &arr, size);
    }
}

// ---------------------------------------------------------------------------
// interpret_as_int
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_interpret(label: &str, bytes: &[u8], len: usize, offset: usize) {
    let (fc, fr) = interpret_pair();
    let mut b_c = bytes.to_vec();
    let mut b_r = bytes.to_vec();
    let rc = unsafe { fc(b_c.as_mut_ptr().add(offset), len) };
    let rr = unsafe { fr(b_r.as_mut_ptr().add(offset), len) };
    assert_eq!(
        rc, rr,
        "{label}: interpret_as_int({bytes:?}+{offset}, len={len}) C={rc} Rust={rr}"
    );
}

/// Row 32 — len == 4, len > 4, known byte patterns, all-0xFF.
#[test]
fn cfg32_interpret_shapes() {
    let cases: [&[u8]; 6] = [
        &[1, 0, 0, 0],
        &[0, 0, 0, 1],
        &[0xFF, 0xFF, 0xFF, 0xFF],
        &[0x78, 0x56, 0x34, 0x12],
        &[0xFF, 0xFF, 0xFF, 0x7F],
        &[0x00, 0x00, 0x00, 0x80],
    ];
    for b in cases {
        chk_interpret("cfg32-len4", b, 4, 0);
    }
    let long: [u8; 16] = [
        0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    ];
    for len in 4..=16usize {
        chk_interpret("cfg32-long", &long, len, 0);
    }
    // len == sizeof(int) exactly, one step above the rejection boundary
    chk_interpret("cfg32-boundary", &long, 4, 0);
}

/// Row 33 — misaligned base pointer.
#[test]
fn cfg33_interpret_unaligned() {
    let buf: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 0xFF, 0xFE, 0xFD, 0xFC];
    for offset in 0..8usize {
        chk_interpret("cfg33", &buf, 4, offset);
        chk_interpret("cfg33-longer", &buf, 12 - offset, offset);
    }
}

/// Row 34 — randomized byte buffers.
#[test]
fn cfg34_interpret_random() {
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..4000 {
        let n = 4 + rng.below(28);
        let buf: Vec<u8> = (0..n).map(|_| rng.next_u8()).collect();
        let offset = rng.below(n - 3);
        let len = 4 + rng.below(n - offset - 3);
        chk_interpret("cfg34", &buf, len, offset);
    }
}

// ---------------------------------------------------------------------------
// count_occurrences
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_count(label: &str, text: &[u8], ch: c_char) {
    let (fc, fr) = count_pair();
    let data = to_cchar(text);
    let rc = unsafe { fc(data.as_ptr(), ch) };
    let rr = unsafe { fr(data.as_ptr(), ch) };
    assert_eq!(rc, rr, "{label}: count_occurrences({text:?}, {ch}) C={rc} Rust={rr}");
}

/// Row 35 — 1-char texts, needle present/absent, ch == 0, ch == (char)0xFF.
#[test]
fn cfg35_count_shapes() {
    let texts: [&[u8]; 8] = [
        b"a\0",
        b"-\0",
        b"test1-2-3-4\0",
        b"--------\0",
        b"\0",
        b"abc\0def\0",
        &[0xFF, 0xFE, 0x80, b'a', 0],
        &[0x7F, 0x80, 0x81, 0xFF, 0x01, 0],
    ];
    let needles: [c_char; 10] = [
        0,
        b'a' as c_char,
        b'-' as c_char,
        b'z' as c_char,
        -1,
        -128,
        127,
        b'1' as c_char,
        0x7F,
        (0x80u8) as i8,
    ];
    for t in texts {
        for &ch in &needles {
            chk_count("cfg35", t, ch);
        }
    }
}

/// Row 36 — randomized NUL-terminated texts × randomized needles.
#[test]
fn cfg36_count_random() {
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..4000 {
        let len = rng.below(129);
        let mut t: Vec<u8> = (0..len)
            .map(|_| {
                let b = rng.next_u8();
                if b == 0 && rng.below(2) == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        t.push(0);
        let ch = rng.next_u8() as c_char;
        chk_count("cfg36", &t, ch);
    }
}

// ---------------------------------------------------------------------------
// complex_iteration
// ---------------------------------------------------------------------------

#[track_caller]
fn chk_complex(label: &str, data: &[c_int], count: usize) {
    let (fc, fr) = complex_pair();
    let mut d_c = data.to_vec();
    let mut d_r = data.to_vec();
    let rc = unsafe { fc(d_c.as_mut_ptr(), count) };
    let rr = unsafe { fr(d_r.as_mut_ptr(), count) };
    assert_eq!(rc, rr, "{label}: complex_iteration(len={}, count={count}) C={rc} Rust={rr}", data.len());
    assert_eq!(d_c, d_r, "{label}: array mutated differently");
}

/// Row 37 — counts, negative values, zero low bytes, XOR-to-zero sets.
#[test]
fn cfg37_complex_shapes() {
    chk_complex("cfg37-1", &[0x1234_5678], 1);
    chk_complex("cfg37-neg", &[-1, -2, -3, -4], 4);
    chk_complex("cfg37-zerolow", &[0x100, 0x200, 0x300, 0x400], 4);
    chk_complex("cfg37-xor0", &[0xAA, 0xAA, 0x55, 0x55], 4);
    chk_complex("cfg37-max", &[i32::MAX, i32::MIN, -1, 0], 4);
    let big: Vec<c_int> = (0..256).map(|i| i as c_int * 7 - 900).collect();
    for count in [1usize, 2, 3, 4, 255, 256] {
        chk_complex("cfg37-big", &big, count);
    }
}

/// Row 38 — randomized arrays.
#[test]
fn cfg38_complex_random() {
    let mut rng = Rng::new(SEED ^ 38);
    for _ in 0..3000 {
        let n = 1 + rng.below(64);
        let arr: Vec<c_int> = (0..n).map(|_| rng.next_i32_interesting()).collect();
        let count = 1 + rng.below(n);
        chk_complex("cfg38", &arr, count);
    }
}

// ---------------------------------------------------------------------------
// Row 39 — whole-pipeline consistency
// ---------------------------------------------------------------------------

/// Re-composes the body of `memchra2` out of the individually-loaded helpers of
/// one implementation and checks it against that implementation's exported
/// `memchra2`.  A divergence here means the composition (buffer handling,
/// `snprintf` formatting, argument threading) is wrong even though the parts
/// agree.
fn compose(from_c: bool, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let (count_c, count_r) = count_pair();
    let (sum_c, sum_r) = safe_sum_pair();
    let (ps_c, ps_r) = process_strings_pair();
    let (itf_c, itf_r) = int_to_float_pair();
    let (pb_c, pb_r) = process_buffer_pair();
    let (int_c, int_r) = interpret_pair();
    let (cx_c, cx_r) = complex_pair();

    let f_count = if from_c { count_c } else { count_r };
    let f_sum = if from_c { sum_c } else { sum_r };
    let f_ps = if from_c { ps_c } else { ps_r };
    let f_itf = if from_c { itf_c } else { itf_r };
    let f_pb = if from_c { pb_c } else { pb_r };
    let f_int = if from_c { int_c } else { int_r };
    let f_cx = if from_c { cx_c } else { cx_r };

    let mut result: c_int = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d)
    let s = format!("test{a}-{b}-{c}-{d}");
    let bytes = s.as_bytes();
    let mut buffer = [0 as c_char; 64];
    let n = bytes.len().min(63);
    for i in 0..n {
        buffer[i] = bytes[i] as c_char;
    }
    buffer[n] = 0;

    let dash = unsafe { f_count(buffer.as_ptr(), b'-' as c_char) };
    result = result.wrapping_add(dash.wrapping_mul(10));

    let mut values: [c_int; 4] = [a, b, c, d];
    result = result.wrapping_add(unsafe { f_sum(values.as_mut_ptr(), 4) });

    let s0 = to_cchar(b"test1\0");
    let s1 = to_cchar(b"test2\0");
    let s2 = to_cchar(b"testing\0");
    let s3 = to_cchar(b"other\0");
    let mut strs: [*mut c_char; 4] = [
        s0.as_ptr() as *mut c_char,
        s1.as_ptr() as *mut c_char,
        s2.as_ptr() as *mut c_char,
        s3.as_ptr() as *mut c_char,
    ];
    let target = to_cchar(b"test\0");
    let matches = unsafe { f_ps(strs.as_mut_ptr(), 4, target.as_ptr()) };
    result = result.wrapping_add(matches.wrapping_mul(5));

    let fv = unsafe { f_itf(a) };
    if fv > 0.0 && fv < 1000.0 {
        result = result.wrapping_add(fv as c_int);
    }

    let strlen = buffer.iter().position(|&x| x == 0).unwrap();
    let buf_sum = unsafe { f_pb(buffer.as_mut_ptr(), strlen) };
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bs: [c_uchar; 4] = [
        (b as u32 & 0xFF) as c_uchar,
        (c as u32 & 0xFF) as c_uchar,
        (d as u32 & 0xFF) as c_uchar,
        0,
    ];
    result ^= unsafe { f_int(bs.as_mut_ptr(), 4) };

    result = result.wrapping_add(unsafe { f_cx(values.as_mut_ptr(), 4) });

    result
}

/// Row 39 — the composed pipeline must equal the exported one-shot entry point,
/// for both implementations, on the same randomized inputs.
#[test]
fn cfg39_pipeline_consistency() {
    let m2_c = unsafe { pair::<FMemchra2>(c_shim(), "memchra2") };
    let (m2_from_shim, m2_rust) = m2_c;

    let mut rng = Rng::new(SEED ^ 39);
    for i in 0..4000 {
        let (a, b, c, d) = if i < 625 {
            let vals = [i32::MIN, i32::MAX, 0, -1, 1];
            (
                vals[i % 5],
                vals[(i / 5) % 5],
                vals[(i / 25) % 5],
                vals[(i / 125) % 5],
            )
        } else {
            (
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
            )
        };

        let comp_c = compose(true, a, b, c, d);
        let comp_r = compose(false, a, b, c, d);
        let one_shot_c = unsafe { m2_from_shim(a, b, c, d) };
        let one_shot_r = unsafe { m2_rust(a, b, c, d) };

        assert_eq!(
            comp_c, one_shot_c,
            "cfg39: C composed={comp_c} vs C memchra2={one_shot_c} for ({a},{b},{c},{d})"
        );
        assert_eq!(
            comp_r, one_shot_r,
            "cfg39: Rust composed={comp_r} vs Rust memchra2={one_shot_r} for ({a},{b},{c},{d})"
        );
        assert_eq!(
            one_shot_c, one_shot_r,
            "cfg39: C={one_shot_c} Rust={one_shot_r} for ({a},{b},{c},{d})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 40 — the `snprintf("test%d-%d-%d-%d", …)` call site
// ---------------------------------------------------------------------------

type FFormat = unsafe extern "C" fn(c_int, c_int, c_int, c_int, *mut c_char, usize);

/// Row 40 — the formatted buffer must match glibc's `%d` byte for byte
/// (the Rust side re-implements the conversion).
#[test]
fn cfg40_snprintf_formatting() {
    let (fc, fr) = unsafe { pair::<FFormat>(c_shim(), "itest_format_buffer") };

    let compare = |a: c_int, b: c_int, c: c_int, d: c_int| {
        let mut buf_c = [0 as c_char; 64];
        let mut buf_r = [0 as c_char; 64];
        unsafe { fc(a, b, c, d, buf_c.as_mut_ptr(), buf_c.len()) };
        unsafe { fr(a, b, c, d, buf_r.as_mut_ptr(), buf_r.len()) };
        assert_eq!(
            buf_c, buf_r,
            "cfg40: snprintf(\"test%d-%d-%d-%d\", {a}, {b}, {c}, {d})\n  C   ={}\n  Rust={}",
            String::from_utf8_lossy(
                &buf_c
                    .iter()
                    .take_while(|&&x| x != 0)
                    .map(|&x| x as u8)
                    .collect::<Vec<u8>>()
            ),
            String::from_utf8_lossy(
                &buf_r
                    .iter()
                    .take_while(|&&x| x != 0)
                    .map(|&x| x as u8)
                    .collect::<Vec<u8>>()
            ),
        );
    };

    // boundary matrix
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0,
        -1,
        1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        999_999_999,
        1_000_000_000,
        -999_999_999,
        -1_000_000_000,
    ];
    for &a in &vals {
        for &b in &vals {
            compare(a, b, 0, -1);
            compare(0, -1, a, b);
        }
    }

    // decimal-width sweep in every position
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for &sign in &[1i64, -1] {
            let v = (sign * p) as i32;
            compare(v, v, v, v);
            compare(v, 1, -2, 3);
            compare(1, v, -2, 3);
            compare(1, -2, v, 3);
            compare(1, -2, 3, v);
        }
        p *= 10;
    }

    // randomized full-range fuzz
    let mut rng = Rng::new(SEED ^ 40);
    for _ in 0..20_000 {
        compare(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
    for _ in 0..5_000 {
        compare(
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
}
