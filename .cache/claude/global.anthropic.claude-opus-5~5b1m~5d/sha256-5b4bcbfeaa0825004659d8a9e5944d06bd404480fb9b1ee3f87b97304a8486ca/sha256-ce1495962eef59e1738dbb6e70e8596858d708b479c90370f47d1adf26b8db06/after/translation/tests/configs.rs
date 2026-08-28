//! Phase B — valid-path differential tests, one `#[test]` per CONFIGS.md row.
//!
//! Both libraries are dlopen'ed and driven through their exported symbols only.
//! Every comparison is on raw IEEE-754 bits, of the return value *and* of the
//! whole scratch buffer (both entry points mutate memory in place).

mod common;

use std::ffi::c_int;

use common::*;

// ===========================================================================
// helpers
// ===========================================================================

fn chk_sc(p: &Pair, row: &str, ctx: &str, buf: &[u32], a: usize, b: usize, len: c_int) {
    let c = sc_call(&p.c, buf, a, b, len);
    let r = sc_call(&p.rust, buf, a, b, len);
    assert_sc_eq(row, ctx, buf, &c, &r);
}

fn chk_sc_f64(p: &Pair, row: &str, ctx: &str, buf: &[u64], a: usize, b: usize, len: c_int) {
    let c = sc_call_via_f64(&p.c, buf, a, b, len);
    let r = sc_call_via_f64(&p.rust, buf, a, b, len);
    if c != r {
        let mut msg = format!("[{row}] spectral_contrast(double*) divergence: {ctx}\n");
        msg += &format!("  ret: C={:#018x} Rust={:#018x}\n", c.0, r.0);
        for i in 0..c.1.len() {
            if c.1[i] != r.1[i] {
                msg += &format!(
                    "  buf[{i}]: in={:#018x} C={:#018x} Rust={:#018x}\n",
                    buf[i], c.1[i], r.1[i]
                );
            }
        }
        msg += &format!("  input (f64 bits) = {:#018x?}\n", buf);
        panic!("{msg}");
    }
}

fn chk_match(p: &Pair, row: &str, ctx: &str, buf: &[u64], t: usize, r_off: usize, bins: c_int, thr: f64) {
    let c = match_call(&p.c, buf, t, r_off, bins, thr);
    let r = match_call(&p.rust, buf, t, r_off, bins, thr);
    assert_match_eq(row, ctx, buf, &c, &r);
    // C41: `match` only reads its inputs.
    assert_eq!(&c.buf[..], buf, "[{row}] C mutated its input buffer: {ctx}");
    assert_eq!(&r.buf[..], buf, "[{row}] Rust mutated its input buffer: {ctx}");
}

/// Build a 2n-element f32 buffer: `a` at 0, `b` at n.
fn sc_buf(rng: &mut Rng, n: usize, mut genf: impl FnMut(&mut Rng) -> u32) -> Vec<u32> {
    (0..2 * n).map(|_| genf(rng)).collect()
}

fn n_of(len: c_int) -> usize {
    if len > 0 { len as usize } else { 1 }
}

// ===========================================================================
// C01..C04 — spectral_contrast over the value-class axis, true f32 ABI
// ===========================================================================

#[test]
fn c01_sc_positive_normals() {
    let p = pair();
    let mut rng = Rng::new(1);
    for &len in LENGTHS {
        let draws = if len > 100 { 40 } else { 500 };
        for d in 0..draws {
            let n = n_of(len);
            let buf = sc_buf(&mut rng, n, |r| r.pos_normal_f32().to_bits());
            chk_sc(&p, "C01", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c02_sc_signed_normals() {
    let p = pair();
    let mut rng = Rng::new(2);
    for &len in LENGTHS {
        let draws = if len > 100 { 40 } else { 500 };
        for d in 0..draws {
            let n = n_of(len);
            let buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            chk_sc(&p, "C02", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c03_sc_arbitrary_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(3);
    for &len in LENGTHS {
        let draws = if len > 100 { 60 } else { 2000 };
        for d in 0..draws {
            let n = n_of(len);
            let buf = sc_buf(&mut rng, n, |r| r.any_f32_bits());
            chk_sc(&p, "C03", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c04_sc_exponent_biased() {
    let p = pair();
    let mut rng = Rng::new(4);
    for &len in LENGTHS {
        let draws = if len > 100 { 60 } else { 1500 };
        for d in 0..draws {
            let n = n_of(len);
            let buf = sc_buf(&mut rng, n, |r| r.exp_biased_f32_bits());
            chk_sc(&p, "C04", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

// ===========================================================================
// C05..C08 — structured relationships between a and b
// ===========================================================================

#[test]
fn c05_sc_constant_a_random_b() {
    let p = pair();
    let mut rng = Rng::new(5);
    for &len in LENGTHS {
        for d in 0..200 {
            let n = n_of(len);
            let k = if d % 4 == 0 {
                rng.any_f32_bits()
            } else {
                rng.signed_normal_f32().to_bits()
            };
            let mut buf = vec![k; n];
            buf.extend((0..n).map(|_| rng.signed_normal_f32().to_bits()));
            chk_sc(&p, "C05", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c06_sc_equal_contents() {
    let p = pair();
    let mut rng = Rng::new(6);
    for &len in LENGTHS {
        for d in 0..200 {
            let n = n_of(len);
            let half: Vec<u32> = (0..n)
                .map(|_| {
                    if d % 5 == 0 {
                        rng.any_f32_bits()
                    } else {
                        rng.signed_normal_f32().to_bits()
                    }
                })
                .collect();
            let mut buf = half.clone();
            buf.extend_from_slice(&half);
            chk_sc(&p, "C06", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c07_sc_negated() {
    let p = pair();
    let mut rng = Rng::new(7);
    for &len in LENGTHS {
        for d in 0..200 {
            let n = n_of(len);
            let half: Vec<f32> = (0..n).map(|_| rng.signed_normal_f32()).collect();
            let mut buf: Vec<u32> = half.iter().map(|v| v.to_bits()).collect();
            buf.extend(half.iter().map(|v| (-*v).to_bits()));
            chk_sc(&p, "C07", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c08_sc_ramps() {
    let p = pair();
    let mut rng = Rng::new(8);
    for &len in LENGTHS {
        for d in 0..60 {
            let n = n_of(len);
            let sa = rng.signed_normal_f32();
            let sb = rng.signed_normal_f32();
            let mut buf: Vec<u32> = (0..n).map(|i| ((i as f32) * sa).to_bits()).collect();
            buf.extend((0..n).map(|i| (((n - i) as f32) * sb).to_bits()));
            chk_sc(&p, "C08", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

// ===========================================================================
// C09..C16 — degenerate magnitudes and special values
// ===========================================================================

#[test]
fn c09_sc_all_subnormal() {
    let p = pair();
    let mut rng = Rng::new(9);
    for &len in LENGTHS {
        for d in 0..200 {
            let n = n_of(len);
            let g = |r: &mut Rng| {
                let sign = (r.next_u32() & 1) << 31;
                let mut m = r.next_u32() & 0x007F_FFFF;
                if m == 0 {
                    m = 1;
                }
                sign | m // exponent 0 => subnormal
            };
            let buf: Vec<u32> = (0..2 * n).map(|_| g(&mut rng)).collect();
            chk_sc(&p, "C09", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c10_sc_overflowing_magnitude() {
    let p = pair();
    let mut rng = Rng::new(10);
    for &len in LENGTHS {
        for d in 0..200 {
            let n = n_of(len);
            // exponent 100..254 => x*x overflows f32 to +inf
            let g = |r: &mut Rng| {
                let sign = (r.next_u32() & 1) << 31;
                let exp = (200 + (r.next_u32() % 55)) << 23;
                let mant = r.next_u32() & 0x007F_FFFF;
                sign | exp | mant
            };
            let buf: Vec<u32> = (0..2 * n).map(|_| g(&mut rng)).collect();
            chk_sc(&p, "C10", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c11_sc_single_infinity() {
    let p = pair();
    let mut rng = Rng::new(11);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..200 {
            let mut buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            let i = rng.below(n);
            let j = rng.below(n);
            buf[i] = if rng.next_u64() & 1 == 0 { 0x7F80_0000 } else { 0xFF80_0000 };
            if d % 3 == 0 {
                buf[n + j] = if rng.next_u64() & 1 == 0 { 0x7F80_0000 } else { 0xFF80_0000 };
            }
            chk_sc(&p, "C11", &format!("len={len} draw={d} i={i}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c12_sc_single_qnan() {
    let p = pair();
    let mut rng = Rng::new(12);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..300 {
            let mut buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            let i = rng.below(n);
            buf[i] = rng.qnan_f32_bits();
            chk_sc(&p, "C12", &format!("len={len} draw={d} i={i}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c13_sc_single_snan() {
    let p = pair();
    let mut rng = Rng::new(13);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..300 {
            let mut buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            let i = rng.below(n);
            buf[i] = rng.snan_f32_bits();
            chk_sc(&p, "C13", &format!("len={len} draw={d} i={i}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c14_sc_multiple_nan_payloads_in_a() {
    let p = pair();
    let mut rng = Rng::new(14);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..400 {
            let mut buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            let k = 2 + rng.below(4.min(n).max(1));
            for _ in 0..k {
                let i = rng.below(n);
                buf[i] = if rng.next_u64() & 1 == 0 {
                    rng.qnan_f32_bits()
                } else {
                    rng.snan_f32_bits()
                };
            }
            chk_sc(&p, "C14", &format!("len={len} draw={d} k={k}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c15_sc_nan_in_both_same_index() {
    let p = pair();
    let mut rng = Rng::new(15);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..400 {
            let mut buf = sc_buf(&mut rng, n, |r| r.signed_normal_f32().to_bits());
            let i = rng.below(n);
            buf[i] = rng.qnan_f32_bits();
            buf[n + i] = rng.qnan_f32_bits();
            if d % 4 == 0 {
                buf[i] = rng.snan_f32_bits();
                buf[n + i] = rng.snan_f32_bits();
            }
            chk_sc(&p, "C15", &format!("len={len} draw={d} i={i}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c16_sc_signed_zero_mixture() {
    let p = pair();
    let mut rng = Rng::new(16);
    for &len in LENGTHS {
        let n = n_of(len);
        for d in 0..200 {
            let buf: Vec<u32> = (0..2 * n)
                .map(|_| if rng.next_u64() & 1 == 0 { 0x0000_0000 } else { 0x8000_0000 })
                .collect();
            chk_sc(&p, "C16", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

// ===========================================================================
// C17..C18 — pointer relationships
// ===========================================================================

#[test]
fn c17_sc_fully_aliased() {
    let p = pair();
    let mut rng = Rng::new(17);
    for &len in &[1i32, 2, 3, 15, 16, 17, 64] {
        let n = n_of(len);
        for d in 0..300 {
            let buf: Vec<u32> = (0..2 * n)
                .map(|_| match d % 3 {
                    0 => rng.signed_normal_f32().to_bits(),
                    1 => rng.any_f32_bits(),
                    _ => rng.exp_biased_f32_bits(),
                })
                .collect();
            chk_sc(&p, "C17", &format!("aliased len={len} draw={d}"), &buf, 0, 0, len);
        }
    }
}

#[test]
fn c18_sc_partial_overlap() {
    let p = pair();
    let mut rng = Rng::new(18);
    for &len in &[2i32, 3, 15, 16, 17, 33, 64] {
        let n = n_of(len);
        for d in 0..300 {
            let buf: Vec<u32> = (0..2 * n)
                .map(|_| if d % 2 == 0 { rng.signed_normal_f32().to_bits() } else { rng.any_f32_bits() })
                .collect();
            chk_sc(&p, "C18", &format!("b=a+1 len={len} draw={d}"), &buf, 0, 1, len);
            let k = 1 + rng.below(n);
            chk_sc(&p, "C18", &format!("b=a+{k} len={len} draw={d}"), &buf, 0, k, len);
            chk_sc(&p, "C18", &format!("a=b+{k} len={len} draw={d}"), &buf, k, 0, len);
        }
    }
}

// ===========================================================================
// C19..C20 — length scans
// ===========================================================================

#[test]
fn c19_sc_length_scan_1_to_64() {
    let p = pair();
    let mut rng = Rng::new(19);
    for len in 1i32..=64 {
        let n = len as usize;
        for d in 0..40 {
            let buf: Vec<u32> = (0..2 * n)
                .map(|_| match d % 3 {
                    0 => rng.signed_normal_f32().to_bits(),
                    1 => rng.any_f32_bits(),
                    _ => rng.exp_biased_f32_bits(),
                })
                .collect();
            chk_sc(&p, "C19", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

#[test]
fn c20_sc_large_lengths() {
    let p = pair();
    let mut rng = Rng::new(20);
    for &len in &[4096i32, 65536] {
        let n = len as usize;
        for d in 0..4 {
            let buf: Vec<u32> = (0..2 * n)
                .map(|_| if d % 2 == 0 { rng.signed_normal_f32().to_bits() } else { rng.exp_biased_f32_bits() })
                .collect();
            chk_sc(&p, "C20", &format!("len={len} draw={d}"), &buf, 0, n, len);
        }
    }
}

// ===========================================================================
// C21..C24 — spectral_contrast called the way match.h declares it (double*)
// ===========================================================================

#[test]
fn c21_sc_double_caller_length_n() {
    let p = pair();
    let mut rng = Rng::new(21);
    for &n in &[1usize, 2, 3, 15, 16, 17, 33, 64] {
        for d in 0..400 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            chk_sc_f64(&p, "C21", &format!("n={n} len=n draw={d}"), &buf, 0, n, n as c_int);
        }
    }
}

#[test]
fn c22_sc_double_caller_length_2n() {
    let p = pair();
    let mut rng = Rng::new(22);
    for &n in &[1usize, 2, 3, 15, 16, 17, 33, 64] {
        for d in 0..400 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.signed_normal_f64().to_bits()).collect();
            chk_sc_f64(&p, "C22", &format!("n={n} len=2n draw={d}"), &buf, 0, n, 2 * n as c_int);
        }
    }
}

#[test]
fn c23_sc_double_caller_arbitrary_bits() {
    let p = pair();
    let mut rng = Rng::new(23);
    for &n in &[1usize, 2, 3, 15, 16, 17, 33, 64] {
        for d in 0..600 {
            let buf: Vec<u64> = (0..2 * n)
                .map(|_| if d % 2 == 0 { rng.any_f64_bits() } else { rng.exp_biased_f64_bits() })
                .collect();
            chk_sc_f64(&p, "C23", &format!("n={n} len=2n draw={d}"), &buf, 0, n, 2 * n as c_int);
            chk_sc_f64(&p, "C23", &format!("n={n} len=n draw={d}"), &buf, 0, n, n as c_int);
        }
    }
}

#[test]
fn c24_sc_double_caller_low_word_is_nan() {
    let p = pair();
    let mut rng = Rng::new(24);
    for &n in &[1usize, 2, 3, 15, 16, 17, 33] {
        for d in 0..500 {
            // High word: a benign finite f64 exponent (which is also a finite
            // f32 pattern). Low word: a NaN f32 pattern with a random payload.
            let buf: Vec<u64> = (0..2 * n)
                .map(|_| {
                    let hi = 0x3FE0_0000u64 | (rng.next_u64() & 0x000F_FFFF);
                    let lo = rng.qnan_f32_bits() as u64;
                    (hi << 32) | lo
                })
                .collect();
            for &len in &[n as c_int, 2 * n as c_int, (2 * n - 1) as c_int] {
                if len < 1 {
                    continue;
                }
                chk_sc_f64(&p, "C24", &format!("n={n} len={len} draw={d}"), &buf, 0, n, len);
            }
        }
    }
}

// ===========================================================================
// C25..C44 — `match`, the composed pipeline
//
// NOTE: `bins <= 0` is deliberately absent from every row here. The C `.so`
// deterministically SIGSEGVs for `bins == 0` (`differentiate(v, 0)` executes
// `v[-1] = 0`, which for a zero-length VLA is exactly `preprocess`'s saved
// return address) and for `bins < 0` (`memcpy` with a wrapped-around size).
// Those are ERRORS.md rows E8/E9 and are exercised out-of-process in
// tests/errors.rs; calling them here would kill the test binary.
// ===========================================================================

const MATCH_BINS: &[c_int] = &[1, 2, 3, 15, 16, 17, 31, 32, 33, 64, 257, 1000];

#[test]
fn c25_match_realistic_spectra() {
    let p = pair();
    let mut rng = Rng::new(25);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        let draws = if bins > 200 { 40 } else { 300 };
        for d in 0..draws {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            for &thr in &[0.25f64, 0.5, 1.0] {
                chk_match(&p, "C25", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c26_match_bins_scan_1_to_80() {
    let p = pair();
    let mut rng = Rng::new(26);
    for bins in 1i32..=80 {
        let n = bins as usize;
        for d in 0..60 {
            let buf: Vec<u64> = (0..2 * n)
                .map(|_| match d % 3 {
                    0 => rng.pos_normal_f64().to_bits(),
                    1 => rng.signed_normal_f64().to_bits(),
                    _ => rng.exp_biased_f64_bits(),
                })
                .collect();
            chk_match(&p, "C26", &format!("bins={bins} draw={d}"), &buf, 0, n, bins, 0.5);
        }
    }
}

#[test]
fn c27_match_identical_contents() {
    let p = pair();
    let mut rng = Rng::new(27);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..80 {
            let half: Vec<u64> = (0..n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            let mut buf = half.clone();
            buf.extend_from_slice(&half);
            for &thr in &[0.0f64, 0.5, 1.0, 2.0] {
                chk_match(&p, "C27", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c28_match_scaled_reference() {
    let p = pair();
    let mut rng = Rng::new(28);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let half: Vec<f64> = (0..n).map(|_| rng.pos_normal_f64()).collect();
            for &s in &[1e-6f64, 0.5, 1.0, 2.0, 1e6] {
                let mut buf: Vec<u64> = half.iter().map(|v| v.to_bits()).collect();
                buf.extend(half.iter().map(|v| (v * s).to_bits()));
                for &thr in &[0.5f64, 1.0] {
                    chk_match(&p, "C28", &format!("bins={bins} s={s} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
                }
            }
        }
    }
}

#[test]
fn c29_match_negated_reference() {
    let p = pair();
    let mut rng = Rng::new(29);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let half: Vec<f64> = (0..n).map(|_| rng.signed_normal_f64()).collect();
            let mut buf: Vec<u64> = half.iter().map(|v| v.to_bits()).collect();
            buf.extend(half.iter().map(|v| (-*v).to_bits()));
            for &thr in &[-1.0f64, 0.0, 0.5, 1.0] {
                chk_match(&p, "C29", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c30_match_test_all_zero() {
    let p = pair();
    let mut rng = Rng::new(30);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let mut buf: Vec<u64> = (0..n)
                .map(|_| if d % 2 == 0 { 0u64 } else { 0x8000_0000_0000_0000u64 })
                .collect();
            buf.extend((0..n).map(|_| rng.pos_normal_f64().to_bits()));
            for thr in thresholds() {
                chk_match(&p, "C30", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c31_match_reference_all_zero() {
    let p = pair();
    let mut rng = Rng::new(31);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let mut buf: Vec<u64> = (0..n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            buf.extend((0..n).map(|_| if d % 2 == 0 { 0u64 } else { 0x8000_0000_0000_0000u64 }));
            for thr in thresholds() {
                chk_match(&p, "C31", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c32_match_both_all_zero() {
    let p = pair();
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for &z in &[0u64, 0x8000_0000_0000_0000u64] {
            let buf: Vec<u64> = vec![z; 2 * n];
            for thr in thresholds() {
                chk_match(&p, "C32", &format!("bins={bins} thr={thr} z={z:#x}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c33_match_constant_spectra() {
    let p = pair();
    let mut rng = Rng::new(33);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let ka = rng.pos_normal_f64().to_bits();
            let kb = if d % 2 == 0 { ka } else { rng.pos_normal_f64().to_bits() };
            let mut buf = vec![ka; n];
            buf.extend(std::iter::repeat(kb).take(n));
            for &thr in &[0.5f64, 1.0] {
                chk_match(&p, "C33", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c34_match_ramps() {
    let p = pair();
    let mut rng = Rng::new(34);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..40 {
            let sa = rng.pos_normal_f64();
            let sb = rng.pos_normal_f64();
            let mut buf: Vec<u64> = (0..n).map(|i| ((i as f64 + 1.0) * sa).to_bits()).collect();
            buf.extend((0..n).map(|i| (((n - i) as f64) * sb).to_bits()));
            for &thr in &[0.25f64, 0.5, 1.0] {
                chk_match(&p, "C34", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c35_match_threshold_sweep() {
    let p = pair();
    let mut rng = Rng::new(35);
    for &bins in &[1i32, 2, 16, 17, 64] {
        let n = bins as usize;
        for d in 0..200 {
            let buf: Vec<u64> = (0..2 * n)
                .map(|_| match d % 3 {
                    0 => rng.pos_normal_f64().to_bits(),
                    1 => rng.signed_normal_f64().to_bits(),
                    _ => rng.exp_biased_f64_bits(),
                })
                .collect();
            for thr in thresholds() {
                chk_match(&p, "C35", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c36_match_arbitrary_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(36);
    for &bins in &[2i32, 3, 16, 17, 33, 64] {
        let n = bins as usize;
        for d in 0..500 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.any_f64_bits()).collect();
            for &thr in &[-1.0f64, 0.0, 0.5, 1.0, f64::NAN] {
                chk_match(&p, "C36", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c37_match_exponent_biased() {
    let p = pair();
    let mut rng = Rng::new(37);
    for &bins in &[1i32, 2, 3, 16, 17, 33, 64, 257] {
        let n = bins as usize;
        for d in 0..300 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.exp_biased_f64_bits()).collect();
            for &thr in &[0.5f64, 1.0] {
                chk_match(&p, "C37", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c38_match_single_special_element() {
    let p = pair();
    let mut rng = Rng::new(38);
    let specials: [u64; 6] = [
        0x7FF0_0000_0000_0000, // +inf
        0xFFF0_0000_0000_0000, // -inf
        0x7FF8_0000_0000_0000, // qNaN
        0x7FF0_0000_0000_0001, // sNaN
        0x0000_0000_0000_0001, // smallest subnormal
        0x7FEF_FFFF_FFFF_FFFF, // DBL_MAX
    ];
    for &bins in &[1i32, 2, 3, 16, 17, 33, 64] {
        let n = bins as usize;
        for d in 0..60 {
            let base: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            for &s in &specials {
                for which in 0..2 {
                    let mut buf = base.clone();
                    let i = rng.below(n) + which * n;
                    buf[i] = s;
                    for &thr in &[0.5f64, 1.0] {
                        chk_match(&p, "C38", &format!("bins={bins} s={s:#018x} i={i} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
                    }
                }
            }
        }
    }
}

#[test]
fn c39_match_aliased_arguments() {
    let p = pair();
    let mut rng = Rng::new(39);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..80 {
            let buf: Vec<u64> = (0..n)
                .map(|_| match d % 3 {
                    0 => rng.pos_normal_f64().to_bits(),
                    1 => rng.any_f64_bits(),
                    _ => rng.exp_biased_f64_bits(),
                })
                .collect();
            for thr in thresholds() {
                chk_match(&p, "C39", &format!("aliased bins={bins} thr={thr} draw={d}"), &buf, 0, 0, bins, thr);
            }
        }
    }
}

#[test]
fn c40_match_partial_overlap() {
    let p = pair();
    let mut rng = Rng::new(40);
    for &bins in &[1i32, 2, 3, 15, 16, 17, 33, 64] {
        let n = bins as usize;
        for d in 0..80 {
            let buf: Vec<u64> = (0..2 * n + 8)
                .map(|_| if d % 2 == 0 { rng.pos_normal_f64().to_bits() } else { rng.any_f64_bits() })
                .collect();
            for &thr in &[0.5f64, 1.0] {
                chk_match(&p, "C40", &format!("r=t+1 bins={bins} thr={thr} draw={d}"), &buf, 0, 1, bins, thr);
                let k = 1 + rng.below(n);
                chk_match(&p, "C40", &format!("r=t+{k} bins={bins} thr={thr} draw={d}"), &buf, 0, k, bins, thr);
                chk_match(&p, "C40", &format!("t=r+{k} bins={bins} thr={thr} draw={d}"), &buf, k, 0, bins, thr);
            }
        }
    }
}

// C41 is asserted inside `chk_match` for every row above.

#[test]
fn c42_match_spike_spectra() {
    let p = pair();
    let mut rng = Rng::new(42);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..60 {
            let mut buf: Vec<u64> = (0..2 * n).map(|_| (1.0 + rng.unit()).to_bits()).collect();
            let i = rng.below(n);
            buf[i] = 1e9f64.to_bits();
            if d % 2 == 0 {
                let j = rng.below(n);
                buf[n + j] = 1e9f64.to_bits();
            }
            for &thr in &[0.25f64, 0.5, 1.0] {
                chk_match(&p, "C42", &format!("bins={bins} i={i} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c43_match_period16_spectra() {
    let p = pair();
    let mut rng = Rng::new(43);
    for &bins in MATCH_BINS {
        let n = bins as usize;
        for d in 0..40 {
            let pat: Vec<f64> = (0..16).map(|_| rng.pos_normal_f64()).collect();
            let pat2: Vec<f64> = (0..16).map(|_| rng.pos_normal_f64()).collect();
            let mut buf: Vec<u64> = (0..n).map(|i| pat[i % 16].to_bits()).collect();
            buf.extend((0..n).map(|i| if d % 2 == 0 { pat[i % 16].to_bits() } else { pat2[i % 16].to_bits() }));
            for &thr in &[0.5f64, 1.0] {
                chk_match(&p, "C43", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

#[test]
fn c44_match_large_bins() {
    let p = pair();
    let mut rng = Rng::new(44);
    for &bins in &[4096i32] {
        let n = bins as usize;
        for d in 0..12 {
            let buf: Vec<u64> = (0..2 * n)
                .map(|_| match d % 3 {
                    0 => rng.pos_normal_f64().to_bits(),
                    1 => rng.signed_normal_f64().to_bits(),
                    _ => rng.exp_biased_f64_bits(),
                })
                .collect();
            for &thr in &[0.5f64, 1.0] {
                chk_match(&p, "C44", &format!("bins={bins} thr={thr} draw={d}"), &buf, 0, n, bins, thr);
            }
        }
    }
}

// ===========================================================================
// C45 / C46 — exhaustive cross-products over IEEE-754 class representatives.
//
// Randomized sampling can only ever hit special values by chance; these rows
// enumerate every *combination* of representative bit patterns for the smallest
// lengths, which is where the class interactions live (0*inf, inf-inf, inf/inf,
// subnormal*subnormal, sNaN vs qNaN payload precedence, +/-0 signs, ...).
// ===========================================================================

/// Every interesting f32 bit pattern, one per IEEE-754 class / boundary.
const F32_CLASSES: &[u32] = &[
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest subnormal
    0x0080_0000, // FLT_MIN (smallest normal)
    0x0080_0001,
    0x3300_0000, // 2^-24
    0x3F00_0000, // 0.5
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x4000_0000, // 2.0
    0x4B00_0000, // 2^23
    0x7E80_0000, // 2^126 (x*x overflows)
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // default qNaN
    0xFFC0_0000, // default qNaN, sign set
    0x7FC0_0001, // qNaN, payload 1
    0x7FFF_FFFF, // qNaN, max payload
    0x7F80_0001, // sNaN, payload 1
    0x7FBF_FFFF, // sNaN, max payload
    0xFF80_0001, // sNaN, sign set
];

/// Every interesting f64 bit pattern.
const F64_CLASSES: &[u64] = &[
    0x0000_0000_0000_0000, // +0
    0x8000_0000_0000_0000, // -0
    0x0000_0000_0000_0001, // smallest positive subnormal
    0x000F_FFFF_FFFF_FFFF, // largest subnormal
    0x0010_0000_0000_0000, // DBL_MIN
    0x3FE0_0000_0000_0000, // 0.5
    0x3FF0_0000_0000_0000, // 1.0
    0xBFF0_0000_0000_0000, // -1.0
    0x4000_0000_0000_0000, // 2.0
    0x4330_0000_0000_0000, // 2^52
    0x7FEF_FFFF_FFFF_FFFF, // DBL_MAX
    0xFFEF_FFFF_FFFF_FFFF, // -DBL_MAX
    0x7FF0_0000_0000_0000, // +inf
    0xFFF0_0000_0000_0000, // -inf
    0x7FF8_0000_0000_0000, // qNaN
    0x7FF0_0000_0000_0001, // sNaN
];

#[test]
fn c45_sc_exhaustive_class_cross_product_len1() {
    let p = pair();
    for &a in F32_CLASSES {
        for &b in F32_CLASSES {
            let buf = vec![a, b];
            chk_sc(&p, "C45", &format!("len=1 a={a:#010x} b={b:#010x}"), &buf, 0, 1, 1);
            // aliased, too
            chk_sc(&p, "C45", &format!("len=1 aliased a={a:#010x}"), &buf, 0, 0, 1);
        }
    }
}

#[test]
fn c45_sc_exhaustive_class_cross_product_len2() {
    let p = pair();
    for &a0 in F32_CLASSES {
        for &a1 in F32_CLASSES {
            for &b0 in F32_CLASSES {
                for &b1 in F32_CLASSES {
                    let buf = vec![a0, a1, b0, b1];
                    chk_sc(
                        &p,
                        "C45",
                        &format!("len=2 a=[{a0:#010x},{a1:#010x}] b=[{b0:#010x},{b1:#010x}]"),
                        &buf,
                        0,
                        2,
                        2,
                    );
                }
            }
        }
    }
}

#[test]
fn c45_sc_exhaustive_class_cross_product_overlap() {
    let p = pair();
    // b = a + 1 over a 3-element window: every triple of classes.
    for &v0 in F32_CLASSES {
        for &v1 in F32_CLASSES {
            for &v2 in F32_CLASSES {
                let buf = vec![v0, v1, v2];
                chk_sc(
                    &p,
                    "C45",
                    &format!("overlap len=2 v=[{v0:#010x},{v1:#010x},{v2:#010x}]"),
                    &buf,
                    0,
                    1,
                    2,
                );
            }
        }
    }
}

#[test]
fn c46_match_exhaustive_class_cross_product_bins2() {
    let p = pair();
    for &t0 in F64_CLASSES {
        for &t1 in F64_CLASSES {
            for &r0 in F64_CLASSES {
                for &r1 in F64_CLASSES {
                    let buf = vec![t0, t1, r0, r1];
                    for &thr in &[0.5f64, 1.0, -1.0, f64::NAN] {
                        chk_match(
                            &p,
                            "C46",
                            &format!("bins=2 t=[{t0:#018x},{t1:#018x}] r=[{r0:#018x},{r1:#018x}] thr={thr}"),
                            &buf,
                            0,
                            2,
                            2,
                            thr,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn c46_match_exhaustive_class_cross_product_bins1_and_3() {
    let p = pair();
    for &t0 in F64_CLASSES {
        for &r0 in F64_CLASSES {
            let buf = vec![t0, r0];
            for thr in thresholds() {
                chk_match(&p, "C46", &format!("bins=1 t={t0:#018x} r={r0:#018x} thr={thr}"), &buf, 0, 1, 1, thr);
            }
        }
    }
    // bins == 3: odd length, so `spectral_contrast` reads the low word of the
    // last touched double only.
    for &t0 in F64_CLASSES {
        for &t1 in F64_CLASSES {
            for &r0 in F64_CLASSES {
                let buf = vec![t0, t1, t0, r0, t1, r0];
                for &thr in &[0.5f64, 1.0] {
                    chk_match(
                        &p,
                        "C46",
                        &format!("bins=3 t0={t0:#018x} t1={t1:#018x} r0={r0:#018x} thr={thr}"),
                        &buf,
                        0,
                        3,
                        3,
                        thr,
                    );
                }
            }
        }
    }
}
