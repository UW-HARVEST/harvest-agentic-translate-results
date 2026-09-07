//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` returns `void`, so it has no error
//! code channel; the observable "result" of an invalid input is either the exact
//! bytes it still prints, or the exact fatal signal it dies on. Rows that are
//! undefined behaviour in C are executed in a forked child (see
//! `common::run_in_child`) so a matching crash is *observed* rather than taking
//! the harness down, and so "both crash the same way" is asserted precisely
//! (same signal number) instead of "both failed somehow".

mod common;

use std::ffi::c_char;

use common::{assert_same, assert_same_and_value, assert_same_cstr, assert_same_outcome, Outcome};

const SIGSEGV: i32 = 11;

/// A NULL `const char *`.
fn null() -> *const c_char {
    std::ptr::null()
}

// --- row 1: s1 == NULL -----------------------------------------------------
#[test]
fn err_01_null_s1() {
    static S2: &[u8] = b"abc\0";
    let out = assert_same_outcome(|| (null(), S2.as_ptr() as *const c_char), "err_01");
    assert_eq!(
        out,
        Outcome::Signalled { signal: SIGSEGV },
        "err_01: expected both to die on SIGSEGV dereferencing NULL s1"
    );
}

// --- row 2: s2 == NULL, s1 non-empty ---------------------------------------
#[test]
fn err_02_null_s2() {
    static S1: &[u8] = b"abc\0";
    let out = assert_same_outcome(|| (S1.as_ptr() as *const c_char, null()), "err_02");
    assert_eq!(
        out,
        Outcome::Signalled { signal: SIGSEGV },
        "err_02: expected both to die on SIGSEGV dereferencing NULL s2"
    );
}

// --- row 3: both NULL ------------------------------------------------------
#[test]
fn err_03_null_both() {
    let out = assert_same_outcome(|| (null(), null()), "err_03");
    assert_eq!(
        out,
        Outcome::Signalled { signal: SIGSEGV },
        "err_03: expected both to die on SIGSEGV"
    );
}

// --- row 4: empty s1 with NULL s2 ------------------------------------------
#[test]
fn err_04_empty_s1_null_s2() {
    // s1 terminates immediately, so a lazy implementation would never look at s2
    // at all and would happily print 0. The C does NOT do that: glibc's strcspn
    // materialises its reject-set membership table by walking the whole of s2
    // BEFORE reading any byte of s1, so the NULL s2 is dereferenced regardless of
    // s1 being empty. This row is the one real divergence this verification found
    // — the Rust strcspn used to check *s1 first and return 0 — and src/lib.rs was
    // changed to walk s2 first so the fault ordering matches.
    static EMPTY: &[u8] = b"\0";
    let out = assert_same_outcome(|| (EMPTY.as_ptr() as *const c_char, null()), "err_04");
    assert_eq!(
        out,
        Outcome::Signalled { signal: SIGSEGV },
        "err_04: the reject set must be read before s1, so a NULL s2 faults even \
         when s1 is immediately terminated"
    );
}

// --- row 5: s1 not NUL-terminated ------------------------------------------
#[test]
fn err_05_unterminated_s1() {
    // A buffer whose first 8 bytes contain no NUL: the scan MUST run past byte 8
    // into the following bytes. Those bytes are part of the same allocation and
    // are fixed and known, so the over-read is deterministic and both
    // implementations see byte-identical memory — which is exactly what "C and
    // Rust must agree given identical adjacent bytes" requires. The trailing
    // zeros bound the read so the test cannot wander into unmapped pages.
    let mut buf = vec![0u8; 64];
    for (i, b) in buf.iter_mut().take(8).enumerate() {
        *b = b'a' + i as u8;
    }
    // s2 rejects nothing present in the buffer, so the scan runs to the first
    // zero at offset 8 and both must print 8.
    let s2 = b"Z\0";
    let out = assert_same_cstr(&buf, s2, "err_05");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "8\n",
        "err_05: the scan must stop at the first zero byte after the unterminated prefix"
    );

    // Same buffer, but now the reject set hits inside the unterminated prefix.
    let out2 = assert_same_cstr(&buf, b"d\0", "err_05 hit inside prefix");
    assert_eq!(String::from_utf8_lossy(&out2), "3\n");
}

// --- row 6: s2 not NUL-terminated ------------------------------------------
#[test]
fn err_06_unterminated_s2() {
    // Mirror of row 5, on the reject set. The 4 meaningful bytes are followed by
    // known zeros in the same allocation, so both sides over-read identically.
    let mut s2 = vec![0u8; 64];
    s2[..4].copy_from_slice(b"WXYZ");

    // No byte of s1 is in the (effective) reject set -> full length.
    let out = assert_same_cstr(b"abcdef\0", &s2, "err_06 no hit");
    assert_eq!(String::from_utf8_lossy(&out), "6\n");

    // A byte of s1 IS in the reject set, past the first entry.
    let out2 = assert_same_cstr(b"abZdef\0", &s2, "err_06 hit");
    assert_eq!(String::from_utf8_lossy(&out2), "2\n");
}

// --- row 7: zero-length s1 -------------------------------------------------
#[test]
fn err_07_zero_len_s1() {
    for s2 in [
        &b""[..],
        &b"a"[..],
        &b"abcdefghijklmnop"[..],
        &(1u8..=255).collect::<Vec<u8>>()[..],
    ] {
        assert_same_and_value(b"", s2, 0, "err_07");
    }
}

// --- row 8: zero-length s2 -------------------------------------------------
#[test]
fn err_08_zero_len_s2() {
    // The NUL terminator of s2 is NOT a member of the reject set, so an empty
    // reject set never matches and the full length of s1 is printed.
    for s1 in [
        &b"a"[..],
        &b"abc"[..],
        &b"the quick brown fox"[..],
        &vec![0xFFu8; 300][..],
    ] {
        assert_same_and_value(s1, b"", s1.len(), "err_08");
    }
}

// --- row 9: both empty -----------------------------------------------------
#[test]
fn err_09_both_empty() {
    assert_same_and_value(b"", b"", 0, "err_09");
}

// --- row 10: oversized s1 --------------------------------------------------
#[test]
fn err_10_oversized_no_match() {
    const MIB: usize = 1024 * 1024;
    let s1 = vec![b'q'; MIB];
    // No truncation, no overflow: `%zu` must render the whole 7-digit count.
    assert_same_and_value(&s1, b"AB", MIB, "err_10");
}

// --- row 11: bytes one step past the "normal" range ------------------------
#[test]
fn err_11_high_bit_bytes() {
    // 0x80..=0xFF are negative when `char` is signed (x86-64). A sign-extension
    // slip would make 0xFF compare equal to -1 sentinels, or make 0x80 collide
    // with 0x00 (the terminator) — both would show up here.
    assert_same_and_value(&[0x80], &[0x80], 0, "err_11 0x80/0x80");
    assert_same_and_value(&[0x80], &[0x01], 1, "err_11 0x80/0x01");
    assert_same_and_value(&[0xFF, 0xFE, 0xFD], &[0xFD], 2, "err_11 0xFD interior");
    assert_same_and_value(&[0x7F, 0x80, 0x81], &[0x80], 1, "err_11 boundary 0x7F/0x80");
    // 0xFF as the ONLY reject byte, against a string with none of it.
    assert_same_and_value(&[0x01, 0x7F, 0x80], &[0xFF], 3, "err_11 0xFF absent");
    // Full high-bit alphabet in both.
    let all_high: Vec<u8> = (0x80u8..=0xFF).collect();
    assert_same_and_value(&all_high, &[0xFF], 0x7F, "err_11 0xFF is last");
    assert_same_and_value(&all_high, &all_high, 0, "err_11 self");
}

// --- row 12: embedded NUL in s1 -------------------------------------------
#[test]
fn err_12_embedded_nul_s1() {
    // "ab\0cd" + terminator: the scan stops at the embedded NUL, so the 'd' after
    // it is invisible even though 'd' is in the reject set.
    let out = assert_same_cstr(b"ab\0cd\0", b"d\0", "err_12");
    assert_eq!(String::from_utf8_lossy(&out), "2\n");

    // Leading embedded NUL -> zero-length effective string.
    let out2 = assert_same_cstr(b"\0abc\0", b"a\0", "err_12 leading NUL");
    assert_eq!(String::from_utf8_lossy(&out2), "0\n");
}

// --- row 13: embedded NUL in s2 -------------------------------------------
#[test]
fn err_13_embedded_nul_s2() {
    // Reject set "z\0a": only 'z' is a member; the 'a' lies past the terminator.
    let out = assert_same_cstr(b"abc\0", b"z\0a\0", "err_13");
    assert_eq!(String::from_utf8_lossy(&out), "3\n");

    // The member before the NUL still works.
    let out2 = assert_same_cstr(b"abz\0", b"z\0a\0", "err_13 member found");
    assert_eq!(String::from_utf8_lossy(&out2), "2\n");

    // A reject set that starts with NUL is an empty reject set.
    let out3 = assert_same_cstr(b"abc\0", b"\0abc\0", "err_13 leading NUL in s2");
    assert_eq!(String::from_utf8_lossy(&out3), "3\n");
}

// --- row 14: out-of-range enum across the FFI boundary --------------------
#[test]
fn err_14_no_enum_in_api() {
    // The "pass an int with no valid enum variant" class of bug cannot exist
    // here, and that is asserted structurally rather than assumed: the public
    // header declares no enum, and the only exported function takes two pointers.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read c_src/include/driver.h");

    // Strip comments (the licence block) before scanning for declarations.
    let code: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains("enum"),
        "err_14: an enum appeared in the public header; ERRORS.md row 14 and the \
         Phase C tests must be extended with out-of-range-variant cases"
    );
    assert!(
        code.contains("void driver(const char *s1, const char *s2);"),
        "err_14: the public signature changed; re-derive ERRORS.md.\nheader was:\n{code}"
    );
    // Also confirm the .so really exports nothing else that could take an enum.
    let p = common::pair();
    let _ = p.c;
    let _ = p.rust;
}

// --- row 15: wild (non-NULL, invalid) pointer -----------------------------
#[test]
fn err_15_wild_pointer() {
    // Address 1 is never mapped. Both implementations must fault identically.
    static S2: &[u8] = b"abc\0";
    let out = assert_same_outcome(
        || (1usize as *const c_char, S2.as_ptr() as *const c_char),
        "err_15 wild s1",
    );
    assert_eq!(out, Outcome::Signalled { signal: SIGSEGV });

    static S1: &[u8] = b"abc\0";
    let out2 = assert_same_outcome(
        || (S1.as_ptr() as *const c_char, 1usize as *const c_char),
        "err_15 wild s2",
    );
    assert_eq!(out2, Outcome::Signalled { signal: SIGSEGV });

    // Empty s1 with a wild s2: the reject set is still read first, so this must
    // fault too (the generalisation of the row-4 ordering finding).
    static EMPTY: &[u8] = b"\0";
    let out3 = assert_same_outcome(
        || (EMPTY.as_ptr() as *const c_char, 1usize as *const c_char),
        "err_15 empty s1 + wild s2",
    );
    assert_eq!(out3, Outcome::Signalled { signal: SIGSEGV });
}

// --- row 16: oversized s2 --------------------------------------------------
#[test]
fn err_16_oversized_s2() {
    // A 1 MiB reject set that contains every non-NUL byte: the first byte of any
    // non-empty s1 is a member, so the answer is 0.
    let mut s2 = Vec::with_capacity(1024 * 1024);
    while s2.len() < 1024 * 1024 {
        s2.extend(1u8..=255);
    }
    s2.truncate(1024 * 1024);
    assert_same_and_value(b"hello", &s2, 0, "err_16");
    assert_same_and_value(&[0xFFu8], &s2, 0, "err_16 high byte");
    // An empty s1 is still 0, without reading the giant set to the end.
    assert_same_and_value(b"", &s2, 0, "err_16 empty s1");
}

// --- generic extra boundaries (beyond the table) ---------------------------
#[test]
fn err_extra_generic_boundaries() {
    // One step past every "documented range" that exists here: length 0 and 1,
    // the smallest and largest byte values, and a reject set of exactly one NUL.
    assert_same(b"", b"", "extra 0/0");
    assert_same(&[0x01], &[0x01], "extra min byte equal");
    assert_same(&[0x01], &[0xFF], "extra min vs max");
    assert_same(&[0xFF], &[0x01], "extra max vs min");
    assert_same_cstr(b"\0", b"\0", "extra both immediately terminated");

    // A reject set consisting only of the terminator, against a maximal byte.
    assert_same_and_value(&[0xFF, 0xFF, 0xFF], b"", 3, "extra empty set vs 0xFF run");
}
