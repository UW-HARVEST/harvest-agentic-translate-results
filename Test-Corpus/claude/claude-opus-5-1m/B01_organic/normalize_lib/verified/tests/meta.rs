//! Meta-tests: they verify that the differential harness itself is meaningful.
//!
//! 1. negative control — an intentionally wrong implementation MUST be rejected
//!    by the same comparison the Phase B/C tests use (otherwise a green suite
//!    would prove nothing);
//! 2. branch coverage — the generators of `CONFIGS.md` must actually reach all
//!    three code paths of the C function (`sum > 0`, `memset`, and the
//!    `dest == src` no-op), with a healthy count for each.

mod common;

use common::*;

/// Exactly what the C does, so it can be perturbed on purpose below.
fn c_model(dest: &mut [f32], src: &[f32], size: usize, in_place: bool, bug: bool) {
    let mut sum = 0.0f32;
    for i in 0..size {
        sum += src[i] * src[i];
    }
    if sum > 0.0f32 {
        let k = 1.0f32 / sum.sqrt();
        for i in 0..size {
            dest[i] = src[i] * k;
        }
        if bug {
            // the classic "optimisation" a translator might introduce:
            // divide instead of multiplying by the reciprocal
            for i in 0..size {
                dest[i] = src[i] / sum.sqrt();
            }
        }
    } else if !in_place {
        for i in 0..size {
            dest[i] = 0.0;
        }
        if bug {
            // and the classic "clean-up": propagate NaN instead of zeroing
            for i in 0..size {
                dest[i] = src[i];
            }
        }
    }
}

#[test]
fn meta_comparison_rejects_a_wrong_implementation() {
    let (c, _r) = load_impls();
    let mut rng = Rng::new(0xDEAD_BEEF);
    let mut caught_normalise = 0usize;
    let mut caught_zerofill = 0usize;
    for _ in 0..2000 {
        let size = 1 + rng.below(16);
        let class = ALL_CLASSES[rng.below(ALL_CLASSES.len())];
        let mut src = vec![0.0f32; size];
        gen_values(class, &mut rng, &mut src);
        let mut base = src.clone();
        base.extend(std::iter::repeat_n(1.5f32, size + 4));
        let dest_off = size + 2;

        let cout = run_one(&c, &base, dest_off, 0, size as std::ffi::c_int);

        let mut buggy = base.clone();
        let (head, tail) = buggy.split_at_mut(dest_off);
        let srcv: Vec<f32> = head[..size].to_vec();
        c_model(tail, &srcv, size, false, true);

        let sum = srcv.iter().fold(0.0f32, |a, v| a + v * v);
        if cmp_bits("negative control", &cout, &buggy).is_err() {
            if sum > 0.0 {
                caught_normalise += 1;
            } else {
                caught_zerofill += 1;
            }
        }
    }
    assert!(
        caught_normalise > 100,
        "the comparison did not catch the perturbed normalisation ({caught_normalise} catches)"
    );
    assert!(
        caught_zerofill > 10,
        "the comparison did not catch the perturbed zero-fill ({caught_zerofill} catches)"
    );
    println!(
        "negative control: {caught_normalise} normalise-bug + {caught_zerofill} zerofill-bug \
         perturbations detected"
    );
}

#[test]
fn meta_generators_reach_every_c_branch() {
    let mut rng = Rng::new(0xC0FF_EE00);
    let mut normalise = 0usize;
    let mut memset = 0usize;
    let mut noop = 0usize;
    let mut sum_inf = 0usize;
    let mut sum_nan = 0usize;
    let mut sum_zero_nonzero_src = 0usize;

    for &class in ALL_CLASSES.iter() {
        for &size_i in SIZES.iter() {
            let size = size_i as usize;
            for trial in 0..8 {
                let mut v = vec![0.0f32; size];
                gen_values(class, &mut rng, &mut v);
                let sum = v.iter().fold(0.0f32, |a, x| a + x * x);
                let in_place = trial % 2 == 0;
                if sum > 0.0 {
                    normalise += 1;
                    if sum.is_infinite() {
                        sum_inf += 1;
                    }
                } else if !in_place {
                    memset += 1;
                } else {
                    noop += 1;
                }
                if sum.is_nan() {
                    sum_nan += 1;
                }
                if sum == 0.0 && v.iter().any(|x| *x != 0.0) {
                    sum_zero_nonzero_src += 1;
                }
            }
        }
    }
    println!(
        "branch coverage: sum>0 -> {normalise}, memset -> {memset}, dest==src no-op -> {noop}, \
         sum==inf -> {sum_inf}, sum==NaN -> {sum_nan}, underflowed sum -> {sum_zero_nonzero_src}"
    );
    assert!(normalise > 500, "the `sum > 0` branch is barely covered");
    assert!(memset > 100, "the `memset` branch is barely covered");
    assert!(noop > 100, "the `dest == src` no-op branch is barely covered");
    assert!(sum_inf > 20, "`sum == +inf` is barely covered");
    assert!(sum_nan > 20, "`sum == NaN` is barely covered");
    assert!(
        sum_zero_nonzero_src > 20,
        "underflow-to-zero sums are barely covered"
    );
}

#[test]
fn meta_calls_really_go_through_the_shared_objects() {
    // A trivially checkable input: both `.so`s must actually modify the buffer.
    let (c, r) = load_impls();
    let base = [3.0f32, 4.0, 0.0, 0.0];
    let cout = run_one(&c, &base, 2, 0, 2);
    let rout = run_one(&r, &base, 2, 0, 2);
    assert_eq!(cout[2].to_bits(), 0.6f32.to_bits(), "C did not write dest");
    assert_eq!(cout[3].to_bits(), 0.8f32.to_bits(), "C did not write dest");
    assert_eq!(bits_of(&cout), bits_of(&rout));
    println!(
        "loaded C={:?} Rust={:?}",
        c_so_path().file_name().unwrap(),
        rust_so_path().file_name().unwrap()
    );
}
