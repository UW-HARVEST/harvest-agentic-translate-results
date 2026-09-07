//! Phase B -- valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the whole memory region byte-for-byte (as raw `u32` bit
//! patterns, so `+0.0` vs `-0.0` and NaN payloads are distinguished).

mod common;

use common::*;

/// Sweep every size in axis C, `TRIALS` randomized inputs each.
fn sweep(ctx: &str, class: ValueClass, placement: Placement, shift: usize, min_size: i32) {
    let mut rng = Rng::new(SEED ^ (class as u64) << 8 ^ (shift as u64) << 32);
    for &size in SIZES {
        if size < min_size {
            continue;
        }
        for t in 0..TRIALS {
            let vals = gen_values(class, size as usize, &mut rng);
            check(
                &format!("{ctx} class={class:?} placement={placement:?} size={size} trial={t}"),
                placement,
                size,
                &vals,
                shift,
            );
        }
    }
}

// --------------------------------------------------------------------- C1
#[test]
fn c1_random_normal_all_sizes() {
    sweep("C1", ValueClass::UnitNormal, Placement::Separate, 0, 0);
}

// --------------------------------------------------------------------- C2
#[test]
fn c2_random_wide_exponent_all_sizes() {
    sweep("C2", ValueClass::WideExponent, Placement::Separate, 0, 0);
}

// --------------------------------------------------------------------- C3
#[test]
fn c3_exact_powers_of_two() {
    sweep("C3", ValueClass::PowerOfTwo, Placement::Separate, 0, 0);
}

// --------------------------------------------------------------------- C4
#[test]
fn c4_sum_overflows_to_inf() {
    sweep("C4", ValueClass::HugeOverflow, Placement::Separate, 0, 1);

    // Deterministic overflow probes: the sum is guaranteed +inf.
    let big = f32::MAX.to_bits();
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        let vals: Vec<u32> = (0..size as usize)
            .map(|i| if i % 2 == 0 { big } else { big | 0x8000_0000 })
            .collect();
        check("C4-flt_max", Placement::Separate, size, &vals, 0);
    }
}

// --------------------------------------------------------------------- C5
#[test]
fn c5_denormal_inputs() {
    sweep("C5", ValueClass::Denormal, Placement::Separate, 0, 1);

    // Deterministic: smallest subnormal everywhere -> products underflow to +0
    // -> sum == +0.0 -> memset branch.
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        let vals = vec![1u32; size as usize]; // 1e-45
        check("C5-min_subnormal", Placement::Separate, size, &vals, 0);

        // FLT_MIN everywhere -> squares are ~1e-76, flush to 0 in f32.
        let vals = vec![f32::MIN_POSITIVE.to_bits(); size as usize];
        check("C5-flt_min", Placement::Separate, size, &vals, 0);

        // Values whose square is representable but subnormal-ish.
        let vals = vec![(1e-20f32).to_bits(); size as usize];
        check("C5-1e-20", Placement::Separate, size, &vals, 0);
    }
}

// --------------------------------------------------------------------- C6
#[test]
fn c6_all_positive_zero_memset() {
    sweep("C6", ValueClass::PositiveZero, Placement::Separate, 0, 0);
}

// --------------------------------------------------------------------- C7
#[test]
fn c7_all_negative_zero_memset() {
    sweep("C7-neg", ValueClass::NegativeZero, Placement::Separate, 0, 0);
    sweep("C7-mixed", ValueClass::MixedZero, Placement::Separate, 0, 0);

    // Explicit: -0.0 input must come out as +0.0 (memset), not -0.0.
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        let vals = vec![(-0.0f32).to_bits(); size as usize];
        let l = build_layout(Placement::Separate, size, &vals, 0);
        let (c, r) = run_both(&l);
        assert_same("C7-explicit", &l, &c, &r);
        for i in 0..size as usize {
            assert_eq!(
                c[l.dest_word + i], 0x0000_0000,
                "C should memset -0.0 input to +0.0 at {i}"
            );
        }
    }
}

// --------------------------------------------------------------------- C8
#[test]
fn c8_nan_elements_memset() {
    sweep("C8", ValueClass::WithNan, Placement::Separate, 0, 1);

    // Deterministic: one NaN at every possible position, for several payloads.
    for &size in SIZES {
        if size < 1 || size > 129 {
            continue;
        }
        for &nan in &[0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFF80_0001, 0x7FFF_FFFF] {
            for pos in 0..size as usize {
                let mut vals: Vec<u32> = (0..size as usize)
                    .map(|i| (0.5f32 + i as f32).to_bits())
                    .collect();
                vals[pos] = nan;
                check(
                    &format!("C8-nan pos={pos} bits=0x{nan:08X}"),
                    Placement::Separate,
                    size,
                    &vals,
                    0,
                );
            }
        }
    }
}

// --------------------------------------------------------------------- C9
#[test]
fn c9_inf_elements_scale() {
    sweep("C9", ValueClass::WithInf, Placement::Separate, 0, 1);

    for &size in SIZES {
        if size < 1 || size > 129 {
            continue;
        }
        for &inf in &[0x7F80_0000u32, 0xFF80_0000] {
            for pos in 0..size as usize {
                let mut vals: Vec<u32> = (0..size as usize)
                    .map(|i| (-1.5f32 * (i as f32 + 1.0)).to_bits())
                    .collect();
                vals[pos] = inf;
                check(
                    &format!("C9-inf pos={pos} bits=0x{inf:08X}"),
                    Placement::Separate,
                    size,
                    &vals,
                    0,
                );
            }
        }
    }
}

// --------------------------------------------------------------------- C10
#[test]
fn c10_mixed_value_classes() {
    sweep("C10-mixed", ValueClass::Mixed, Placement::Separate, 0, 0);
    sweep("C10-randombits", ValueClass::RandomBits, Placement::Separate, 0, 0);
}

// --------------------------------------------------------------------- C11
#[test]
fn c11_in_place_scale() {
    sweep("C11-unit", ValueClass::UnitNormal, Placement::Aliased, 0, 0);
    sweep("C11-wide", ValueClass::WideExponent, Placement::Aliased, 0, 0);
    sweep("C11-pow2", ValueClass::PowerOfTwo, Placement::Aliased, 0, 0);
    sweep("C11-huge", ValueClass::HugeOverflow, Placement::Aliased, 0, 1);
}

// --------------------------------------------------------------------- C12
#[test]
fn c12_in_place_noop_branch() {
    // dest == src with a zero-norm / NaN src: BOTH branches are skipped, so
    // the buffer must be bit-identical to the input (including -0.0 signs and
    // NaN payloads) -- verified against C as well as asserted outright.
    for class in [
        ValueClass::PositiveZero,
        ValueClass::NegativeZero,
        ValueClass::MixedZero,
        ValueClass::WithNan,
    ] {
        sweep("C12", class, Placement::Aliased, 0, 0);
    }

    for &size in SIZES {
        if size < 1 {
            continue;
        }
        let mut vals = vec![(-0.0f32).to_bits(); size as usize];
        vals[size as usize / 2] = 0x8000_0000; // -0.0 again, explicit
        let l = build_layout(Placement::Aliased, size, &vals, 0);
        let (c, r) = run_both(&l);
        assert_same("C12-explicit-negzero", &l, &c, &r);
        assert_eq!(c, l.words, "C: aliased zero-norm must be a strict no-op");

        // NaN in place: no-op, payload survives.
        let mut vals: Vec<u32> = (0..size as usize).map(|i| (i as f32 + 1.0).to_bits()).collect();
        vals[0] = 0x7F80_00AB;
        let l = build_layout(Placement::Aliased, size, &vals, 0);
        let (c, r) = run_both(&l);
        assert_same("C12-explicit-nan", &l, &c, &r);
        assert_eq!(c, l.words, "C: aliased NaN must be a strict no-op");
    }
}

// --------------------------------------------------------------------- C13
#[test]
fn c13_forward_overlap_scale() {
    let mut rng = Rng::new(SEED ^ 0xC13);
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        for k in overlap_ks(size) {
            for class in [ValueClass::UnitNormal, ValueClass::WideExponent] {
                for t in 0..8 {
                    let vals = gen_values(class, size as usize, &mut rng);
                    check(
                        &format!("C13 size={size} k={k} class={class:?} t={t}"),
                        Placement::ForwardOverlap(k),
                        size,
                        &vals,
                        0,
                    );
                }
            }
        }
    }
}

// --------------------------------------------------------------------- C14
#[test]
fn c14_backward_overlap_scale() {
    let mut rng = Rng::new(SEED ^ 0xC14);
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        for k in overlap_ks(size) {
            for class in [ValueClass::UnitNormal, ValueClass::WideExponent] {
                for t in 0..8 {
                    let vals = gen_values(class, size as usize, &mut rng);
                    check(
                        &format!("C14 size={size} k={k} class={class:?} t={t}"),
                        Placement::BackwardOverlap(k),
                        size,
                        &vals,
                        0,
                    );
                }
            }
        }
    }
}

// --------------------------------------------------------------------- C15
#[test]
fn c15_overlap_memset_branch() {
    let mut rng = Rng::new(SEED ^ 0xC15);
    for &size in SIZES {
        if size < 1 {
            continue;
        }
        for k in overlap_ks(size) {
            for class in [ValueClass::MixedZero, ValueClass::WithNan, ValueClass::NegativeZero] {
                for t in 0..8 {
                    let vals = gen_values(class, size as usize, &mut rng);
                    check(
                        &format!("C15-fwd size={size} k={k} class={class:?} t={t}"),
                        Placement::ForwardOverlap(k),
                        size,
                        &vals,
                        0,
                    );
                    let vals = gen_values(class, size as usize, &mut rng);
                    check(
                        &format!("C15-bwd size={size} k={k} class={class:?} t={t}"),
                        Placement::BackwardOverlap(k),
                        size,
                        &vals,
                        0,
                    );
                }
            }
        }
    }
}

fn overlap_ks(size: i32) -> Vec<usize> {
    let n = size as usize;
    let mut v = vec![1usize];
    for k in [2, 3, 4, n / 2, n - 1, n] {
        if k >= 1 && !v.contains(&k) {
            v.push(k);
        }
    }
    v.retain(|&k| k >= 1 && k <= n.max(1));
    v
}

// --------------------------------------------------------------------- C16
#[test]
fn c16_size_one_sweep() {
    let mut rng = Rng::new(SEED ^ 0xC16);
    for class in ALL_CLASSES {
        for t in 0..512 {
            let vals = gen_values(*class, 1, &mut rng);
            check(
                &format!("C16 class={class:?} t={t}"),
                Placement::Separate,
                1,
                &vals,
                0,
            );
            check(
                &format!("C16-aliased class={class:?} t={t}"),
                Placement::Aliased,
                1,
                &vals,
                0,
            );
        }
    }

    // Exhaustive-ish over exponents: every finite non-zero single element must
    // normalize to exactly +-1.0 (or produce the C's exact quirk).
    for e in -149i32..=127 {
        for &sign in &[0u32, 0x8000_0000u32] {
            let v = if e < -126 {
                // subnormal
                let shift = e + 149;
                if shift < 0 || shift > 22 {
                    continue;
                }
                sign | (1u32 << shift)
            } else {
                sign | (((e + 127) as u32) << 23)
            };
            check(&format!("C16-exp e={e} bits=0x{v:08X}"), Placement::Separate, 1, &[v], 0);
        }
    }

    // And every "interesting" special bit pattern.
    for &v in &[
        0x0000_0000u32,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007F_FFFF,
        0x0080_0000,
        0x3F80_0000,
        0xBF80_0000,
        0x7F7F_FFFF,
        0xFF7F_FFFF,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
    ] {
        check(&format!("C16-special 0x{v:08X}"), Placement::Separate, 1, &[v], 0);
        check(&format!("C16-special-alias 0x{v:08X}"), Placement::Aliased, 1, &[v], 0);
    }
}

// --------------------------------------------------------------------- C17
#[test]
fn c17_large_size() {
    let mut rng = Rng::new(SEED ^ 0xC17);
    for &size in &[1000i32, 1024, 1031, 4096, 4097] {
        for class in [
            ValueClass::UnitNormal,
            ValueClass::WideExponent,
            ValueClass::Mixed,
            ValueClass::RandomBits,
        ] {
            for t in 0..16 {
                let vals = gen_values(class, size as usize, &mut rng);
                check(
                    &format!("C17 size={size} class={class:?} t={t}"),
                    Placement::Separate,
                    size,
                    &vals,
                    0,
                );
                let vals = gen_values(class, size as usize, &mut rng);
                check(
                    &format!("C17-alias size={size} class={class:?} t={t}"),
                    Placement::Aliased,
                    size,
                    &vals,
                    0,
                );
            }
        }
    }
}

// --------------------------------------------------------------------- C18
#[test]
fn c18_no_out_of_window_writes() {
    // `run_both`/`assert_same` already compare the guard words, but assert the
    // stronger invariant explicitly: C never touches a byte outside dest[0..size).
    let mut rng = Rng::new(SEED ^ 0xC18);
    for &size in SIZES {
        for class in ALL_CLASSES {
            let vals = gen_values(*class, size as usize, &mut rng);
            let l = build_layout(Placement::Separate, size, &vals, 0);
            let (c, r) = run_both(&l);
            assert_same("C18", &l, &c, &r);

            let d0 = l.dest_word;
            let d1 = d0 + size.max(0) as usize;
            for i in 0..l.words.len() {
                if i >= d0 && i < d1 {
                    continue;
                }
                assert_eq!(
                    c[i], l.words[i],
                    "C wrote outside the dest window at word {i} (size={size}, class={class:?})"
                );
                assert_eq!(
                    r[i], l.words[i],
                    "RUST wrote outside the dest window at word {i} (size={size}, class={class:?})"
                );
            }
        }
    }
}

// --------------------------------------------------------------------- C19
#[test]
fn c19_misaligned_buffers() {
    for shift in 1..=3usize {
        sweep("C19-unit", ValueClass::UnitNormal, Placement::Separate, shift, 0);
        sweep("C19-mixed", ValueClass::Mixed, Placement::Separate, shift, 0);
        sweep("C19-zero", ValueClass::MixedZero, Placement::Separate, shift, 0);
        sweep("C19-alias", ValueClass::WideExponent, Placement::Aliased, shift, 0);
    }
}

// --------------------------------------------------------------------- C20
#[test]
fn c20_size_zero() {
    for placement in [
        Placement::Separate,
        Placement::Aliased,
        Placement::ForwardOverlap(1),
        Placement::BackwardOverlap(1),
    ] {
        let l = build_layout(placement, 0, &[], 0);
        let (c, r) = run_both(&l);
        assert_same("C20", &l, &c, &r);
        assert_eq!(c, l.words, "C: size==0 must be a strict no-op ({placement:?})");
        assert_eq!(r, l.words, "RUST: size==0 must be a strict no-op ({placement:?})");
    }
}
