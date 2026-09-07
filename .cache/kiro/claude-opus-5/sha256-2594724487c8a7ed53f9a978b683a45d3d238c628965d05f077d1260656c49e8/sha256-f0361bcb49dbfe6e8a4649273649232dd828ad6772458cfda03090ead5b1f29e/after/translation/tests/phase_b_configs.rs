//! Phase B — valid-path differential tests, gated on `CONFIGS.md`.
//!
//! Every one of the 128 `CONFIGS.md` rows is the cross product
//! `i (2) x layer (4) x k (16)`. For each row the bits the C never uses
//! (`h[0]`, `h[1]` bit0, `h[2]`'s low nibble) are randomized with a fixed seed,
//! so a row only passes if it holds across all the values that must not matter.
//!
//! Both implementations are reached exclusively through their `.so` exports.

mod common;

use common::{Pair, Rng};

/// Randomized iterations per `CONFIGS.md` row.
const ITERS_PER_ROW: usize = 64;

/// Build a header for row `(i, layer, k)`, randomizing every bit the C ignores.
fn header_for(rng: &mut Rng, i: u8, layer: u8, k: u8) -> [u8; 3] {
    // h[1]: bit3 = i, bits2..1 = layer, bit0 free, bits7..4 free.
    let free_hi = rng.next_u8() & 0xF0;
    let free_lo_bit = rng.next_u8() & 0x01;
    let h1 = free_hi | (i << 3) | (layer << 1) | free_lo_bit;
    // h[2]: high nibble = k, low nibble free.
    let h2 = (k << 4) | (rng.next_u8() & 0x0F);
    [rng.next_u8(), h1, h2]
}

/// Rows 1..128 of `CONFIGS.md`, randomized.
#[test]
fn phase_b_all_config_rows() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let mut row = 0usize;
    let mut checked = 0usize;

    for i in 0u8..2 {
        for layer in 0u8..4 {
            for k in 0u8..16 {
                row += 1;
                let ctx = format!("CONFIGS.md row {row}: i={i}, layer={layer:#04b}, k={k}");
                for _ in 0..ITERS_PER_ROW {
                    let h = header_for(&mut rng, i, layer, k);
                    // Re-assert the row's invariants actually hold for this input.
                    assert_eq!(u8::from(h[1] & 0x8 != 0), i, "row setup broken");
                    assert_eq!((h[1] >> 1) & 3, layer, "row setup broken");
                    assert_eq!(h[2] >> 4, k, "row setup broken");
                    pair.assert_same(&h, &ctx);
                    checked += 1;
                }
            }
        }
    }

    assert_eq!(row, 128, "expected exactly 128 CONFIGS.md rows");
    assert_eq!(checked, 128 * ITERS_PER_ROW);
}

/// Superset of every row: all 65 536 `(h[1], h[2])` pairs. The input space that
/// can affect the result is finite and small, so Phase B does not have to
/// sample — it proves equivalence exhaustively.
#[test]
fn phase_b_exhaustive_h1_h2() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xA5A5_5A5A_DEAD_BEEF);
    for h1 in 0u16..256 {
        for h2 in 0u16..256 {
            let h = [rng.next_u8(), h1 as u8, h2 as u8];
            pair.assert_same(&h, "exhaustive h1 x h2");
        }
    }
}

/// `h[0]` is never read by the C. The result must be invariant under all 256
/// values of it, for a spread of `(h[1], h[2])` including the edge rows.
#[test]
fn phase_b_h0_is_ignored() {
    let pair = Pair::load();
    for &(h1, h2) in &[
        (0x00u8, 0x00u8), // reserved layer, k=0  -> OOB<
        (0x00, 0xF0),     // reserved layer, k=15 -> offset 0
        (0x08, 0x00),     // i=1, reserved layer  -> alias row
        (0x02, 0x90),     // i=0, layer 0b01
        (0x0E, 0xF0),     // i=1, layer 0b11, k=15 -> OOB>
        (0xFF, 0xFF),     // all bits set
    ] {
        let base = pair.assert_same(&[0x00, h1, h2], "h0 baseline");
        for h0 in 0u16..256 {
            let got = pair.assert_same(&[h0 as u8, h1, h2], "h0 sweep");
            assert_eq!(
                got, base,
                "h[0]={h0:#04x} changed the result for h1={h1:#04x} h2={h2:#04x}: {base} -> {got}"
            );
        }
    }
}

/// The low bit of `h[1]` and the low nibble of `h[2]` are shifted/masked out by
/// the C and must not affect the result either.
#[test]
fn phase_b_unused_bitfields_are_ignored() {
    let pair = Pair::load();
    for i in 0u8..2 {
        for layer in 0u8..4 {
            for k in 0u8..16 {
                let canonical = [0u8, (i << 3) | (layer << 1), k << 4];
                let base = pair.assert_same(&canonical, "unused-bits baseline");
                for lowbit in 0u8..2 {
                    for lownib in 0u8..16 {
                        let h = [0u8, (i << 3) | (layer << 1) | lowbit, (k << 4) | lownib];
                        let got = pair.assert_same(&h, "unused-bits sweep");
                        assert_eq!(
                            got, base,
                            "unused bits changed result: i={i} layer={layer} k={k} \
                             lowbit={lowbit} lownib={lownib}"
                        );
                    }
                }
            }
        }
    }
}

/// The upper nibble of `h[1]` (bits 7..4) is likewise untouched by the C.
#[test]
fn phase_b_h1_high_nibble_is_ignored() {
    let pair = Pair::load();
    for i in 0u8..2 {
        for layer in 0u8..4 {
            for k in [0u8, 1, 7, 14, 15] {
                let base = pair.assert_same(&[0, (i << 3) | (layer << 1), k << 4], "hi-nib base");
                for hi in 0u8..16 {
                    let h = [0u8, (hi << 4) | (i << 3) | (layer << 1), k << 4];
                    let got = pair.assert_same(&h, "hi-nib sweep");
                    assert_eq!(got, base, "h[1] high nibble {hi:#x} changed result");
                }
            }
        }
    }
}
