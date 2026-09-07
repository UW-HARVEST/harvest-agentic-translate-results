//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md` (C1 .. C32), each driven with many
//! randomized inputs from a fixed-seed SplitMix64 so any divergence is
//! reproducible. Both `.so`s are loaded with `libloading`; only exported
//! `extern "C"` symbols are ever called.
//!
//! Comparison is always on raw IEEE-754 bit patterns, plus the full contents
//! of every in-place-mutated buffer.

mod common;
use common::*;
use std::ffi::c_int;

/// How many randomized inputs per (row, shape) combination.
/// Override with `REPS=<n> cargo test` to widen the sweep.
fn reps() -> usize {
    std::env::var("REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
}

// ---------------------------------------------------------------------------
// Data generators — one per value class that CONFIGS.md identifies.
// ---------------------------------------------------------------------------

/// Plausible normalized spectrum-ish f32 data.
fn gen_f32_realistic(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.range(-4.0, 4.0) as f32).collect()
}

/// Non-negative energy bins.
fn gen_f32_energy(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.range(0.0, 1.0) as f32).collect()
}

/// Every bit pattern is fair game: NaNs, infinities, denormals, huge, tiny.
fn gen_f32_raw(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.raw_f32()).collect()
}

fn gen_f32_zeros(rng: &mut Rng, n: usize, mixed_signs: bool) -> Vec<f32> {
    (0..n)
        .map(|_| {
            if mixed_signs && rng.next_u64() & 1 == 1 {
                -0.0f32
            } else {
                0.0f32
            }
        })
        .collect()
}

fn gen_f32_with_inf(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| match rng.below(4) {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            _ => rng.range(-2.0, 2.0) as f32,
        })
        .collect()
}

/// Quiet NaNs with assorted payloads and signs, sprinkled among finite values.
fn gen_f32_qnan(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| {
            if rng.below(3) == 0 {
                let payload = (rng.next_u32() & 0x003F_FFFF) | 0x0000_0001;
                let sign = (rng.next_u32() & 1) << 31;
                f32::from_bits(sign | 0x7FC0_0000 | payload)
            } else {
                rng.range(-2.0, 2.0) as f32
            }
        })
        .collect()
}

/// Signalling NaNs (quiet bit clear, mantissa non-zero).
fn gen_f32_snan(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| {
            if rng.below(3) == 0 {
                let payload = (rng.next_u32() & 0x003F_FFFF) | 0x0000_0001;
                let sign = (rng.next_u32() & 1) << 31;
                f32::from_bits(sign | 0x7F80_0000 | payload) // quiet bit clear
            } else {
                rng.range(-2.0, 2.0) as f32
            }
        })
        .collect()
}

/// Subnormal f32s — squaring them underflows to zero, so the magnitude becomes
/// 0 even though no element is 0 (E9 / C10).
fn gen_f32_subnormal(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| {
            let m = (rng.next_u32() & 0x007F_FFFF) | 1;
            let sign = (rng.next_u32() & 1) << 31;
            f32::from_bits(sign | m)
        })
        .collect()
}

/// Huge f32s — `mulss` overflows to +inf in *single* precision (C11).
fn gen_f32_huge(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| {
            let s = if rng.next_u64() & 1 == 1 { -1.0 } else { 1.0 };
            (s * rng.range(1e30, 3.4e38)) as f32
        })
        .collect()
}

/// All the f32 generators, for rows that sweep the value classes.
type Gen32 = (&'static str, fn(&mut Rng, usize) -> Vec<f32>);
fn all_gen32() -> Vec<Gen32> {
    vec![
        ("realistic", gen_f32_realistic),
        ("energy", gen_f32_energy),
        ("raw", gen_f32_raw),
        ("inf", gen_f32_with_inf),
        ("qnan", gen_f32_qnan),
        ("snan", gen_f32_snan),
        ("subnormal", gen_f32_subnormal),
        ("huge", gen_f32_huge),
    ]
}

// --- f64 generators (for `match`) ------------------------------------------

fn gen_f64_spectrum(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.range(0.0, 1.0)).collect()
}

fn gen_f64_signed(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.range(-1.0, 1.0)).collect()
}

/// Arbitrary raw f64 bit patterns. The f32 view of a random f64 mantissa is a
/// NaN roughly 1 time in 256, so this row is the highest-yield one for the
/// `float_t` type-confusion paths (C27).
fn gen_f64_raw(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.raw_f64()).collect()
}

fn gen_f64_constant(rng: &mut Rng, n: usize) -> Vec<f64> {
    let c = rng.range(-10.0, 10.0);
    vec![c; n]
}

fn gen_f64_ramp(rng: &mut Rng, n: usize) -> Vec<f64> {
    let step = rng.range(-2.0, 2.0);
    let base = rng.range(-5.0, 5.0);
    (0..n).map(|i| base + step * i as f64).collect()
}

fn gen_f64_impulse(rng: &mut Rng, n: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];
    if n > 0 {
        let k = rng.below(n);
        v[k] = rng.range(1.0, 1e6);
    }
    v
}

fn gen_f64_step(rng: &mut Rng, n: usize) -> Vec<f64> {
    let k = if n > 0 { rng.below(n) } else { 0 };
    let lo = rng.range(-1.0, 1.0);
    let hi = rng.range(-1.0, 1.0);
    (0..n).map(|i| if i < k { lo } else { hi }).collect()
}

fn gen_f64_alternating(rng: &mut Rng, n: usize) -> Vec<f64> {
    let a = rng.range(0.1, 100.0);
    (0..n)
        .map(|i| if i % 2 == 0 { a } else { -a })
        .collect()
}

fn gen_f64_zeros(rng: &mut Rng, n: usize, mixed: bool) -> Vec<f64> {
    (0..n)
        .map(|_| {
            if mixed && rng.next_u64() & 1 == 1 {
                -0.0
            } else {
                0.0
            }
        })
        .collect()
}

fn gen_f64_with_specials(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| match rng.below(8) {
            0 => f64::INFINITY,
            1 => f64::NEG_INFINITY,
            2 => f64::NAN,
            3 => f64::from_bits(0x7FF8_0000_0000_0001 | ((rng.next_u64() & 0xFFFF) << 4)),
            4 => f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
            5 => -0.0,
            _ => rng.range(-3.0, 3.0),
        })
        .collect()
}

/// Very large magnitudes — the *high* 32-bit half of each `double` then decodes
/// to an f32 NaN or infinity (C28).
fn gen_f64_huge(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| {
            let s = if rng.next_u64() & 1 == 1 { -1.0 } else { 1.0 };
            s * rng.range(1e290, 1e308)
        })
        .collect()
}

/// Very small magnitudes — the high half decodes to an f32 zero/denormal.
fn gen_f64_tiny(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| {
            let s = if rng.next_u64() & 1 == 1 { -1.0 } else { 1.0 };
            s * rng.range(1e-308, 1e-290)
        })
        .collect()
}

type Gen64 = (&'static str, fn(&mut Rng, usize) -> Vec<f64>);
fn all_gen64() -> Vec<Gen64> {
    vec![
        ("spectrum", gen_f64_spectrum),
        ("signed", gen_f64_signed),
        ("raw", gen_f64_raw),
        ("constant", gen_f64_constant),
        ("ramp", gen_f64_ramp),
        ("impulse", gen_f64_impulse),
        ("step", gen_f64_step),
        ("alternating", gen_f64_alternating),
        ("specials", gen_f64_with_specials),
        ("huge", gen_f64_huge),
        ("tiny", gen_f64_tiny),
    ]
}

// ===========================================================================
// C1 — spectral_contrast, length == 0
// ===========================================================================

#[test]
fn c1_spectral_length_zero() {
    let p = load();
    let mut rng = Rng::new();
    for _ in 0..reps() {
        let a = gen_f32_raw(&mut rng, 8);
        let b = gen_f32_raw(&mut rng, 8);
        diff_spectral(&p, "C1 len=0", &a, &b, 0, Alias::Disjoint);
    }
    // Null pointers with length 0 — nothing is dereferenced.
    let rc = unsafe { (p.c.spectral_fn)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    let rr = unsafe { (p.rs.spectral_fn)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "C1 NULL/len=0: C={rc:?} Rust={rr:?}"
    );
}

// ===========================================================================
// C2 — spectral_contrast, length == 1, arbitrary bit patterns
// ===========================================================================

#[test]
fn c2_spectral_length_one() {
    let p = load();
    let mut rng = Rng::new();
    for (name, g) in all_gen32() {
        for _ in 0..reps() {
            let a = g(&mut rng, 1);
            let b = g(&mut rng, 1);
            diff_spectral(&p, &format!("C2 len=1 {name}"), &a, &b, 1, Alias::Disjoint);
        }
    }
}

// ===========================================================================
// C3 — spectral_contrast, lengths 2..=8 exhaustive
// ===========================================================================

#[test]
fn c3_spectral_small_lengths() {
    let p = load();
    let mut rng = Rng::new();
    for len in 2..=8i32 {
        for (name, g) in all_gen32() {
            for _ in 0..reps() {
                let a = g(&mut rng, len as usize);
                let b = g(&mut rng, len as usize);
                diff_spectral(
                    &p,
                    &format!("C3 len={len} {name}"),
                    &a,
                    &b,
                    len,
                    Alias::Disjoint,
                );
            }
        }
    }
}

// ===========================================================================
// C4 / C5 — spectral_contrast, boundary and large lengths
// ===========================================================================

#[test]
fn c4_c5_spectral_boundary_and_large_lengths() {
    let p = load();
    let mut rng = Rng::new();
    for len in interesting_lengths() {
        for (name, g) in all_gen32() {
            let n_reps = if len > 256 { 4 } else { reps() / 2 };
            for _ in 0..n_reps {
                let a = g(&mut rng, len as usize);
                let b = g(&mut rng, len as usize);
                diff_spectral(
                    &p,
                    &format!("C4/C5 len={len} {name}"),
                    &a,
                    &b,
                    len,
                    Alias::Disjoint,
                );
            }
        }
    }
}

// ===========================================================================
// C6 — zero magnitude (unguarded 0.0 / 0.0)
// ===========================================================================

#[test]
fn c6_spectral_zero_magnitude() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1, 2, 3, 15, 16, 17, 64] {
        for mixed in [false, true] {
            let a = gen_f32_zeros(&mut rng, len, mixed);
            let b = gen_f32_zeros(&mut rng, len, mixed);
            diff_spectral(
                &p,
                &format!("C6 zeros len={len} mixed={mixed}"),
                &a,
                &b,
                len as c_int,
                Alias::Disjoint,
            );
            // one side zero, other side finite
            let c = gen_f32_realistic(&mut rng, len);
            diff_spectral(
                &p,
                &format!("C6 zero/finite len={len}"),
                &a,
                &c,
                len as c_int,
                Alias::Disjoint,
            );
            diff_spectral(
                &p,
                &format!("C6 finite/zero len={len}"),
                &c,
                &a,
                len as c_int,
                Alias::Disjoint,
            );
        }
    }
}

// ===========================================================================
// C7 / C8 / C9 / C10 / C11 — special value classes, swept over lengths
// ===========================================================================

#[test]
fn c7_to_c11_spectral_special_values() {
    let p = load();
    let mut rng = Rng::new();
    let gens: Vec<Gen32> = vec![
        ("C7 inf", gen_f32_with_inf),
        ("C8 qnan", gen_f32_qnan),
        ("C9 snan", gen_f32_snan),
        ("C10 subnormal", gen_f32_subnormal),
        ("C11 huge", gen_f32_huge),
    ];
    for (name, g) in gens {
        for len in [1, 2, 3, 4, 7, 8, 15, 16, 17, 32, 33, 64, 128] {
            for _ in 0..reps() {
                let a = g(&mut rng, len);
                let b = g(&mut rng, len);
                diff_spectral(
                    &p,
                    &format!("{name} len={len}"),
                    &a,
                    &b,
                    len as c_int,
                    Alias::Disjoint,
                );
                // mixed: special on one side only
                let f = gen_f32_realistic(&mut rng, len);
                diff_spectral(
                    &p,
                    &format!("{name}/finite len={len}"),
                    &a,
                    &f,
                    len as c_int,
                    Alias::Disjoint,
                );
                diff_spectral(
                    &p,
                    &format!("finite/{name} len={len}"),
                    &f,
                    &b,
                    len as c_int,
                    Alias::Disjoint,
                );
            }
        }
    }
}

// ===========================================================================
// C12 — fully aliased arguments
// ===========================================================================

#[test]
fn c12_spectral_aliased_same_pointer() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1, 2, 3, 15, 16, 17, 33, 64] {
        for (name, g) in all_gen32() {
            for _ in 0..reps() / 2 {
                let a = g(&mut rng, len);
                diff_spectral(
                    &p,
                    &format!("C12 alias len={len} {name}"),
                    &a,
                    &a,
                    len as c_int,
                    Alias::Same,
                );
            }
        }
    }
}

// ===========================================================================
// C13 — partially overlapping arguments
// ===========================================================================

#[test]
fn c13_spectral_partial_overlap() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1, 2, 3, 8, 16, 17, 33] {
        for k in [1usize, 2, 3] {
            for (name, g) in all_gen32() {
                for _ in 0..reps() / 4 {
                    let a = g(&mut rng, len + k);
                    diff_spectral(
                        &p,
                        &format!("C13 overlapB len={len} k={k} {name}"),
                        &a,
                        &a,
                        len as c_int,
                        Alias::OffsetB(k),
                    );
                    diff_spectral(
                        &p,
                        &format!("C13 overlapA len={len} k={k} {name}"),
                        &a,
                        &a,
                        len as c_int,
                        Alias::OffsetA(k),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// C14 — negative length (E7)
// ===========================================================================

#[test]
fn c14_spectral_negative_length() {
    let p = load();
    let mut rng = Rng::new();
    for len in [-1i32, -2, -7, -1000, -65536, i32::MIN, i32::MIN + 1] {
        for _ in 0..reps() / 4 {
            let a = gen_f32_raw(&mut rng, 8);
            let b = gen_f32_raw(&mut rng, 8);
            diff_spectral(&p, &format!("C14 len={len}"), &a, &b, len, Alias::Disjoint);
        }
    }
}

// ===========================================================================
// C15 — match, bins == 0.
//
// MEASURED, NOT ASSUMED: this is *not* a valid configuration. C `match` with
// `bins == 0` allocates two zero-length VLAs at `%rsp`, and then
// `differentiate(v, 0)` executes its unguarded trailing store `v[length-1] = 0`
// as `v[-1] = 0`, i.e. it writes eight zero bytes to `%rsp - 8` — which is
// exactly the slot holding `preprocess`'s return address into `match`.
// `preprocess` therefore returns to address 0 and the process dies with
// SIGSEGV, for every `threshold` (the energy gate can never fire first,
// because it needs `0.0 < threshold * 0.0`, and that product is either `0.0`
// or a QNaN).
//
// Verified standalone against the C `.so`: exit status 139 = SIGSEGV.
//
// So `bins <= 0` belongs to the *error* surface, not the configuration
// surface. It is covered by `tests/errors.rs` (rows E5/E8/E14/E15), which runs
// the call in a forked child process so the crash can be observed instead of
// taking the test runner down with it. Nothing to do here.
// ===========================================================================

// ===========================================================================
// C16 — match, bins == 1 (differentiate zeroes the single element)
// ===========================================================================

#[test]
fn c16_match_bins_one() {
    let p = load();
    let mut rng = Rng::new();
    for (name, g) in all_gen64() {
        for th in threshold_classes() {
            for _ in 0..4 {
                let t = g(&mut rng, 1);
                let r = g(&mut rng, 1);
                diff_match(
                    &p,
                    &format!("C16 bins=1 {name} th={:016x}", th.to_bits()),
                    &t,
                    &r,
                    1,
                    th,
                    false,
                );
            }
        }
    }
}

// ===========================================================================
// C17 — match, bins 2..=8 exhaustive (odd and even, E17)
// ===========================================================================

#[test]
fn c17_match_small_bins() {
    let p = load();
    let mut rng = Rng::new();
    for bins in 2..=8i32 {
        for (name, g) in all_gen64() {
            for _ in 0..reps() {
                let t = g(&mut rng, bins as usize);
                let r = g(&mut rng, bins as usize);
                let th = rng.range(-0.5, 1.5);
                diff_match(&p, &format!("C17 bins={bins} {name}"), &t, &r, bins, th, false);
            }
        }
    }
}

// ===========================================================================
// C18 — match, bins straddling N_SMOOTH == 16 (E18 attenuated tail)
// ===========================================================================

#[test]
fn c18_match_nsmooth_boundary() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [14i32, 15, 16, 17, 18] {
        for (name, g) in all_gen64() {
            for _ in 0..reps() {
                let t = g(&mut rng, bins as usize);
                let r = g(&mut rng, bins as usize);
                for th in [-1.0, 0.0, 0.5, 1.0, rng.range(-2.0, 2.0)] {
                    diff_match(&p, &format!("C18 bins={bins} {name}"), &t, &r, bins, th, false);
                }
            }
        }
    }
}

// ===========================================================================
// C19 — match, medium/large bins
// ===========================================================================

#[test]
fn c19_match_large_bins() {
    let p = load();
    let mut rng = Rng::new();
    for bins in interesting_lengths() {
        for (name, g) in all_gen64() {
            let n_reps = if bins > 256 { 3 } else { reps() / 2 };
            for _ in 0..n_reps {
                let t = g(&mut rng, bins as usize);
                let r = g(&mut rng, bins as usize);
                let th = rng.range(-1.0, 1.5);
                diff_match(&p, &format!("C19 bins={bins} {name}"), &t, &r, bins, th, false);
            }
        }
    }
}

// ===========================================================================
// C20 — the energy gate is TAKEN (early `return 0`, E1)
// ===========================================================================

#[test]
fn c20_match_gate_taken() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 8, 16, 17, 64] {
        for _ in 0..reps() {
            let r = gen_f64_spectrum(&mut rng, bins as usize);
            // scale `test` far below `reference` so total(test) < 1.0*total(ref)
            let t: Vec<f64> = r.iter().map(|x| x * 1e-6).collect();
            for th in [1.0, 0.5, 0.9, 1e-3] {
                diff_match(&p, &format!("C20 bins={bins} th={th}"), &t, &r, bins, th, false);
            }
        }
    }
}

// ===========================================================================
// C21 — the energy gate is NOT taken
// ===========================================================================

#[test]
fn c21_match_gate_not_taken() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 8, 16, 17, 64] {
        for _ in 0..reps() {
            let r = gen_f64_spectrum(&mut rng, bins as usize);
            let t: Vec<f64> = r.iter().map(|x| x * 1e6).collect();
            for th in [1.0, 0.5, 0.0, -1.0] {
                diff_match(&p, &format!("C21 bins={bins} th={th}"), &t, &r, bins, th, false);
            }
        }
    }
}

// ===========================================================================
// C22 — the energy gate comparison is UNORDERED (E2 / E3)
// ===========================================================================

#[test]
fn c22_match_gate_unordered() {
    let p = load();
    let mut rng = Rng::new();
    let nans = [
        f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF4_DEAD_BEEF_0000),
    ];
    for bins in [1i32, 2, 8, 16, 17, 33] {
        for _ in 0..reps() / 2 {
            let base = gen_f64_spectrum(&mut rng, bins as usize);

            // (a) threshold is NaN
            for &nan in &nans {
                diff_match(
                    &p,
                    &format!("C22a bins={bins}"),
                    &base,
                    &base,
                    bins,
                    nan,
                    false,
                );
            }

            // (b) threshold = +/-inf with total(reference) == 0  => inf*0 = QNaN
            let zeros = vec![0.0f64; bins as usize];
            for th in [f64::INFINITY, f64::NEG_INFINITY] {
                diff_match(
                    &p,
                    &format!("C22b bins={bins} th={th}"),
                    &base,
                    &zeros,
                    bins,
                    th,
                    false,
                );
                diff_match(
                    &p,
                    &format!("C22b' bins={bins} th={th}"),
                    &zeros,
                    &base,
                    bins,
                    th,
                    false,
                );
            }

            // (c) total(test) is NaN, several NaN positions and payloads
            for pos in 0..(bins as usize).min(4) {
                let mut t = base.clone();
                t[pos] = nans[pos % nans.len()];
                for th in [0.0, 0.5, 1.0, f64::INFINITY] {
                    diff_match(
                        &p,
                        &format!("C22c bins={bins} pos={pos} th={th}"),
                        &t,
                        &base,
                        bins,
                        th,
                        false,
                    );
                }
            }

            // (d) multiple NaNs in one array: which payload survives the
            //     accumulator depends on the exact SSE operand roles.
            if bins >= 3 {
                let mut t = base.clone();
                t[0] = nans[1];
                t[1] = nans[2];
                t[bins as usize - 1] = nans[4];
                diff_match(
                    &p,
                    &format!("C22d bins={bins}"),
                    &t,
                    &base,
                    bins,
                    0.5,
                    false,
                );
            }

            // (e) +inf and -inf in the same array => inf + (-inf) = QNaN
            if bins >= 2 {
                let mut t = base.clone();
                t[0] = f64::INFINITY;
                t[1] = f64::NEG_INFINITY;
                diff_match(
                    &p,
                    &format!("C22e bins={bins}"),
                    &t,
                    &base,
                    bins,
                    0.5,
                    false,
                );
            }
        }
    }
}

// ===========================================================================
// C23 — match(test, test, bins, th): identical spectra via the same pointer
// ===========================================================================

#[test]
fn c23_match_same_pointer() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 8, 15, 16, 17, 33, 64] {
        for (name, g) in all_gen64() {
            for _ in 0..reps() / 2 {
                let t = g(&mut rng, bins as usize);
                for th in [-1.0, 0.0, 0.5, 1.0, 1.0 + f64::EPSILON, 2.0] {
                    diff_match(
                        &p,
                        &format!("C23 bins={bins} {name} th={th}"),
                        &t,
                        &t,
                        bins,
                        th,
                        true,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// C24 — constant spectra => differentiate yields all zeros => zero magnitude
// ===========================================================================

#[test]
fn c24_match_constant_spectra() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 8, 15, 16, 17, 33, 64, 128] {
        for _ in 0..reps() {
            let t = gen_f64_constant(&mut rng, bins as usize);
            let r = gen_f64_constant(&mut rng, bins as usize);
            for th in [-1.0, -0.0, 0.0, 0.5, 1.0, 2.0] {
                diff_match(&p, &format!("C24 bins={bins} th={th}"), &t, &r, bins, th, false);
            }
            // exactly zero everywhere
            let z = vec![0.0f64; bins as usize];
            let nz = gen_f64_zeros(&mut rng, bins as usize, true);
            diff_match(&p, &format!("C24z bins={bins}"), &z, &z, bins, 0.5, false);
            diff_match(&p, &format!("C24nz bins={bins}"), &nz, &nz, bins, 0.5, false);
            diff_match(&p, &format!("C24zn bins={bins}"), &z, &nz, bins, -1.0, false);
        }
    }
}

// ===========================================================================
// C25 — shape family: ramp / impulse / step / alternating
// ===========================================================================

#[test]
fn c25_match_shapes() {
    let p = load();
    let mut rng = Rng::new();
    let shapes: Vec<Gen64> = vec![
        ("ramp", gen_f64_ramp),
        ("impulse", gen_f64_impulse),
        ("step", gen_f64_step),
        ("alternating", gen_f64_alternating),
    ];
    for bins in [2i32, 3, 15, 16, 17, 32, 33, 64] {
        for (n1, g1) in &shapes {
            for (n2, g2) in &shapes {
                for _ in 0..reps() / 2 {
                    let t = g1(&mut rng, bins as usize);
                    let r = g2(&mut rng, bins as usize);
                    let th = rng.range(-1.0, 1.5);
                    diff_match(
                        &p,
                        &format!("C25 bins={bins} {n1}/{n2}"),
                        &t,
                        &r,
                        bins,
                        th,
                        false,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// C26 — spectra containing inf / qNaN / sNaN
// ===========================================================================

#[test]
fn c26_match_special_spectra() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 8, 15, 16, 17, 33, 64] {
        for _ in 0..reps() {
            let t = gen_f64_with_specials(&mut rng, bins as usize);
            let r = gen_f64_with_specials(&mut rng, bins as usize);
            let th = rng.range(-2.0, 2.0);
            diff_match(&p, &format!("C26 bins={bins}"), &t, &r, bins, th, false);
            let f = gen_f64_spectrum(&mut rng, bins as usize);
            diff_match(&p, &format!("C26 spec/fin bins={bins}"), &t, &f, bins, th, false);
            diff_match(&p, &format!("C26 fin/spec bins={bins}"), &f, &r, bins, th, false);
        }
    }
}

// ===========================================================================
// C27 — arbitrary raw f64 bit patterns (highest-yield row)
// ===========================================================================

#[test]
fn c27_match_raw_bit_patterns() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 4, 5, 6, 7, 8, 15, 16, 17, 33, 64, 65] {
        for _ in 0..(reps() * 4) {
            let t = gen_f64_raw(&mut rng, bins as usize);
            let r = gen_f64_raw(&mut rng, bins as usize);
            let th = if rng.next_u64() & 1 == 0 {
                rng.range(-2.0, 2.0)
            } else {
                rng.raw_f64()
            };
            diff_match(&p, &format!("C27 bins={bins}"), &t, &r, bins, th, false);
        }
    }
}

// ===========================================================================
// C28 — huge / tiny f64 values (the *high* half of each double becomes an
//       f32 NaN / inf / denormal under the type confusion)
// ===========================================================================

#[test]
fn c28_match_huge_and_tiny() {
    let p = load();
    let mut rng = Rng::new();
    let gens: Vec<Gen64> = vec![("huge", gen_f64_huge), ("tiny", gen_f64_tiny)];
    for bins in [1i32, 2, 3, 8, 15, 16, 17, 33, 64] {
        for (n1, g1) in &gens {
            for (n2, g2) in &gens {
                for _ in 0..reps() {
                    let t = g1(&mut rng, bins as usize);
                    let r = g2(&mut rng, bins as usize);
                    for th in [-1.0, 0.0, 0.5, 1.0] {
                        diff_match(
                            &p,
                            &format!("C28 bins={bins} {n1}/{n2} th={th}"),
                            &t,
                            &r,
                            bins,
                            th,
                            false,
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// C29 — threshold exactly equal to the produced contrast (tests `>=` vs `>`)
// ===========================================================================

#[test]
fn c29_match_threshold_equals_contrast() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [2i32, 3, 8, 16, 17, 33, 64] {
        for _ in 0..reps() {
            let t = gen_f64_spectrum(&mut rng, bins as usize);
            let r = gen_f64_spectrum(&mut rng, bins as usize);

            // Reproduce, through the exported low-level entry point, exactly
            // what `match` computes, so we learn the contrast value and can
            // then feed it back as `threshold`.
            let contrast = pipeline_contrast(&p.c, &t, &r, bins);

            for th in [
                contrast,
                f64::from_bits(contrast.to_bits().wrapping_sub(1)),
                f64::from_bits(contrast.to_bits().wrapping_add(1)),
            ] {
                if th.is_nan() {
                    continue;
                }
                diff_match(
                    &p,
                    &format!("C29 bins={bins} th=contrast{:+}", 0),
                    &t,
                    &r,
                    bins,
                    th,
                    false,
                );
            }
        }
    }
}

// ===========================================================================
// C30 — threshold boundary bit patterns
// ===========================================================================

#[test]
fn c30_match_threshold_boundaries() {
    let p = load();
    let mut rng = Rng::new();
    let thresholds = {
        let mut v = threshold_classes();
        v.extend_from_slice(&[
            f64::from_bits(0x8000_0000_0000_0000), // -0.0
            f64::from_bits(0x0000_0000_0000_0001), // smallest subnormal
            f64::from_bits(0x8000_0000_0000_0001), // -smallest subnormal
            f64::MIN,
            -f64::MIN_POSITIVE,
            1.0 - f64::EPSILON / 2.0,
        ]);
        v
    };
    for bins in [1i32, 2, 8, 16, 17, 33] {
        for (name, g) in all_gen64() {
            let t = g(&mut rng, bins as usize);
            let r = g(&mut rng, bins as usize);
            for &th in &thresholds {
                diff_match(
                    &p,
                    &format!("C30 bins={bins} {name} th={:016x}", th.to_bits()),
                    &t,
                    &r,
                    bins,
                    th,
                    false,
                );
            }
        }
    }
}

// ===========================================================================
// C31 — composed check: drive match's pipeline through the EXPORTED
//       spectral_contrast and confirm the two agree in both libraries.
// ===========================================================================

/// Replicate `match`'s preprocessing in the harness (pure Rust, no library
/// call), then call the *exported* `spectral_contrast` on the resulting
/// `double` buffers reinterpreted as `float` — exactly what C `match` does.
fn pipeline_contrast(im: &Impl, test: &[f64], reference: &[f64], bins: c_int) -> f64 {
    let n = bins.max(0) as usize;
    let mut t = test[..n].to_vec();
    let mut r = reference[..n].to_vec();
    preprocess_ref(&mut t);
    preprocess_ref(&mut r);
    unsafe { (im.spectral_fn)(t.as_mut_ptr() as *mut f32, r.as_mut_ptr() as *mut f32, bins) }
}

/// `preprocess` from match.c: smoothen, differentiate, smoothen (f64 domain).
fn preprocess_ref(v: &mut [f64]) {
    smoothen_ref(v);
    differentiate_ref(v);
    smoothen_ref(v);
}

fn smoothen_ref(v: &mut [f64]) {
    let len = v.len();
    for i in 0..len {
        let mut sum = 0.0f64;
        let mut j = 0usize;
        while j < 16 && i + j < len {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / 16.0;
    }
}

fn differentiate_ref(v: &mut [f64]) {
    let len = v.len();
    if len == 0 {
        return;
    }
    for i in 0..len - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[len - 1] = 0.0;
}

#[test]
fn c31_pipeline_via_exported_low_level_entry_point() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 8, 15, 16, 17, 33, 64] {
        for (name, g) in all_gen64() {
            for _ in 0..reps() / 2 {
                let t = g(&mut rng, bins as usize);
                let r = g(&mut rng, bins as usize);

                // The exported low-level entry point must agree between the
                // two libraries on the pipeline's intermediate buffers.
                let mut tc = t.clone();
                let mut rc = r.clone();
                preprocess_ref(&mut tc);
                preprocess_ref(&mut rc);
                let a32: Vec<f32> = {
                    let s = unsafe {
                        std::slice::from_raw_parts(tc.as_ptr() as *const f32, tc.len() * 2)
                    };
                    s[..bins.max(0) as usize].to_vec()
                };
                let b32: Vec<f32> = {
                    let s = unsafe {
                        std::slice::from_raw_parts(rc.as_ptr() as *const f32, rc.len() * 2)
                    };
                    s[..bins.max(0) as usize].to_vec()
                };
                diff_spectral(
                    &p,
                    &format!("C31 bins={bins} {name}"),
                    &a32,
                    &b32,
                    bins,
                    Alias::Disjoint,
                );

                // And `match` itself, gated below the contrast so the gate
                // never short-circuits: threshold = -inf forces the full path.
                let contrast_c = pipeline_contrast(&p.c, &t, &r, bins);
                let contrast_rs = pipeline_contrast(&p.rs, &t, &r, bins);
                assert_eq!(
                    contrast_c.to_bits(),
                    contrast_rs.to_bits(),
                    "C31 pipeline contrast diverged: bins={bins} {name} C={:016x} R={:016x}",
                    contrast_c.to_bits(),
                    contrast_rs.to_bits()
                );
                diff_match(
                    &p,
                    &format!("C31 match bins={bins} {name}"),
                    &t,
                    &r,
                    bins,
                    f64::NEG_INFINITY,
                    false,
                );
            }
        }
    }
}

// ===========================================================================
// C32 — repeated calls: no hidden state, and the twice-mutated buffers agree
// ===========================================================================

#[test]
fn c32_repeated_calls_no_hidden_state() {
    let p = load();
    let mut rng = Rng::new();

    for len in [1usize, 2, 3, 16, 17, 64] {
        for (name, g) in all_gen32() {
            for _ in 0..reps() / 4 {
                let init = g(&mut rng, len);
                let run = |im: &Impl| -> (u64, u64, Vec<u64>) {
                    let mut a = Buf::from_f32(&init);
                    let mut b = Buf::from_f32(&init);
                    let r1 = unsafe { (im.spectral_fn)(a.as_f32_ptr(), b.as_f32_ptr(), len as c_int) };
                    let r2 = unsafe { (im.spectral_fn)(a.as_f32_ptr(), b.as_f32_ptr(), len as c_int) };
                    let mut bits = a.all_bits();
                    bits.extend(b.all_bits());
                    (r1.to_bits(), r2.to_bits(), bits)
                };
                let (c1, c2, cb) = run(&p.c);
                let (r1, r2, rb) = run(&p.rs);
                assert_eq!(
                    (c1, c2, &cb),
                    (r1, r2, &rb),
                    "C32 spectral repeated len={len} {name}"
                );
            }
        }
    }

    for bins in [1i32, 2, 16, 17, 64] {
        for (name, g) in all_gen64() {
            for _ in 0..reps() / 4 {
                let t = g(&mut rng, bins as usize);
                let r = g(&mut rng, bins as usize);
                let th = rng.range(-1.0, 1.5);
                let run = |im: &Impl| -> (c_int, c_int) {
                    let mut bt = Buf::from_f64(&t);
                    let mut br = Buf::from_f64(&r);
                    let a = unsafe { (im.match_fn)(bt.as_f64_ptr(), br.as_f64_ptr(), bins, th) };
                    let b = unsafe { (im.match_fn)(bt.as_f64_ptr(), br.as_f64_ptr(), bins, th) };
                    (a, b)
                };
                assert_eq!(run(&p.c), run(&p.rs), "C32 match repeated bins={bins} {name}");
            }
        }
    }
}
