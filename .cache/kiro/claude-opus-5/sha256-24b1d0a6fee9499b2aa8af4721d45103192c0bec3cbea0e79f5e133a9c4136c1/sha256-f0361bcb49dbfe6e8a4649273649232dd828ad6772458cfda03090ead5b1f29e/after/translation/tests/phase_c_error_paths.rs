//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition and asserts the two
//! `.so` files return the SAME sentinel (`NULL` vs a specific non-NULL result),
//! not merely "both failed somehow".

mod common;

use common::{pair, Outcome};

// ---------------------------------------------------------------------------
// Rows 1 + 8: `src == NULL` -> NULL, for every interesting `size`.
// ---------------------------------------------------------------------------

#[test]
fn err_row1_null_src_every_size() {
    let sizes = [
        0i32,
        1,
        2,
        3,
        -1,
        -2,
        -3,
        42,
        -42,
        0x1FFF_FFFF,
        0x2000_0000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    for size in sizes {
        let c = pair().c.call(size, None);
        let r = pair().rs.call(size, None);
        assert_eq!(c, Outcome::Null, "C must return NULL for src=NULL, size={size}");
        assert_eq!(r, Outcome::Null, "Rust must return NULL for src=NULL, size={size}");
        assert_eq!(c, r, "row1 NULL src, size={size}");
    }
    // Sweep a wide band of sizes to be sure the NULL check really precedes
    // every other branch.
    for size in -512i32..=512 {
        pair().assert_same(size, None, "row1 NULL src sweep");
    }
}

// ---------------------------------------------------------------------------
// Rows 2 + 10: signed `int` overflow in `size*4/3+4` -> huge calloc -> NULL.
// ---------------------------------------------------------------------------

#[test]
fn err_row2_calloc_failure_int_overflow() {
    let src = b"payload\0";
    // 0x2000_0000 * 4 == 0x8000_0000, which overflows a signed int to a
    // negative value; sign-extended to size_t this is an unsatisfiable
    // allocation, so `calloc` fails and the function returns NULL. The whole
    // band [0x2000_0000, 0x3FFF_FFFC] behaves this way (cap <= -1), and the
    // read loop is never reached, so `src` may stay tiny.
    for size in [
        0x2000_0000i32,
        0x2000_0001,
        0x2555_5555,
        0x3000_0000,
        0x3FFF_0000,
        0x3FFF_FFF0,
        0x3FFF_FFFC,
    ] {
        let c_null = pair().c.call_nullness(size, Some(src));
        let r_null = pair().rs.call_nullness(size, Some(src));
        assert!(c_null, "C must return NULL (calloc failure) for size={size:#x}");
        assert!(r_null, "Rust must return NULL (calloc failure) for size={size:#x}");
        assert_eq!(c_null, r_null, "row2 calloc failure, size={size:#x}");
    }
}

// ---------------------------------------------------------------------------
// Rows 3 + 9: `size == 0` is the strlen mode, NOT an error.
// ---------------------------------------------------------------------------

#[test]
fn err_row3_size_zero_is_strlen_not_error() {
    // Empty string: strlen == 0, loop never runs, buffer is 4 zeroed bytes.
    let empty = [0u8];
    let c = pair().c.call(0, Some(&empty));
    let r = pair().rs.call(0, Some(&empty));
    assert_ne!(c, Outcome::Null, "size=0 on \"\" must NOT be an error in C");
    assert_ne!(r, Outcome::Null, "size=0 on \"\" must NOT be an error in Rust");
    assert_eq!(c, Outcome::Str(Vec::new()), "C: empty output");
    assert_eq!(r, Outcome::Str(Vec::new()), "Rust: empty output");

    // Non-empty: must silently substitute strlen(src).
    for s in [
        &b"a\0"[..],
        &b"ab\0"[..],
        &b"abc\0"[..],
        &b"abcd\0"[..],
        &b"the quick brown fox\0"[..],
    ] {
        let body = &s[..s.len() - 1];
        let c0 = pair().c.call(0, Some(s));
        let r0 = pair().rs.call(0, Some(s));
        assert_ne!(c0, Outcome::Null, "size=0 must not error, src={s:?}");
        assert_eq!(c0, r0, "row3 strlen substitution, src={s:?}");
        assert_eq!(
            c0,
            pair().c.call(body.len() as i32, Some(body)),
            "size=0 must equal size=strlen(src)"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 4: negative `size`. Contrary to what the loop condition alone suggests,
// only -1, -2 and -3 are accepted: for size <= -4 the capacity expression
// `size*4/3 + 4` is <= -1, sign-extends to a huge size_t, and the calloc fails.
// Verified empirically against the C over -64..=-1.
// ---------------------------------------------------------------------------

#[test]
fn err_row4_negative_size_accept_reject_bands() {
    let src = b"some data here\0";

    // Accepted band: non-NULL, empty output (the loop never iterates).
    for size in [-1i32, -2, -3] {
        let c = pair().c.call(size, Some(src));
        let r = pair().rs.call(size, Some(src));
        assert_ne!(c, Outcome::Null, "C: size={size} must be accepted");
        assert_eq!(
            c,
            Outcome::Str(Vec::new()),
            "C: size={size} must yield an empty string"
        );
        assert_eq!(c, r, "row4 accepted negative size={size}");
    }

    // Rejected band: calloc failure -> NULL.
    for size in [-4i32, -5, -6, -7, -8, -100, -12345, -1_000_000] {
        let c_null = pair().c.call_nullness(size, Some(src));
        let r_null = pair().rs.call_nullness(size, Some(src));
        assert!(c_null, "C: size={size} must return NULL (cap <= -1)");
        assert!(r_null, "Rust: size={size} must return NULL (cap <= -1)");
    }

    // Full sweep of the non-wrapping negative range: both must agree on every
    // single value, and the accept/reject split must be exactly at -4/-3.
    for size in -100_000i32..0 {
        let c_null = pair().c.call_nullness(size, Some(src));
        let r_null = pair().rs.call_nullness(size, Some(src));
        assert_eq!(c_null, r_null, "row4 sweep divergence at size={size}");
        assert_eq!(
            c_null,
            size <= -4,
            "row4 unexpected C decision at size={size}"
        );
    }
    // And byte-compare the accepted values.
    for size in -3i32..0 {
        pair().assert_same(size, Some(src), "row4 accepted byte-compare");
    }
}

// ---------------------------------------------------------------------------
// Row 5: INT_MIN wraps `size*4` to exactly 0 -> tiny 4-byte allocation.
// ---------------------------------------------------------------------------

#[test]
fn err_row5_int_min_wraps_to_tiny_alloc() {
    let src = b"data\0";
    let c = pair().c.call(i32::MIN, Some(src));
    let r = pair().rs.call(i32::MIN, Some(src));
    assert_ne!(
        c,
        Outcome::Null,
        "C: INT_MIN wraps size*4 to 0, so calloc(1, 4) succeeds"
    );
    assert_eq!(c, Outcome::Str(Vec::new()), "C: INT_MIN yields an empty string");
    assert_eq!(c, r, "row5 INT_MIN");

    // Neighbours of INT_MIN, where the wrap lands on other values.
    for size in [i32::MIN + 1, i32::MIN + 2, i32::MIN + 3, i32::MIN / 2] {
        pair().assert_same(size, Some(src), "row5 INT_MIN neighbourhood");
    }
}

// ---------------------------------------------------------------------------
// Row 6: large negative `size` wraps to a large POSITIVE capacity.
// ---------------------------------------------------------------------------

#[test]
fn err_row6_large_negative_size() {
    let src = b"data\0";
    // Wrapping makes the accept/reject decision value-dependent, so it is
    // compared band-for-band against the C rather than predicted.
    for size in [
        -2_000_000_000i32,
        -1_610_612_736,
        -1_500_000_000,
        -1_073_741_824,
        -1_000_000_000,
        -600_000_000,
        -536_870_912,
        -536_870_911,
        -400_000_000,
        -300_000_000,
        -100_000_000,
    ] {
        let c = pair().c.call(size, Some(src));
        let r = pair().rs.call(size, Some(src));
        assert_eq!(c, r, "row6 large negative size={size}");
        // When accepted, the loop still never runs, so the result is empty.
        if c != Outcome::Null {
            assert_eq!(
                c,
                Outcome::Str(Vec::new()),
                "accepted large negative size must yield an empty string"
            );
        }
    }
    // Sweep the wrap boundaries in coarse steps: every 2^20 across the whole
    // negative range, so both sides must agree on every band transition.
    let mut size = i32::MIN;
    loop {
        pair().assert_same_nullness(size, Some(src), "row6 negative band sweep");
        match size.checked_add(1 << 20) {
            Some(next) if next < 0 => size = next,
            _ => break,
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11: one step BELOW the overflow threshold - allocation-decision parity.
// ---------------------------------------------------------------------------

#[test]
fn err_row11_one_below_overflow_threshold() {
    // 0x1FFF_FFFF * 4 == 0x7FFF_FFFC, still positive, so cap is ~683 MiB and
    // the allocation is genuinely attempted AND the read loop runs. That
    // demands a real 0x1FFF_FFFF-byte source buffer, otherwise the C would
    // read out of bounds. Only NULL-ness is compared; materialising two
    // 683 MiB outputs adds nothing over the capacity-math parity being tested.
    let big = vec![0u8; 0x1FFF_FFFF];
    pair().assert_same_nullness(0x1FFF_FFFF, Some(&big), "row11 at threshold-1");
    pair().assert_same_nullness(0x1FFF_FFFE, Some(&big), "row11 at threshold-2");
    drop(big);

    // A smaller but still large positive size, fully comparable.
    let mid = vec![0x5Au8; 0x0100_0001];
    pair().assert_same(0x0100_0001, Some(&mid), "row11 16 MiB comparable");
}

// ---------------------------------------------------------------------------
// Generic FFI boundary conditions.
//
// Row 12 of ERRORS.md is N/A: the ABI is `char *encode_base64(int, const char *)`
// with no enum/flag/struct parameter, so there is no "invalid variant" to pass.
// The `int` parameter's full equivalence-class set is covered instead: 0, +, -,
// INT_MIN, INT_MAX, and one step either side of the overflow threshold. The
// tests below sweep the remaining generic boundaries.
// ---------------------------------------------------------------------------

#[test]
fn boundary_zero_and_one_past_every_documented_edge() {
    let src = b"0123456789abcdef\0";
    let body = &src[..src.len() - 1];

    // size == 0 (mode switch), and one step either side.
    for size in [-1i32, 0, 1] {
        pair().assert_same(size, Some(src), "boundary around 0");
    }

    // size exactly at, one below, and one above the real buffer length.
    let n = body.len() as i32;
    for size in [n - 1, n] {
        pair().assert_same(size, Some(body), "boundary at buffer length");
    }
    // `n + 1` would read one byte past `body`; use the NUL-terminated buffer,
    // where that byte is the in-bounds NUL.
    pair().assert_same(n + 1, Some(src), "boundary one past buffer length");

    // The two capacity-rounding edges for every small size.
    for size in 1..=64i32 {
        pair().assert_same(size, Some(src.repeat(8).as_slice()), "boundary small sizes");
    }
}

#[test]
fn boundary_null_pointer_takes_precedence_over_every_other_condition() {
    // The NULL check is first in the C, so it must win even for sizes that
    // would otherwise fail the allocation (row 2) or wrap (row 5).
    for size in [0x2000_0000i32, i32::MIN, i32::MAX, -1, 0] {
        assert_eq!(
            pair().c.call(size, None),
            Outcome::Null,
            "C: NULL check must precede size handling, size={size}"
        );
        assert_eq!(
            pair().rs.call(size, None),
            Outcome::Null,
            "Rust: NULL check must precede size handling, size={size}"
        );
    }
}

#[test]
fn boundary_int_extremes_exhaustive_neighbourhoods() {
    let src = b"xyz\0";
    // Only the INT_MIN side is observable. On the INT_MAX side `size*4` wraps
    // to a small positive `cap` (INT_MAX -> cap == 3, INT_MAX-1 -> cap == 2),
    // so `calloc` succeeds and the C loop writes ~2.8 GiB past a 3-byte
    // buffer: ERRORS.md row 7, undefined behaviour with no ground truth to
    // compare against. Those inputs are deliberately not called.
    for delta in 0..16i32 {
        let size = i32::MIN + delta;
        pair().assert_same(size, Some(src), "boundary INT_MIN neighbourhood");
    }
    // The largest positive sizes that ARE observable: the top of the
    // calloc-failure band, where cap stays negative.
    for size in [0x3FFF_FFFCi32, 0x3FFF_FFFB, 0x3FFF_FFFA] {
        pair().assert_same_nullness(size, Some(src), "boundary top of failure band");
    }
}
