//! Differential tests: the C `.so` and the Rust `.so` are BOTH loaded with
//! `libloading` and every call goes through their exported `encode_base64`
//! symbol. No Rust function is ever called directly.
//!
//! Phase B — one test per row of `CONFIGS.md` (rows 1..=18).
//! Phase C — one test per row of `ERRORS.md`  (rows 1..=10) plus generic
//!           boundary coverage.

mod common;

use common::*;
use std::os::raw::c_int;

// ===========================================================================
// Phase A sanity: both libraries really do export the symbol.
// ===========================================================================

#[test]
fn phase_a_symbol_present_in_both_sos() {
    let l = libs();
    let c = l.c_encode();
    let r = l.rs_encode();
    // Resolving the symbols is the assertion; touch them so nothing is elided.
    assert!(!(*c as usize == 0));
    assert!(!(*r as usize == 0));
}

// ===========================================================================
// Phase B — valid-path differential tests, gated on CONFIGS.md
// ===========================================================================

/// Row 1 — A1 (`size = 0`) + empty NUL-terminated string.
#[test]
fn phase_b_row01_strlen_mode_empty_string() {
    check("row01 empty", 0, b"\0");
}

/// Helper for rows 2/3/4: `size = 0`, NUL-terminated ASCII with
/// `strlen % 3 == rem`.
fn strlen_mode_ascii(label: &str, rem: usize) {
    let mut rng = Rng::new(SEED ^ rem as u64);
    for k in 0..SAMPLES {
        let len = 3 * k + rem;
        if len == 0 {
            continue;
        }
        let mut p = rng.ascii(len);
        assert_eq!(len % 3, rem);
        p.push(0); // NUL terminator for strlen mode
        check(label, 0, &p);
    }
}

/// Row 2 — A1 + `strlen % 3 == 0` (B0, no padding).
#[test]
fn phase_b_row02_strlen_mode_len_mod3_eq0() {
    strlen_mode_ascii("row02 strlen %3==0", 0);
}

/// Row 3 — A1 + `strlen % 3 == 1` (B1, two `=`).
#[test]
fn phase_b_row03_strlen_mode_len_mod3_eq1() {
    strlen_mode_ascii("row03 strlen %3==1", 1);
}

/// Row 4 — A1 + `strlen % 3 == 2` (B2, one `=`).
#[test]
fn phase_b_row04_strlen_mode_len_mod3_eq2() {
    strlen_mode_ascii("row04 strlen %3==2", 2);
}

/// Row 5 — A1 + high-bit bytes (E2, signed `char` conversion), all `% 3`
/// classes.
#[test]
fn phase_b_row05_strlen_mode_high_bit_bytes() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for len in 1..=(3 * SAMPLES) {
        let mut p = rng.high_bit(len);
        // Guarantee at least one byte with the high bit set.
        p[len / 2] |= 0x80;
        p.push(0);
        check("row05 strlen high-bit", 0, &p);
    }
}

/// Row 6 — A2 + `size == 1` (single iteration, two `=`).
#[test]
fn phase_b_row06_explicit_size_1() {
    let mut rng = Rng::new(SEED ^ 0x06);
    // Exhaustive over all 256 byte values, plus randomized repeats.
    for b in 0u16..=255 {
        check("row06 size=1 exhaustive", 1, &[b as u8, 0xAA, 0xBB, 0xCC]);
    }
    for _ in 0..SAMPLES {
        let p = rng.bytes(4);
        check("row06 size=1 random", 1, &p);
    }
}

/// Row 7 — A2 + `size == 2` (single iteration, one `=`).
#[test]
fn phase_b_row07_explicit_size_2() {
    let mut rng = Rng::new(SEED ^ 0x07);
    // Exhaustive over all 65536 two-byte payloads.
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            check("row07 size=2 exhaustive", 2, &[a as u8, b as u8, 0x5A, 0x5A]);
        }
    }
    for _ in 0..SAMPLES {
        let p = rng.bytes(4);
        check("row07 size=2 random", 2, &p);
    }
}

/// Row 8 — A2 + `size == 3` (single iteration, no padding).
#[test]
fn phase_b_row08_explicit_size_3() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..(SAMPLES * 64) {
        let p = rng.bytes(4);
        check("row08 size=3 random", 3, &p);
    }
    // Corner triples.
    for t in [
        [0x00u8, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF],
        [0x00, 0x00, 0xFF],
        [0xFF, 0x00, 0x00],
        [0x80, 0x7F, 0x80],
        [0x01, 0x02, 0x03],
        [0xFC, 0x0F, 0xC0],
    ] {
        check("row08 size=3 corner", 3, &t);
    }
}

/// Helper for rows 9/10/11: explicit `size`, many iterations, full byte range.
fn explicit_many(label: &str, rem: usize) {
    let mut rng = Rng::new(SEED ^ (0x100 + rem as u64));
    for k in 2..(2 + SAMPLES) {
        let len = 3 * k + rem;
        assert_eq!(len % 3, rem);
        let p = rng.bytes(len);
        check(label, len as c_int, &p);
    }
}

/// Row 9 — A2 + many iterations, `size % 3 == 0`, full byte range (incl. NULs).
#[test]
fn phase_b_row09_explicit_many_mod3_eq0() {
    explicit_many("row09 explicit %3==0", 0);
}

/// Row 10 — A2 + many iterations, `size % 3 == 1`.
#[test]
fn phase_b_row10_explicit_many_mod3_eq1() {
    explicit_many("row10 explicit %3==1", 1);
}

/// Row 11 — A2 + many iterations, `size % 3 == 2`.
#[test]
fn phase_b_row11_explicit_many_mod3_eq2() {
    explicit_many("row11 explicit %3==2", 2);
}

/// Row 12 — payload that forces all 64 `encode()` outputs, i.e. every arm of
/// the `static char encode(unsigned char)` chain including `u == 62` (`+`) and
/// `u == 63` (`/`).
#[test]
fn phase_b_row12_all_64_encode_arms() {
    // Pack the 6-bit values 0..63 into 48 bytes (64 * 6 bits = 384 bits).
    let mut bits: Vec<u8> = Vec::new();
    for v in 0u8..64 {
        for shift in (0..6).rev() {
            bits.push((v >> shift) & 1);
        }
    }
    assert_eq!(bits.len(), 384);
    let payload: Vec<u8> = bits
        .chunks(8)
        .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | b))
        .collect();
    assert_eq!(payload.len(), 48);

    let (c, r) = both(payload.len() as c_int, &payload);
    // Independently confirm the C really produced all 64 alphabet characters.
    if let Ret::Buf(b) = &c {
        let alphabet: Vec<u8> =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".to_vec();
        for ch in &alphabet {
            assert!(
                b.contains(ch),
                "row12 payload failed to exercise encode() output {:?}",
                *ch as char
            );
        }
    } else {
        panic!("row12: C returned NULL unexpectedly");
    }
    assert_same("row12 all 64 arms", payload.len() as c_int, &payload, c, r);

    // Also sweep every 6-bit value in each of the four output positions.
    for v in 0u8..64 {
        let p = [v << 2, v << 4, v << 6, (v << 2) | 0x03, v, 0xFF];
        check("row12 per-position", 6, &p);
        check("row12 per-position n=4", 4, &p);
    }
}

/// Row 13 — explicit `size` strictly less than the real buffer length: the C
/// must not read or emit the extra bytes.
#[test]
fn phase_b_row13_size_smaller_than_buffer() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for _ in 0..SAMPLES {
        let buf_len = 32 + rng.below(64);
        let p = rng.bytes(buf_len);
        for size in 1..=buf_len {
            check("row13 partial", size as c_int, &p);
        }
    }
}

/// Row 14 — all-`0x00` payload with explicit size (NULs must not terminate).
#[test]
fn phase_b_row14_all_zero_payload() {
    for len in 1..=(3 * SAMPLES) {
        let p = vec![0u8; len + 8];
        check("row14 all zero", len as c_int, &p);
    }
}

/// Row 15 — all-`0xFF` payload (heavy `/` arm).
#[test]
fn phase_b_row15_all_ff_payload() {
    for len in 1..=(3 * SAMPLES) {
        let p = vec![0xFFu8; len + 8];
        check("row15 all 0xFF", len as c_int, &p);
    }
}

/// Row 16 — long payloads (256..=4096) in all three `% 3` classes; checks the
/// allocation sizing and the trailing zero padding of the buffer.
#[test]
fn phase_b_row16_long_payloads() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for base in [256usize, 511, 512, 1000, 1023, 1024, 2048, 4095, 4096] {
        for rem in 0..3usize {
            let len = base + rem;
            let p = rng.bytes(len);
            check("row16 long", len as c_int, &p);
        }
    }
}

/// Row 17 — A1 (`size = 0`) with a NUL in the middle: `strlen` stops early, so
/// only the prefix is encoded.
#[test]
fn phase_b_row17_strlen_mode_embedded_nul() {
    let mut rng = Rng::new(SEED ^ 0x17);
    for prefix in 0..(3 * SAMPLES) {
        let mut p = rng.high_bit(prefix); // no NULs inside the prefix
        p.push(0); // early terminator
        p.extend_from_slice(&rng.bytes(16)); // ignored tail
        p.push(0); // final terminator, keeps strlen in-bounds regardless
        check("row17 embedded NUL", 0, &p);
    }
}

/// Row 18 — exhaustive `size` sweep 1..=64 with randomized payloads.
#[test]
fn phase_b_row18_size_sweep_1_to_64() {
    let mut rng = Rng::new(SEED ^ 0x18);
    for size in 1..=64usize {
        for _ in 0..16 {
            let p = rng.bytes(size + 8);
            check("row18 sweep", size as c_int, &p);
        }
    }
}

// ===========================================================================
// Phase C — error-path differential tests, gated on ERRORS.md
// ===========================================================================

/// ERRORS.md row 1 — `src == NULL` for arbitrary `size` → both return NULL.
#[test]
fn phase_c_row01_null_src() {
    for size in [
        0,
        1,
        2,
        3,
        4,
        63,
        64,
        -1,
        -2,
        -3,
        -4,
        -5,
        -100,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
    ] {
        let (c, r) = both_null(size);
        assert_eq!(
            c,
            Ret::Null,
            "C must reject NULL src (size={size}) but returned {c:?}"
        );
        assert_same("errors row01 NULL src", size, &[], c, r);
    }
}

/// ERRORS.md row 2 — `size == -1` → `calloc(1,3)` succeeds, empty output.
#[test]
fn phase_c_row02_size_minus_1() {
    assert_eq!(alloc_len(-1), 3);
    let p = b"abcdefgh\0";
    let (c, r) = both(-1, p);
    assert_eq!(c, Ret::Buf(vec![0, 0, 0]), "C size=-1 shape");
    assert_same("errors row02 size=-1", -1, p, c, r);
}

/// ERRORS.md row 3 — `size == -2` → `calloc(1,2)` succeeds, empty output.
#[test]
fn phase_c_row03_size_minus_2() {
    assert_eq!(alloc_len(-2), 2);
    let p = b"abcdefgh\0";
    let (c, r) = both(-2, p);
    assert_eq!(c, Ret::Buf(vec![0, 0]), "C size=-2 shape");
    assert_same("errors row03 size=-2", -2, p, c, r);
}

/// ERRORS.md row 4 — `size == -3` → `calloc(1,0)`, non-NULL, zero bytes.
#[test]
fn phase_c_row04_size_minus_3_zero_alloc() {
    assert_eq!(alloc_len(-3), 0);
    let p = b"abcdefgh\0";
    let (c, r) = both(-3, p);
    assert_eq!(c, Ret::Buf(vec![]), "C size=-3 must be a non-NULL 0-byte alloc");
    assert_same("errors row04 size=-3", -3, p, c, r);
}

/// ERRORS.md row 5 — `size == -4` → `n == -1` → `calloc(1, SIZE_MAX)` fails.
#[test]
fn phase_c_row05_size_minus_4_calloc_fails() {
    assert_eq!(alloc_len(-4), -1);
    let p = b"abcdefgh\0";
    let (c, r) = both(-4, p);
    assert_eq!(c, Ret::Null, "C size=-4 must return NULL");
    assert_same("errors row05 size=-4", -4, p, c, r);
}

/// ERRORS.md row 6 — `size == -5` → `n == -2` → `calloc` fails.
#[test]
fn phase_c_row06_size_minus_5_calloc_fails() {
    assert_eq!(alloc_len(-5), -2);
    let p = b"abcdefgh\0";
    let (c, r) = both(-5, p);
    assert_eq!(c, Ret::Null, "C size=-5 must return NULL");
    assert_same("errors row06 size=-5", -5, p, c, r);
}

/// ERRORS.md row 7 — every `size <= -4` whose `n = size*4/3+4` stays negative
/// (i.e. no `int` wraparound) → `calloc(1, (size_t)n)` fails → NULL.
///
/// Note the wraparound carve-out: once `size * 4` overflows `int` (roughly
/// `size < INT_MIN/4`) `n` can come back positive and the call SUCCEEDS. That
/// is the C's behaviour and is asserted here too, driven off `alloc_len`.
#[test]
fn phase_c_row07_all_negative_sizes_below_minus_3() {
    let p = b"abcdefghijklmnop\0";

    // (a) the no-wraparound band: |size*4| fits in an int, so n < 0 => NULL.
    for size in [
        -6i32,
        -7,
        -8,
        -9,
        -10,
        -100,
        -1000,
        -100_000,
        -1_000_000,
        -100_000_000,
        -536_870_911, // > INT_MIN/4, so size*4 does not overflow
    ] {
        assert!(alloc_len(size) < 0, "size={size} should give n<0");
        let (c, r) = both(size, p);
        assert_eq!(
            c,
            Ret::Null,
            "C size={size} (n={}) must return NULL",
            alloc_len(size)
        );
        assert_same("errors row07 big negatives", size, p, c, r);
    }

    // (b) the wraparound band near INT_MIN: n comes back >= 0 and the C
    //     SUCCEEDS. Verify C and Rust agree, and that NULL-ness is exactly
    //     predicted by alloc_len.
    for size in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        i32::MIN + 4,
        i32::MIN / 2,
        i32::MIN / 4,
        -536_870_912, // == INT_MIN/4 exactly: size*4 overflows
    ] {
        let n = alloc_len(size);
        let (c, r) = both(size, p);
        if n < 0 {
            assert_eq!(c, Ret::Null, "size={size} n={n}: expected C NULL");
        }
        assert_same("errors row07 wraparound", size, p, c, r);
    }

    // (c) contiguous negative sweep so no arithmetic corner is missed.
    for size in -2000i32..=-1 {
        let (c, r) = both(size, p);
        assert_same("errors row07 sweep", size, p, c, r);
    }
}

/// ERRORS.md row 8 — `size == INT_MIN`: `INT_MIN * 4` wraps to 0, so
/// `n == 4` and `calloc` SUCCEEDS; the loop guard `0 < INT_MIN` is false.
#[test]
fn phase_c_row08_size_int_min_wraps_to_success() {
    assert_eq!(alloc_len(i32::MIN), 4);
    let p = b"abcdefgh\0";
    let (c, r) = both(i32::MIN, p);
    assert_eq!(
        c,
        Ret::Buf(vec![0, 0, 0, 0]),
        "C size=INT_MIN must return a non-NULL 4-byte zeroed buffer"
    );
    assert_same("errors row08 size=INT_MIN", i32::MIN, p, c, r);
}

/// ERRORS.md row 9 — `size == 0` is NOT an error: length comes from `strlen`.
#[test]
fn phase_c_row09_size_zero_is_strlen_not_error() {
    let mut rng = Rng::new(SEED ^ 0x09);
    for len in 0..64usize {
        let mut p = rng.high_bit(len);
        p.push(0);
        let (c, r) = both(0, &p);
        assert_ne!(c, Ret::Null, "C size=0 with valid src must not return NULL");
        assert_same("errors row09 size=0 strlen", 0, &p, c, r);
    }
}

/// ERRORS.md row 10 — `size == 0` with an empty string: `strlen == 0`, so
/// `n == 4` and zero loop iterations.
#[test]
fn phase_c_row10_size_zero_empty_string() {
    let p = b"\0";
    let (c, r) = both(0, p);
    assert_eq!(c, Ret::Buf(vec![0, 0, 0, 0]), "C size=0 empty-string shape");
    assert_same("errors row10 size=0 empty", 0, p, c, r);
}

// ---------------------------------------------------------------------------
// Generic boundary coverage required by Phase C beyond the table.
// ---------------------------------------------------------------------------

/// NULL pointer combined with the zero/negative/oversized length boundaries.
#[test]
fn phase_c_generic_null_pointer_matrix() {
    for size in [i32::MIN, -4, -3, -1, 0, 1, 3, i32::MAX] {
        let (c, r) = both_null(size);
        assert_eq!(c, Ret::Null);
        assert_same("generic NULL matrix", size, &[], c, r);
    }
}

/// Zero length in both modes, and the "one step past" boundaries around each
/// interesting `size` value the arithmetic distinguishes.
#[test]
fn phase_c_generic_one_step_past_boundaries() {
    let p = b"0123456789abcdefghij\0";
    // Around the sign/allocation boundaries -5,-4,-3,-2,-1,0,1 and around the
    // per-iteration boundaries 3,4 / 6,7.
    for size in [-6i32, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
        let (c, r) = both(size, p);
        assert_same("generic one-step-past", size, p, c, r);
    }
}

/// `int size` has no enum variants, but the FFI boundary still accepts every
/// bit pattern. Sweep a randomized selection of arbitrary `int` values that are
/// safe to drive (`size <= 0`, where the loop never dereferences `src`), so an
/// "out of documented range" scalar is proven to behave identically.
#[test]
fn phase_c_generic_arbitrary_int_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    let p = b"padded payload for safety\0";
    for _ in 0..4096 {
        // Any non-positive int: the C loop body never executes, so this is
        // fully defined for both implementations.
        let raw = rng.next_u64() as u32 as i32;
        let size = if raw > 0 { -raw } else { raw };
        let (c, r) = both(size, p);
        assert_same("generic arbitrary int", size, p, c, r);
    }
}

/// The allocation-size formula itself must agree for every possible `int`,
/// since it decides NULL vs non-NULL. Verified against the C for the sizes we
/// can safely drive, and exhaustively self-consistent for the rest.
#[test]
fn phase_c_generic_alloc_len_formula_agrees_with_c() {
    let p = b"abc\0";
    // For every non-positive size in a wide band, NULL-ness must match, and
    // NULL-ness is exactly determined by alloc_len(size).
    for size in -5000i32..=0 {
        let (c, r) = both(size, p);
        let n = alloc_len(size);
        let expect_null = n < 0;
        if expect_null {
            assert_eq!(c, Ret::Null, "size={size} n={n}: expected C NULL");
        } else {
            assert_ne!(c, Ret::Null, "size={size} n={n}: expected C non-NULL");
        }
        assert_same("generic alloc_len", size, p, c, r);
    }
}

/// Exhaustive belt-and-braces sweep: ALL 2^24 three-byte payloads (covers every
/// possible 4-tuple of `encode()` outputs and every bit position of the
/// b1/b2/b3 -> b4/b5/b6/b7 repacking). Slow (~10^7 FFI round trips), so it is
/// `#[ignore]`d by default; run with
/// `cargo test --release -- --ignored --nocapture`.
#[test]
#[ignore]
fn phase_b_exhaustive_all_three_byte_payloads() {
    for v in 0u32..(1 << 24) {
        let p = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        let (c, r) = both(3, &p);
        if c != r {
            assert_same("exhaustive 3-byte", 3, &p, c, r);
        }
    }
}
