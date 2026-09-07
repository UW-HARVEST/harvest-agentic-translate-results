//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library's only rejection sentinel is the return value `0`; success is
//! `1`. Each test asserts the C and Rust `.so`s return the SAME int, and
//! additionally pins the expected sentinel so "both wrong in the same way" is
//! also caught.

mod common;

use common::{make_byte1, make_byte2, valid_pair, Libs, Rng, SEED};

const ITERS: usize = 20_000;

/// Row 1 — `h2[0] != 0xff` (wrong sync byte). All 255 wrong values.
#[test]
fn err01_h2_byte0_not_ff() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..256 {
        let (h1, mut h2) = valid_pair(&mut rng);
        for v in 0u16..256 {
            if v == 0xff {
                continue;
            }
            h2[0] = v as u8;
            assert_eq!(libs.diff(&h1, &h2), 0, "h2[0]={v:#04x} must be rejected");
        }
    }
}

/// Row 2 — `h2[1]` matches neither `(x & 0xF0) == 0xf0` nor `(x & 0xFE) == 0xe2`.
#[test]
fn err02_h2_byte1_bad_sync_form() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE2);

    let bad: Vec<u8> = (0u16..256)
        .map(|v| v as u8)
        .filter(|&v| (v & 0xF0) != 0xf0 && (v & 0xFE) != 0xe2)
        .collect();
    assert_eq!(bad.len(), 238, "sanity: 256 - 16 (0xFx) - 2 (0xE2/0xE3)");

    for &b1 in &bad {
        for _ in 0..64 {
            let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
            let h2 = [0xff, b1, b2, rng.u8()];
            let h1 = [rng.u8(), b1, b2, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 0, "h2[1]={b1:#04x} must be rejected");
        }
    }
}

/// Row 3 — `((h2[1] >> 1) & 3) == 0` (reserved layer field 0).
///
/// Note this is only reachable for form-F bytes (`0xF0, 0xF1, 0xF8, 0xF9`),
/// because form-E forces the layer field to 1.
#[test]
fn err03_h2_layer_field_zero() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE3);

    let layer0: Vec<u8> = (0u16..256)
        .map(|v| v as u8)
        .filter(|&v| ((v & 0xF0) == 0xf0 || (v & 0xFE) == 0xe2) && ((v >> 1) & 3) == 0)
        .collect();
    assert_eq!(layer0, vec![0xf0, 0xf1, 0xf8, 0xf9]);

    for &b1 in &layer0 {
        for _ in 0..512 {
            let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
            let h2 = [0xff, b1, b2, rng.u8()];
            let h1 = [rng.u8(), b1, b2, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 0, "h2[1]={b1:#04x} layer 0 must be rejected");
        }
    }
}

/// Row 4 — `(h2[2] >> 4) == 15` (bitrate index 15).
#[test]
fn err04_h2_bitrate_index_15() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE4);
    for low in 0u16..16 {
        for _ in 0..1_000 {
            let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
            let b2 = 0xF0 | (low as u8);
            assert_eq!(b2 >> 4, 15);
            let h2 = [0xff, b1, b2, rng.u8()];
            let h1 = [rng.u8(), b1, b2, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 0, "h2[2]={b2:#04x} must be rejected");
        }
    }
}

/// Row 5 — `((h2[2] >> 2) & 3) == 3` (reserved sample-rate index 3).
#[test]
fn err05_h2_sample_rate_index_3() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE5);
    for bitrate in 0u8..=14 {
        for _ in 0..1_000 {
            let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
            let b2 = (bitrate << 4) | (3 << 2) | (rng.u8() & 3);
            assert_eq!((b2 >> 2) & 3, 3);
            let h2 = [0xff, b1, b2, rng.u8()];
            let h1 = [rng.u8(), b1, b2, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 0, "h2[2]={b2:#04x} must be rejected");
        }
    }
}

/// Row 6 — `((h1[1] ^ h2[1]) & 0xFE) != 0`: byte-1 mismatch above bit 0.
#[test]
fn err06_byte1_mismatch() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE6);
    let mut checked = 0usize;
    for _ in 0..ITERS {
        let (mut h1, h2) = valid_pair(&mut rng);
        // Flip at least one bit in 1..=7.
        let mask = loop {
            let m = rng.u8() & 0xFE;
            if m != 0 {
                break m;
            }
        };
        h1[1] = h2[1] ^ mask;
        assert_ne!((h1[1] ^ h2[1]) & 0xFE, 0);
        assert_eq!(libs.diff(&h1, &h2), 0, "h1={h1:02x?} h2={h2:02x?}");
        checked += 1;
    }
    assert_eq!(checked, ITERS);
}

/// Row 7 — `((h1[2] ^ h2[2]) & 0x0C) != 0`: sample-rate index mismatch.
#[test]
fn err07_sample_rate_mismatch() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE7);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let s_h2 = rng.range(0, 2) as u8;
        // Pick a different srate index for h1 (may be 3 — h1 is not validated).
        let mut s_h1 = rng.range(0, 3) as u8;
        if s_h1 == s_h2 {
            s_h1 = (s_h2 + 1) & 3;
        }
        // Keep free-ness identical so row 8's condition cannot be the cause.
        let br = rng.range(1, 14) as u8;
        let b2b = (br << 4) | (s_h2 << 2) | (rng.u8() & 3);
        let b2a = (br << 4) | (s_h1 << 2) | (rng.u8() & 3);
        assert_ne!((b2a ^ b2b) & 0x0C, 0);
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 0, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 8 — free-bitrate mismatch on the `& 0xF0` test.
#[test]
fn err08_free_bitrate_mismatch() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let srate = rng.range(0, 2) as u8; // identical -> row 7 cannot fire
        let (br_h1, br_h2) = if rng.below(2) == 0 {
            (0u8, rng.range(1, 14) as u8)
        } else {
            (rng.range(1, 15) as u8, 0u8)
        };
        let b2a = (br_h1 << 4) | (srate << 2) | (rng.u8() & 3);
        let b2b = (br_h2 << 4) | (srate << 2) | (rng.u8() & 3);
        assert_ne!((b2a & 0xF0) == 0, (b2b & 0xF0) == 0);
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 0, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 9 — exact-length (3-byte) allocations: no read past `h[2]` in either
/// implementation, and identical results.
#[test]
fn err09_exact_length_buffers_no_overread() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..50_000 {
        let mut a = [0u8; 3];
        let mut b = [0u8; 3];
        rng.fill(&mut a);
        rng.fill(&mut b);
        if rng.below(2) == 0 {
            b[0] = 0xff;
            b[1] |= 0xF0;
        }
        libs.diff_exact(&a, &b);
    }
    // Also the all-zero and all-0xff extremes.
    libs.diff_exact(&[0, 0, 0], &[0, 0, 0]);
    libs.diff_exact(&[0xff, 0xff, 0xff], &[0xff, 0xff, 0xff]);
    libs.diff_exact(&[0, 0, 0], &[0xff, 0xff, 0xff]);
    libs.diff_exact(&[0xff, 0xff, 0xff], &[0, 0, 0]);
}

/// Row 10 — `h1 == NULL` while `h2` is INVALID. The C `&&` short-circuits on
/// `hdr_valid(h2)`, so `h1` is never dereferenced; the Rust must do the same.
///
/// Every `h2` used here is invalid via at least one of the five `hdr_valid`
/// conjuncts, so this exercises the null short-circuit through each of them.
#[test]
fn err10_null_h1_with_invalid_h2() {
    let libs = Libs::load();

    // One invalid h2 per hdr_valid conjunct, plus a fully-zero header.
    let invalid_h2: [[u8; 4]; 6] = [
        [0x00, 0xFB, 0x90, 0x00], // conjunct 1: sync byte
        [0xFF, 0x00, 0x90, 0x00], // conjunct 2: sync form
        [0xFF, 0xF0, 0x90, 0x00], // conjunct 3: layer field 0
        [0xFF, 0xFB, 0xF0, 0x00], // conjunct 4: bitrate 15
        [0xFF, 0xFB, 0x9C, 0x00], // conjunct 5: srate 3
        [0x00, 0x00, 0x00, 0x00], // all zero
    ];

    for h2 in &invalid_h2 {
        let c = unsafe { libs.c_compare(std::ptr::null(), h2.as_ptr()) };
        let r = unsafe { libs.rs_compare(std::ptr::null(), h2.as_ptr()) };
        assert_eq!(c, 0, "C must reject invalid h2 without touching h1: {h2:02x?}");
        assert_eq!(r, c, "DIVERGENCE null-h1 h2={h2:02x?}: C={c} Rust={r}");
    }

    // Also with a 3-byte-exact allocation for h2.
    for h2 in &invalid_h2 {
        let b: Box<[u8]> = h2[..3].to_vec().into_boxed_slice();
        let c = unsafe { libs.c_compare(std::ptr::null(), b.as_ptr()) };
        let r = unsafe { libs.rs_compare(std::ptr::null(), b.as_ptr()) };
        assert_eq!(c, 0);
        assert_eq!(r, c);
    }
}

/// Row 11 — bytes the C never reads (`h1[0]`, `h1[3..]`, `h2[3..]`) must not
/// influence the result, for either implementation.
#[test]
fn err11_never_read_bytes_are_irrelevant() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xEB);
    for _ in 0..20_000 {
        let mut a = [0u8; 12];
        let mut b = [0u8; 12];
        rng.fill(&mut a);
        rng.fill(&mut b);
        if rng.below(2) == 0 {
            b[0] = 0xff;
            b[1] |= 0xF0;
        }
        let baseline = libs.diff(&a, &b);

        for _ in 0..4 {
            let mut a2 = a;
            let mut b2 = b;
            a2[0] = rng.u8(); // h1[0] is never read
            for i in 3..12 {
                a2[i] = rng.u8();
                b2[i] = rng.u8();
            }
            assert_eq!(
                libs.diff(&a2, &b2),
                baseline,
                "an unread byte changed the result: a={a2:02x?} b={b2:02x?}"
            );
        }
    }
}

/// Generic boundary: "out-of-range enum" analogue. This API takes no enums, so
/// the equivalent is every possible byte value in each inspected field,
/// including values with no meaningful interpretation. Exhaustive over
/// `h2[1] x h2[2]` with `h1` = `h2` (65 536 combinations).
#[test]
fn err_generic_all_byte_values_self_compare() {
    let libs = Libs::load();
    for b1 in 0u16..256 {
        for b2 in 0u16..256 {
            let h = [0xffu8, b1 as u8, b2 as u8, 0x00];
            libs.diff(&h, &h);
            // and with a non-0xff h1[0]
            let h1 = [0x00u8, b1 as u8, b2 as u8, 0xff];
            libs.diff(&h1, &h);
        }
    }
}

/// Generic boundary: both pointers null AND h2 invalid is impossible (h2 must
/// be read), but a null `h2` with a null `h1` is undefined in C — instead we
/// pin the closest defined boundary: `h1 == h2` (aliasing the same buffer).
#[test]
fn err_generic_aliased_pointers() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xEC);
    for _ in 0..50_000 {
        let mut h = [0u8; 4];
        rng.fill(&mut h);
        if rng.below(2) == 0 {
            h[0] = 0xff;
        }
        let boxed: Box<[u8]> = h[..3].to_vec().into_boxed_slice();
        let p = boxed.as_ptr();
        let c = unsafe { libs.c_compare(p, p) };
        let r = unsafe { libs.rs_compare(p, p) };
        assert_eq!(c, r, "DIVERGENCE aliased h={h:02x?}: C={c} Rust={r}");
    }
}

/// Generic boundary: the return value must be exactly 0 or 1 (a C logical
/// expression), never another truthy int.
#[test]
fn err_generic_return_value_is_zero_or_one() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xED);
    for _ in 0..200_000 {
        let mut a = [0u8; 4];
        let mut b = [0u8; 4];
        rng.fill(&mut a);
        rng.fill(&mut b);
        if rng.below(2) == 0 {
            b[0] = 0xff;
            b[1] |= 0xF0;
        }
        let v = libs.diff(&a, &b);
        assert!(v == 0 || v == 1, "return value out of {{0,1}}: {v}");
    }
}
