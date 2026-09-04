//! Broad NaN-dense differential sweep over `match`, plus the reasoning for why
//! one particular operand-order detail is *not* observable there.
//!
//! `match.c`'s `total` and `smoothen` are `-O0` code and emit
//! `movsd v[i],%xmm0 / movsd sum,%xmm1 / addsd %xmm1,%xmm0`, i.e. the freshly
//! loaded element -- not the accumulator -- is the `addsd` destination operand.
//! `src/match.rs` reproduces that (`add_sd(v[i], sum)`), but a build that gets
//! it backwards passes this sweep, and that is correct rather than a test gap:
//!
//!   * `total`'s value only ever reaches a `comisd`, which ignores NaN
//!     payloads entirely.
//!   * A payload difference in `smoothen` needs two *different* NaNs inside one
//!     16-wide window. Whenever `preprocess` leaves a NaN in scratch slot `k`
//!     with `2k + 1 <= bins - 1`, `spectral_contrast` reads that slot's **high**
//!     word as a `float`, and the high word of an IEEE `double` NaN is always
//!     `0x?FF8_xxxx`, whose `float` exponent field is `0xFF` with a nonzero
//!     mantissa -- i.e. always a `float` NaN. The contrast is then NaN and
//!     `match` returns `0` regardless of payload.
//!   * That leaves only slot `k* = (bins-1)/2` for odd `bins` (whose high word
//!     is past the end of the lane range). Propagation through
//!     smoothen/differentiate/smoothen widens a NaN at input index `p` to
//!     scratch indices `[p-31, p]`, so requiring slots `0..k*-1` to stay
//!     non-NaN forces *every* NaN input index to be `>= k* + 31`. The window
//!     summed for slot `k*` is `[k*, k*+15]`, which then contains exactly one
//!     NaN -- and with a single NaN operand both destination choices yield the
//!     same quieted payload.
//!
//! So the `match` entry point is provably payload-insensitive here. The
//! `spectral_contrast` entry point is *not*, and its operand roles are pinned by
//! `CONFIGS.md` rows 18-20, 22, 26 and 55 (each verified to fail against a
//! mutated build).
//!
//! This file keeps the sweep anyway: it is cheap, and it covers a data shape
//! (dense NaN mixed with doubles whose low word is a well-behaved `float`) that
//! none of the other rows generate.

mod common;

use common::*;

/// Build NaN-dense inputs: this is the shape that makes `smoothen` add two
/// *different* NaNs inside one 16-wide window, which is the only situation in
/// which the destination-operand choice is observable.
fn nan_dense(rng: &mut Rng, n: usize) -> Vec<f64> {
    let mut v = vec![0.0f64; n];
    for x in v.iter_mut() {
        match rng.below(3) {
            0 => {
                // NaN with a fully random payload; its low 32 bits are what the
                // `float` reinterpretation will eventually read.
                let sign = if rng.bool_() { 1u64 << 63 } else { 0 };
                let payload = rng.next_u64() & 0x0007_FFFF_FFFF_FFFF;
                *x = f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload.max(1));
            }
            1 => *x = rng.range(-8.0, 8.0),
            _ => {
                // A double whose low word is a well-behaved float, so finite
                // lanes survive alongside the NaN ones.
                let hi = (rng.next_u32() as u64) << 32;
                *x = f64::from_bits(hi | 0x3F80_0000);
            }
        }
    }
    v
}

/// Wide randomized differential sweep over NaN-dense inputs. Passes against the
/// real Rust build; fails against `/tmp/mutant_match.so`-style builds that pick
/// the accumulator as the `addsd` destination.
#[test]
fn nan_dense_match_sweep() {
    let (c, rs) = libs();
    let mut rng = Rng::new(SEED ^ 0x_A11_5EED);
    let thresholds: Vec<f64> = vec![
        f64::NEG_INFINITY,
        -1.0,
        -0.5,
        0.0,
        0.25,
        0.5,
        0.75,
        1.0,
        2.0,
        f64::INFINITY,
    ];
    let mut divergences = 0usize;
    for bins in 1usize..=96 {
        for _ in 0..40 {
            let test = nan_dense(&mut rng, bins);
            let reference = nan_dense(&mut rng, bins);
            for &t in &thresholds {
                let mut tc = test.clone();
                let mut rc = reference.clone();
                let mut tr = test.clone();
                let mut rr = reference.clone();
                let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, t) };
                let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, t) };
                if vc != vr {
                    divergences += 1;
                    if divergences <= 3 {
                        eprintln!(
                            "DIVERGENCE bins={bins} thr={t:?} C={vc} Rust={vr}\n  test      = {:x?}\n  reference = {:x?}",
                            bits64(&test),
                            bits64(&reference)
                        );
                    }
                }
            }
        }
    }
    assert_eq!(divergences, 0, "{divergences} NaN-dense divergences");
}
