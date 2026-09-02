//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries
//! (null pointer, zero length, oversized length, one step past every valid
//! range). Both `.so`s must return the SAME sentinel, not merely "both failed".

mod common;

use common::*;
use std::ffi::c_char;

// --- Row 1: NULL pointer ---------------------------------------------------

#[test]
fn err_01_null_pointer() {
    // `if (src && *src)` short-circuits => return NULL. Passing NULL through
    // the FFI boundary must NOT dereference in either implementation.
    let out = assert_same_raw(std::ptr::null::<c_char>(), 14);
    assert_eq!(out, Outcome::Null, "NULL input must yield the NULL sentinel");
}

// --- Row 2: empty string ---------------------------------------------------

#[test]
fn err_02_empty_string() {
    let out = assert_same(b"");
    assert_eq!(out, Outcome::Null, "empty string must yield the NULL sentinel");
}

// --- Rows 3 & 4: allocation failure ---------------------------------------

#[test]
fn err_03_calloc_success_path_shape() {
    // The `calloc` FAILURE branch is driven by real fault injection in
    // tests/alloc_contract.rs (part3_calloc_failure), which arms the interposed
    // calloc to return NULL for exactly strlen+1+13 bytes.
    //
    // What is asserted here is the complementary success-path shape: a non-NULL
    // pointer to an identically sized, calloc-zeroed region.
    let out = assert_same(b"QUJD");
    match out {
        Outcome::Buffer { full, .. } => {
            assert_eq!(full.len(), 4 + 1 + 13, "calloc size must be strlen+1+13");
            // Tail beyond the decoded bytes must still be calloc-zeroed.
            assert!(full[3..].iter().all(|&b| b == 0));
        }
        Outcome::Null => panic!("unexpected NULL on the success path"),
    }
}

#[test]
fn err_04_repeated_calls_no_double_free() {
    // The `malloc` FAILURE branch (free(dest); return NULL) is driven by real
    // fault injection in tests/alloc_contract.rs
    // (part4_malloc_failure_frees_dest), which counts the frees to prove the
    // cleanup happens.
    //
    // What is asserted here: repeated calls are byte-identical and no double
    // free / use-after-free occurs — the harness frees every returned pointer,
    // so a mismatch in ownership semantics would abort under the allocator.
    for _ in 0..200 {
        assert_same(b"QUJDRUZH");
    }
}

// --- Row 5: all-non-base64 input returns a buffer, NOT NULL ---------------

#[test]
fn err_05_all_non_base64_returns_empty_not_null() {
    // This is the anti-blind-spot row: an "invalid" input that the C
    // deliberately does NOT reject. It must not become a NULL in Rust.
    for input in [
        &b"!"[..],
        b"!!!",
        b"   ",
        b"\n\t\r",
        b"---",
        b"___",
        b"\x80\x81\xff",
        b"@[`{",
        b".,;:",
        b"\\|~^",
    ] {
        let out = assert_same(input);
        match out {
            Outcome::Buffer { c_strlen, full, .. } => {
                assert_eq!(
                    c_strlen, 0,
                    "input {input:?} must decode to the empty string, not NULL"
                );
                assert!(full.iter().all(|&b| b == 0), "buffer must be zero-filled");
            }
            Outcome::Null => panic!("input {input:?}: C returns non-NULL, so Rust must too"),
        }
    }
}

// --- Row 6: is_base64 rejects every byte outside the alphabet -------------

#[test]
fn err_06_is_base64_rejects_all_non_alphabet_bytes() {
    // Exhaustive over all 255 non-NUL bytes. For each rejected byte, wrapping
    // it around a known-good group must produce exactly the group's own output
    // (the byte is dropped), and this must hold identically in both libs.
    let baseline = assert_same(b"QUJD");
    for b in 1u16..=255 {
        let byte = b as u8;
        let accepted = byte.is_ascii_uppercase()
            || byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || byte == b'+'
            || byte == b'/'
            || byte == b'=';
        // Differential check regardless of class.
        let sandwiched = vec![byte, b'Q', b'U', byte, b'J', b'D', byte];
        assert_same(&sandwiched);
        if !accepted {
            // Dropped => decoded payload identical to the bare group, though
            // the allocation (and hence the compared region) is longer.
            let out = assert_same(&sandwiched);
            if let (
                Outcome::Buffer { full: bl, c_strlen: bs, .. },
                Outcome::Buffer { full: cur, c_strlen: cs, .. },
            ) = (&baseline, &out)
            {
                assert_eq!(bs, cs, "byte 0x{byte:02x} must be ignored by is_base64");
                assert_eq!(
                    &bl[..*bs],
                    &cur[..*cs],
                    "byte 0x{byte:02x} must not affect the decoded payload"
                );
            }
        }
    }
}

// --- Row 7: decode() fall-through: '/' and '=' both decode to 63 ---------

#[test]
fn err_07_decode_fallthrough_slash_and_equals_both_63() {
    // The C quirk: decode('=') hits the same `return 63` as decode('/').
    // "///A" and "==="+... must therefore agree in the sextets that are
    // actually emitted. Rather than assert the quirk from the outside, drive
    // both libs on every input that reaches the fall-through.
    for a in [b'/', b'='] {
        for b in [b'/', b'='] {
            for c in [b'/', b'='] {
                for d in [b'/', b'='] {
                    assert_same(&[a, b, c, d]);
                    assert_same(&[a, b, c, d, a, b, c, d]);
                }
            }
        }
    }
    // First sextet is position-independent of the '='-suppression logic, so
    // "/AAA" and "=AAA" must produce the same first output byte in both libs.
    let s = assert_same(b"/AAA");
    let e = assert_same(b"=AAA");
    if let (Outcome::Buffer { full: fs, .. }, Outcome::Buffer { full: fe, .. }) = (&s, &e) {
        assert_eq!(fs[0], fe[0], "decode('/') and decode('=') both return 63");
        assert_eq!(fs[0], 63 << 2, "63 << 2 == 0xFC");
    }
}

// --- Rows 8 & 9: '=' suppression branches --------------------------------

#[test]
fn err_08_equals_at_c3_suppresses_byte() {
    // c3 == '=' skips the 2nd output byte of the group (lib.c:98).
    for &(input, _label) in &[
        (&b"AA=A"[..], "c3 only"),
        (b"AA==", "c3 and c4"),
        (b"QU=D", "c3 mid-alphabet"),
        (b"QU=DQUJD", "c3 in first of two groups"),
        (b"QUJDQU=D", "c3 in second of two groups"),
        (b"AA=AAA=AAA=A", "c3 in three consecutive groups"),
    ] {
        let out = assert_same(input);
        assert!(matches!(out, Outcome::Buffer { .. }));
    }
    // Randomized: '=' pinned at every group offset 2.
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let groups = rng.range(1, 20);
        let mut v = from_set(&mut rng, ALPHABET, groups * 4);
        for g in 0..groups {
            v[g * 4 + 2] = b'=';
        }
        assert_same(&v);
    }
}

#[test]
fn err_09_equals_at_c4_suppresses_byte() {
    // c4 == '=' skips the 3rd output byte of the group (lib.c:102).
    for input in [
        &b"AAA="[..],
        b"QUJD",
        b"QUJ=",
        b"QUJ=QUJD",
        b"QUJDQUJ=",
        b"AAA=AAA=AAA=",
    ] {
        assert_same(input);
    }
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS {
        let groups = rng.range(1, 20);
        let mut v = from_set(&mut rng, ALPHABET, groups * 4);
        for g in 0..groups {
            v[g * 4 + 3] = b'=';
        }
        assert_same(&v);
    }
    // Both suppressions together at every group.
    for _ in 0..ITERS {
        let groups = rng.range(1, 20);
        let mut v = from_set(&mut rng, ALPHABET, groups * 4);
        for g in 0..groups {
            v[g * 4 + 2] = b'=';
            v[g * 4 + 3] = b'=';
        }
        assert_same(&v);
    }
}

// --- Rows 10-12: truncated trailing groups (l % 4 != 0) -----------------

fn truncated_row(seed_xor: u64, m: usize) {
    let mut rng = Rng::new(SEED ^ seed_xor);
    for _ in 0..ITERS {
        let groups = rng.range(0, 20);
        let len = groups * 4 + m;
        let v = from_set(&mut rng, ALPHABET_EQ, len);
        assert_same(&v);
    }
}

#[test]
fn err_10_trailing_group_len_mod4_1() {
    // Only buf[k] is read; c2/c3/c4 keep their 'A' defaults, so the C emits
    // three bytes for a one-character group. Replicate exactly.
    for input in [&b"A"[..], b"Z", b"a", b"z", b"0", b"9", b"+", b"/", b"="] {
        assert_same(input);
    }
    let out = assert_same(b"Q");
    if let Outcome::Buffer { c_strlen, .. } = out {
        // decode('Q')=16 -> 0x40 0x00 0x00 ; strlen stops at the first NUL.
        assert_eq!(c_strlen, 1);
    }
    truncated_row(10, 1);
}

#[test]
fn err_11_trailing_group_len_mod4_2() {
    for input in [&b"QU"[..], b"AA", b"=="] {
        assert_same(input);
    }
    truncated_row(11, 2);
}

#[test]
fn err_12_trailing_group_len_mod4_3() {
    for input in [&b"QUJ"[..], b"AAA", b"==="] {
        assert_same(input);
    }
    truncated_row(12, 3);
}

// --- Row 13: exhaustive single byte --------------------------------------

#[test]
fn err_13_exhaustive_single_byte() {
    // Every byte value 0x01..0xFF as a one-character input. Also covers
    // "one step past" each decode() range boundary.
    for b in 1u16..=255 {
        assert_same(&[b as u8]);
    }
    // The documented boundary set, spelled out so a regression names itself.
    for &(ch, _desc) in &[
        (b'@', "one below 'A'"),
        (b'[', "one above 'Z'"),
        (b'`', "one below 'a'"),
        (b'{', "one above 'z'"),
        (b'/', "one below '0'"),
        (b':', "one above '9'"),
        (b'*', "one below '+'"),
        (b',', "one above '+'"),
        (b'.', "one below '/'"),
    ] {
        assert_same(&[ch]);
        assert_same(&[ch, ch]);
        assert_same(&[ch, ch, ch]);
        assert_same(&[ch, ch, ch, ch]);
    }
}

// --- Row 14: oversized input --------------------------------------------

#[test]
fn err_14_oversized_input_1mib() {
    // No length check exists in the C at all. 1 MiB is far below INT_MAX so
    // the `int l = strlen(src) + 1` arithmetic is well defined in both libs.
    let mut rng = Rng::new(SEED ^ 14);
    let big = from_set(&mut rng, ALPHABET_EQ, 1 << 20);
    assert_same(&big);

    // Same size but entirely noise => filtered length 0.
    let noise = from_set(&mut rng, NOISE, 1 << 20);
    assert_same(&noise);

    // Same size, all '=' => both suppression branches on every group.
    assert_same(&vec![b'='; 1 << 20]);
}

// --- Generic C-API boundaries (beyond the table) -------------------------

#[test]
fn err_generic_zero_and_one_length_boundaries() {
    // zero length (empty string) => NULL sentinel, already row 2; re-assert
    // next to the length-1 boundary so the transition is covered in one place.
    assert_eq!(assert_same(b""), Outcome::Null);
    for b in [b'A', b'Z', b'a', b'z', b'0', b'9', b'+', b'/', b'=', b'!', 0x80, 0xff] {
        assert!(matches!(assert_same(&[b]), Outcome::Buffer { .. }));
    }
}

#[test]
fn err_generic_no_enum_parameters_exist() {
    // The public API takes a single `const char *` and no enums or flags, so
    // there is no out-of-range enum value to smuggle across the FFI boundary
    // (see ERRORS.md). The nearest analogue is an out-of-range *character*
    // value, i.e. a byte with no valid base64 meaning — covered exhaustively
    // here as the FFI-level equivalent, for every byte and in every position
    // of a 4-character group.
    for b in 1u16..=255 {
        let byte = b as u8;
        for pos in 0..4 {
            let mut group = [b'A'; 4];
            group[pos] = byte;
            assert_same(&group);
        }
    }
}

#[test]
fn err_generic_repeated_calls_are_stateless() {
    // No global state in the C, so repeated and interleaved calls must be
    // independent. A divergence here would reveal hidden state in the Rust
    // translation (e.g. a cached buffer).
    let inputs: [&[u8]; 6] = [b"QUJD", b"", b"!!!", b"AA==", b"////", b"Zg"];
    for _ in 0..100 {
        for i in inputs {
            assert_same(i);
        }
    }
}
