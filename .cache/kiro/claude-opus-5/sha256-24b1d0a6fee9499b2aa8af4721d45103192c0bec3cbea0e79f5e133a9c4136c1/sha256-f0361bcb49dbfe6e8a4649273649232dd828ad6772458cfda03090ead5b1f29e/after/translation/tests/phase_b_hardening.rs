//! Phase B/C hardening — checks the NUL-terminated comparison cannot see.
//!
//! 1. Byte-for-byte comparison of the ENTIRE `calloc`'d buffer, including every
//!    byte past the terminator. The C never writes a NUL explicitly; it relies
//!    on `calloc` zeroing the tail. A translation that wrote its own terminator,
//!    or used `malloc` + partial fill, would pass every NUL-prefix test and fail
//!    here.
//! 2. The `strlen`-result-truncated-to-`int` path, including the case where the
//!    truncation flips the sign.

mod common;

use common::{cap_bytes, pair, Outcome, Rng};

// ---------------------------------------------------------------------------
// Whole-buffer parity, including the zero tail.
// ---------------------------------------------------------------------------

#[test]
fn full_buffer_parity_exhaustive_lengths() {
    let mut rng = Rng::new(0xBEEF_0001);
    for len in 1..=300usize {
        for _ in 0..6 {
            let bytes = rng.bytes(len);
            pair().assert_same_full_buffer(len as i32, Some(&bytes), "full buffer sweep");
        }
        // Uniform min/max buffers too.
        pair().assert_same_full_buffer(len as i32, Some(&vec![0u8; len]), "full buffer 0x00");
        pair().assert_same_full_buffer(len as i32, Some(&vec![0xFFu8; len]), "full buffer 0xFF");
    }
}

#[test]
fn full_buffer_tail_is_zero_in_both_and_length_matches_c_formula() {
    let mut rng = Rng::new(0xBEEF_0002);
    for len in 1..=120usize {
        let bytes = rng.bytes(len);
        let cap = cap_bytes(len as i32);
        let written = 4 * ((len + 2) / 3);
        assert!(written <= cap, "C formula sanity: len={len}");

        for imp in [&pair().c, &pair().rs] {
            let buf = imp
                .call_full_buffer(len as i32, Some(&bytes))
                .unwrap_or_else(|| panic!("{}: unexpected NULL at len={len}", imp.name));
            assert_eq!(buf.len(), cap, "{}: capacity mismatch at len={len}", imp.name);
            assert!(
                buf[written..].iter().all(|&b| b == 0),
                "{}: bytes past the written region must be zero (len={len})",
                imp.name
            );
            assert!(
                buf[..written].iter().all(|&b| b != 0),
                "{}: no NUL may appear inside the encoded region (len={len})",
                imp.name
            );
        }
    }
}

#[test]
fn full_buffer_parity_strlen_mode_exhaustive_lengths() {
    let mut rng = Rng::new(0xBEEF_0003);
    for len in 1..=300usize {
        let cap = cap_bytes(len as i32);
        let written = 4 * ((len + 2) / 3);
        for _ in 0..6 {
            let mut bytes = rng.nonzero_bytes(len);
            bytes.push(0);
            let c = pair().c.call_full_buffer_with_cap(0, Some(&bytes), cap);
            let r = pair().rs.call_full_buffer_with_cap(0, Some(&bytes), cap);
            assert_eq!(c, r, "strlen-mode full buffer divergence at strlen={len}");
            let buf = c.expect("strlen mode should succeed");
            assert!(
                buf[written..].iter().all(|&b| b == 0),
                "strlen-mode tail must be zero at strlen={len}"
            );
        }
    }
}

#[test]
fn full_buffer_parity_negative_and_zero_size() {
    // The accepted negative sizes (-3..=-1) return an untouched zeroed buffer;
    // every byte of it must match, not just the empty NUL-prefix.
    let src = b"payload\0";
    for size in [-1i32, -2, -3] {
        pair().assert_same_full_buffer(size, Some(src), "full buffer negative");
    }
    // size == 0 in strlen mode: buffer capacity is derived from strlen(src),
    // not from the `size` argument, so the capacity is supplied explicitly.
    for s in [&b"\0"[..], &b"a\0"[..], &b"ab\0"[..], &b"abc\0"[..], &b"abcd\0"[..]] {
        let n = s.len() as i32 - 1; // strlen(src)
        let cap = cap_bytes(n);
        let c = pair().c.call_full_buffer_with_cap(0, Some(s), cap);
        let r = pair().rs.call_full_buffer_with_cap(0, Some(s), cap);
        assert_eq!(c, r, "full buffer strlen mode, src={s:?}");
        assert_eq!(
            c.as_ref().map(|v| v.len()),
            Some(cap),
            "strlen-mode capacity must follow the C formula, src={s:?}"
        );
        // The tail past the encoded region must be zero in both.
        let written = if n == 0 { 0 } else { 4 * ((n as usize + 2) / 3) };
        for (name, buf) in [("C", &c), ("Rust", &r)] {
            let buf = buf.as_ref().unwrap();
            assert!(
                buf[written..].iter().all(|&b| b == 0),
                "{name}: strlen-mode tail must be zero, src={s:?}"
            );
        }
    }
    // INT_MIN: cap wraps to exactly 4; all four bytes must be zero in both.
    let c = pair().c.call_full_buffer(i32::MIN, Some(src));
    let r = pair().rs.call_full_buffer(i32::MIN, Some(src));
    assert_eq!(c, Some(vec![0u8; 4]), "C: INT_MIN yields a 4-byte zeroed buffer");
    assert_eq!(c, r, "full buffer INT_MIN");
}

// ---------------------------------------------------------------------------
// `strlen` truncated to `int`.
//
// `size = strlen(src)` assigns a `size_t` into an `int`. For a string whose
// length is >= 2^31 the truncation flips the sign, which completely changes
// which branch the rest of the function takes. This is the single most
// translation-sensitive line in the library, so it is exercised for real.
//
// 2 GiB of memory; skipped automatically if the allocation is refused.
// ---------------------------------------------------------------------------

#[test]
fn strlen_truncation_to_int_sign_flip() {
    const LEN: usize = 0x8000_0000; // strlen == 2^31 -> (int)2^31 == INT_MIN

    let mut src = match std::panic::catch_unwind(|| {
        let mut v: Vec<u8> = Vec::new();
        v.try_reserve_exact(LEN + 1).map_err(|_| ())?;
        v.resize(LEN, b'A');
        v.push(0);
        Ok::<Vec<u8>, ()>(v)
    }) {
        Ok(Ok(v)) => v,
        _ => {
            eprintln!("skipping strlen_truncation_to_int_sign_flip: cannot allocate 2 GiB");
            return;
        }
    };
    assert_eq!(src.len(), LEN + 1);

    // strlen(src) == 2^31, truncated to int == INT_MIN. cap = (INT_MIN*4)/3+4,
    // and INT_MIN*4 wraps to 0, so cap == 4 and the loop never runs: the
    // 2 GiB string encodes to the EMPTY string in both implementations.
    let c = pair().c.call_full_buffer_with_cap(0, Some(&src), 4);
    let r = pair().rs.call_full_buffer_with_cap(0, Some(&src), 4);
    assert_eq!(c, r, "strlen truncation (2^31) divergence");
    assert_eq!(
        c,
        Some(vec![0u8; 4]),
        "a 2 GiB string must encode to an empty 4-byte buffer via int truncation"
    );

    // One byte shorter: strlen == 2^31 - 1 == INT_MAX. cap wraps to 3, calloc
    // succeeds, and the loop then runs over all 2 GiB writing ~2.8 GiB into a
    // 3-byte buffer -- ERRORS.md row 7, C-side UB. Not called.

    // One byte shorter still, and inside the safe calloc-failure band: shrink
    // to 0x2000_0000 so strlen == 0x2000_0000, cap < 0, both return NULL.
    src.truncate(0x2000_0000);
    src.push(0);
    let c_null = pair().c.call_nullness(0, Some(&src));
    let r_null = pair().rs.call_nullness(0, Some(&src));
    assert!(c_null, "C: strlen == 0x2000_0000 must overflow cap and return NULL");
    assert_eq!(c_null, r_null, "strlen == 0x2000_0000 divergence");

    // And a length just below the overflow threshold, where it all succeeds.
    src.truncate(0x1000_0001);
    src.push(0);
    let c = pair().c.call(0, Some(&src));
    let r = pair().rs.call(0, Some(&src));
    assert_ne!(c, Outcome::Null, "strlen == 0x1000_0001 should succeed");
    assert_eq!(c, r, "strlen == 0x1000_0001 divergence");
}
