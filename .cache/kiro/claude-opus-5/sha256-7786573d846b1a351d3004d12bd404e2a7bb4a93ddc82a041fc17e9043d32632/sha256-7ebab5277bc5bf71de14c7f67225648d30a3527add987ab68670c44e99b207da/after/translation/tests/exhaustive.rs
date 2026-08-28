//! Exhaustive verification.
//!
//! `tritanopia` is a pure function of a 24-bit input, so the entire domain can
//! be enumerated. Passing this test means the Rust `.so` and the C `.so` agree
//! byte-for-byte on *every* possible input -- there is no untested case left.

mod common;

use common::{load_pair, CbRgb255};

#[test]
fn exhaustive_all_16_777_216_inputs() {
    let (c, rust) = load_pair();

    let mut checked: u64 = 0;
    let mut mismatches: Vec<(CbRgb255, CbRgb255, CbRgb255)> = Vec::new();

    for r in 0..=255u8 {
        for g in 0..=255u8 {
            for b in 0..=255u8 {
                let input = CbRgb255 { r, g, b };
                let expected = c.tritanopia(input);
                let actual = rust.tritanopia(input);
                if actual != expected {
                    if mismatches.len() < 20 {
                        mismatches.push((input, expected, actual));
                    }
                }
                checked += 1;
            }
        }
    }

    assert_eq!(checked, 256 * 256 * 256);
    assert!(
        mismatches.is_empty(),
        "{} of {checked} inputs mismatched. First examples (input, C, Rust): {:#?}",
        mismatches.len(),
        mismatches
    );
}
