//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no rejection paths at all
//! (see the grep evidence in `ERRORS.md`), so each row asserts the *same*
//! non-rejection: identical stdout bytes, full 33-byte record, no truncation,
//! no sentinel special-casing, and no trap/abort in either implementation.

mod common;

use common::*;
use std::ffi::c_int;

/// A "rejection" for this API would show up as a short/absent record, so assert
/// the exact shape both implementations must produce.
#[track_caller]
fn assert_accepted_identically(x: c_int) -> Vec<u8> {
    let out = assert_same(x);
    assert_eq!(
        out.len(),
        33,
        "driver({x}) produced {} bytes, expected 33 — looks like a rejection",
        out.len()
    );
    assert_eq!(*out.last().unwrap(), b'\n');
    assert_eq!(out, expected_bytes(x), "byte model mismatch for {x}");
    out
}

// --------------------------------------------------------------------------
// E1 — no input is rejected
// --------------------------------------------------------------------------
#[test]
fn err_e1_no_input_is_rejected() {
    let mut rng = Rng::new(0xE1E1_E1E1);
    // Structural extremes plus randomized values from the whole domain.
    let mut cases: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..SAMPLES {
        cases.push(rng.next_i32());
    }
    for x in cases {
        assert_accepted_identically(x);
    }
}

// --------------------------------------------------------------------------
// E2 — INT_MIN
// --------------------------------------------------------------------------
#[test]
fn err_e2_int_min() {
    let out = assert_accepted_identically(i32::MIN);
    assert_eq!(&out[..8], b"00000080");
    // One step "past" the range in the only direction possible: wrap to INT_MAX.
    let out = assert_accepted_identically(i32::MIN.wrapping_sub(1));
    assert_eq!(&out[..8], b"ffffff7f");
}

// --------------------------------------------------------------------------
// E3 — INT_MAX
// --------------------------------------------------------------------------
#[test]
fn err_e3_int_max() {
    let out = assert_accepted_identically(i32::MAX);
    assert_eq!(&out[..8], b"ffffff7f");
    let out = assert_accepted_identically(i32::MAX.wrapping_add(1));
    assert_eq!(&out[..8], b"00000080");
}

// --------------------------------------------------------------------------
// E4 — -1 is data, not a sentinel
// --------------------------------------------------------------------------
#[test]
fn err_e4_minus_one_is_not_a_sentinel() {
    let out = assert_accepted_identically(-1);
    assert_eq!(&out[..8], b"ffffffff");
    // Neighbours behave the same way; -1 gets no special treatment.
    for x in [-3i32, -2, -1, 0, 1] {
        assert_accepted_identically(x);
    }
}

// --------------------------------------------------------------------------
// E5 — zero
// --------------------------------------------------------------------------
#[test]
fn err_e5_zero() {
    let out = assert_accepted_identically(0);
    assert_eq!(&out[..8], b"00000000");
    assert_eq!(out.len(), 33, "zero must not shorten the record");
}

// --------------------------------------------------------------------------
// E6 — out-of-range enum-style values across the FFI boundary
// --------------------------------------------------------------------------
#[test]
fn err_e6_out_of_range_enum_values() {
    // Values that would have no valid variant in any C enum, plus every value
    // just past a plausible small-enum range. C enums accept any int, so all of
    // these are real inputs the C accepts verbatim.
    let mut cases: Vec<c_int> = vec![
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        255,
        256,
        257,
        1000,
        0x7fff,
        0x8000,
        0xffff,
        0x10000,
        i32::MAX,
        i32::MIN,
        0xdeadbeefu32 as i32,
        0xcafebabeu32 as i32,
        0xffffff00u32 as i32,
        0x80000001u32 as i32,
    ];
    let mut rng = Rng::new(0xE6E6_E6E6);
    for _ in 0..SAMPLES {
        cases.push(rng.next_i32());
    }
    for x in cases {
        assert_accepted_identically(x);
    }
}

// --------------------------------------------------------------------------
// E7 — embedded NUL bytes must not truncate
// --------------------------------------------------------------------------
#[test]
fn err_e7_embedded_nul_no_truncation() {
    let fixed: [i32; 6] = [
        0x00ff00ffu32 as i32,
        0xff00ff00u32 as i32,
        0x00000001,
        0x01000000,
        0x00010000,
        0x00000000,
    ];
    for &x in &fixed {
        let out = assert_accepted_identically(x);
        assert_eq!(out.len(), 33, "truncated at NUL for 0x{:08x}", x as u32);
    }

    let mut rng = Rng::new(0xE7E7_E7E7);
    for _ in 0..SAMPLES {
        let mask = (rng.next_u32() & 0x0f) as u8;
        let mut bytes = [0u8; 4];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = if mask & (1 << i) != 0 { 0 } else { rng.byte(true) };
        }
        assert_accepted_identically(i32::from_le_bytes(bytes));
    }

    // Note: bytes 8..16 of the struct (bathrooms == 2.0) are *always* seven NUL
    // bytes followed by 0x40, so every single call already proves no truncation.
    let out = assert_accepted_identically(0x11223344);
    assert_eq!(&out[16..32], b"0000000000000040");
}

// --------------------------------------------------------------------------
// E8 — high-bit bytes are unsigned, never sign-extended
// --------------------------------------------------------------------------
#[test]
fn err_e8_high_bit_bytes_unsigned() {
    // Every possible high-bit byte in every lane.
    for b in 0x80u8..=0xff {
        for lane in 0..4usize {
            let mut bytes = [0u8; 4];
            bytes[lane] = b;
            let x = i32::from_le_bytes(bytes);
            let out = assert_accepted_identically(x);
            let pair = std::str::from_utf8(&out[lane * 2..lane * 2 + 2]).unwrap();
            assert_eq!(
                pair,
                format!("{b:02x}"),
                "byte {b:#04x} in lane {lane} was not printed as unsigned"
            );
            // A sign-extension bug would produce more than 33 bytes.
            assert_eq!(out.len(), 33);
        }
    }
    // All four bytes high-bit at once.
    let x = 0xffffffffu32 as i32;
    let out = assert_accepted_identically(x);
    assert_eq!(&out[..8], b"ffffffff");
}

// --------------------------------------------------------------------------
// E9 — newline / percent payload bytes must not corrupt framing
// --------------------------------------------------------------------------
#[test]
fn err_e9_newline_and_percent_payload_bytes() {
    for special in [0x0au8, 0x25u8, 0x73u8 /* 's' */, 0x6eu8 /* 'n' */] {
        for lane in 0..4usize {
            let mut bytes = [0xa5u8; 4];
            bytes[lane] = special;
            let x = i32::from_le_bytes(bytes);
            let out = assert_accepted_identically(x);
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                1,
                "framing broken by payload byte {special:#04x} in lane {lane}"
            );
            assert_eq!(*out.last().unwrap(), b'\n');
        }
    }
    // All four bytes = 0x0a / 0x25.
    for special in [0x0au8, 0x25u8] {
        let x = i32::from_le_bytes([special; 4]);
        let out = assert_accepted_identically(x);
        assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 1);
    }
}

// --------------------------------------------------------------------------
// E10 — the reachable loop bound is exact (no over-read, no early stop)
// --------------------------------------------------------------------------
#[test]
fn err_e10_loop_bound_exact() {
    const SIZEOF_HOUSE: usize = 16;
    let mut rng = Rng::new(0xE10_E10);
    for _ in 0..SAMPLES {
        let x = rng.next_i32();
        let c = run_c(x);
        let r = run_rust(x);
        assert_eq!(c, r);
        // Exactly 16 byte-pairs, i.e. no read past `raw` and no early stop.
        assert_eq!(c.len(), 2 * SIZEOF_HOUSE + 1);
        assert_eq!(r.len(), 2 * SIZEOF_HOUSE + 1);
        assert_eq!(c[..32].iter().filter(|b| b.is_ascii()).count(), 32);
    }
}

// --------------------------------------------------------------------------
// E11 — no residual state under repeated / interleaved abuse
// --------------------------------------------------------------------------
#[test]
fn err_e11_no_residual_state() {
    let cf = c_driver();
    let rf = rust_driver();

    // 500 calls per library back to back, alternating extreme values.
    let extremes: [c_int; 6] = [i32::MIN, i32::MAX, -1, 0, 1, 0xdeadbeefu32 as i32];
    let xs: Vec<c_int> = (0..500).map(|i| extremes[i % extremes.len()]).collect();

    let c = run_c_batch(&xs);
    let r = run_rust_batch(&xs);
    assert_eq!(c, r, "residual state diverged across 500 calls");
    let model: Vec<u8> = xs.iter().flat_map(|&x| expected_bytes(x)).collect();
    assert_eq!(c, model);

    // Interleaved, and then a fresh single call must still be pristine.
    let _ = capture("e11-interleave", || {
        for &x in &xs {
            unsafe {
                cf(x);
                rf(x);
            }
        }
    });
    assert_accepted_identically(42);
    assert_accepted_identically(i32::MIN);
}
