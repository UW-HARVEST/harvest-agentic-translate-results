//! Differential tests: every call goes through a dynamically loaded `.so`
//! export, for both the C reference and the Rust translation.

mod common;

use common::{load_pair, CbRgb255};

fn rgb(r: u8, g: u8, b: u8) -> CbRgb255 {
    CbRgb255 { r, g, b }
}

fn assert_same(c: &common::Impl, rust: &common::Impl, input: CbRgb255) {
    let expected = c.tritanopia(input);
    let actual = rust.tritanopia(input);
    assert_eq!(
        actual, expected,
        "mismatch for input {input:?}: C gave {expected:?}, Rust gave {actual:?}"
    );
}

/// Both libraries load and expose the `tritanopia` symbol.
#[test]
fn libraries_load_and_export_tritanopia() {
    let (c, rust) = load_pair();
    assert_eq!(c.label, "c");
    assert_eq!(rust.label, "rust");
    assert_same(&c, &rust, rgb(0, 0, 0));
}

/// Corner cases of the input cube: the extremes exercise the `> 0.04045`
/// gamma-removal branch boundary and, via the tritanopia matrix, drive
/// channels outside `[0, 1]` so that the out-of-range `unsigned char` cast in
/// `cbDenorm` is hit.
#[test]
fn cube_corners_and_channel_extremes() {
    let (c, rust) = load_pair();
    let extremes = [0u8, 1, 2, 9, 10, 11, 12, 127, 128, 254, 255];
    for &r in &extremes {
        for &g in &extremes {
            for &b in &extremes {
                assert_same(&c, &rust, rgb(r, g, b));
            }
        }
    }
}

/// Grey ramp: R == G == B for every level. The matrix rows sum to ~1 here, so
/// these stay in range and validate the ordinary path end to end.
#[test]
fn grey_ramp() {
    let (c, rust) = load_pair();
    for v in 0..=255u8 {
        assert_same(&c, &rust, rgb(v, v, v));
    }
}

/// Pure and saturated channels across the full 0..=255 range. Maximises the
/// blue/green imbalance that pushes the red channel negative or above one.
#[test]
fn single_channel_sweeps() {
    let (c, rust) = load_pair();
    for v in 0..=255u8 {
        for input in [
            rgb(v, 0, 0),
            rgb(0, v, 0),
            rgb(0, 0, v),
            rgb(v, 255, 0),
            rgb(v, 0, 255),
            rgb(255, v, 0),
            rgb(0, v, 255),
            rgb(255, 0, v),
            rgb(0, 255, v),
            rgb(255, 255, v),
            rgb(255, v, 255),
            rgb(v, 255, 255),
        ] {
            assert_same(&c, &rust, input);
        }
    }
}

/// The `cbNorm` output values straddling the `0.04045` threshold: 10/255 is
/// below it, 11/255 is above, so these inputs cover both gamma branches on
/// each channel independently.
#[test]
fn gamma_threshold_neighbourhood() {
    let (c, rust) = load_pair();
    for r in 8..=14u8 {
        for g in 8..=14u8 {
            for b in 8..=14u8 {
                assert_same(&c, &rust, rgb(r, g, b));
            }
        }
    }
}

/// Deterministic pseudo-random sweep over the whole cube.
#[test]
fn pseudo_random_sample() {
    let (c, rust) = load_pair();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..200_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let r = (state & 0xFF) as u8;
        let g = ((state >> 8) & 0xFF) as u8;
        let b = ((state >> 16) & 0xFF) as u8;
        assert_same(&c, &rust, rgb(r, g, b));
    }
}
