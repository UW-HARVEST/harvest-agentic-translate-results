//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads both `.so`s and calls `synth_pair` through the exported C
//! symbol on each side. Nothing is called directly in-process.

mod common;

use common::*;
use std::ffi::c_int;

/// pcm scratch buffer: big enough for `nch` up to 64 in both directions.
const PCM_LEN: usize = 4096;
const PCM_MID: usize = 2048;

fn pcm_sentinel(rng: &mut Rng) -> Vec<i16> {
    (0..PCM_LEN).map(|_| rng.next_u32() as i16).collect()
}

// ---------------------------------------------------------------------------
// C1 / C2 — the degenerate "empty" shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_all_positive_zero() {
    let h = Harness::load();
    let z = z_zeros();
    let prefill = vec![0x5A5Au16 as i16; PCM_LEN];
    let out = h.assert_same("C1 all +0.0 nch=2", &z, 2, &prefill, PCM_MID);
    // Cross-check against the documented C semantics: a == +0.0 -> s == 0.
    assert_eq!(out[PCM_MID], 0, "C1 pcm[0]");
    assert_eq!(out[PCM_MID + 32], 0, "C1 pcm[16*nch]");
}

#[test]
fn cfg_c2_all_negative_zero() {
    let h = Harness::load();
    let z = vec![-0.0f32; Z_LEN];
    let prefill = vec![0x5A5Au16 as i16; PCM_LEN];
    h.assert_same("C2 all -0.0 nch=2", &z, 2, &prefill, PCM_MID);
}

#[test]
fn cfg_c2b_negative_zero_accumulator() {
    // Hand-built sign pattern that keeps *every* block-1 term at -0.0, the one
    // accumulator value `solve_a1` cannot reach (see common/mod.rs).
    let h = Harness::load();
    let mut z = z_zeros();
    set_tap1(&mut z, 14, -0.0); // (z14 - z0) * 29     = -0.0
    set_tap1(&mut z, 0, 0.0);
    set_tap1(&mut z, 1, -0.0); // (z1 + z13) * 213    = -0.0
    set_tap1(&mut z, 13, -0.0);
    set_tap1(&mut z, 12, -0.0); // (z12 - z2) * 459    = -0.0
    set_tap1(&mut z, 2, 0.0);
    set_tap1(&mut z, 3, -0.0); // (z3 + z11) * 2037   = -0.0
    set_tap1(&mut z, 11, -0.0);
    set_tap1(&mut z, 10, -0.0); // (z10 - z4) * 5153   = -0.0
    set_tap1(&mut z, 4, 0.0);
    set_tap1(&mut z, 5, -0.0); // (z5 + z9) * 6574    = -0.0
    set_tap1(&mut z, 9, -0.0);
    set_tap1(&mut z, 8, -0.0); // (z8 - z6) * 37489   = -0.0
    set_tap1(&mut z, 6, 0.0);
    set_tap1(&mut z, 7, -0.0); // z7 * 75038          = -0.0
    let prefill = vec![0i16; PCM_LEN];
    h.assert_same("C2b -0.0 accumulator", &z, 2, &prefill, PCM_MID);
}

// ---------------------------------------------------------------------------
// C3 / C4 — isolate every tap slot and every coefficient
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_single_block1_tap_sweep() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC3);
    let prefill = vec![0i16; PCM_LEN];
    for tap in 0..N_TAPS {
        for iter in 0..500 {
            let v = match iter % 5 {
                0 => rng.scaled(1e-3),
                1 => rng.scaled(1.0),
                2 => rng.scaled(1e3),
                3 => rng.mixed(),
                _ => rng.any_f32(),
            };
            let mut z = z_zeros();
            set_tap1(&mut z, tap, v);
            h.assert_same(
                &format!("C3 block1 tap {tap} v={:08x}", v.to_bits()),
                &z,
                2,
                &prefill,
                PCM_MID,
            );
        }
    }
}

#[test]
fn cfg_c4_single_block2_tap_sweep() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC4);
    let prefill = vec![0i16; PCM_LEN];
    // All 15 slots, including the 7 odd ones block 2 never reads.
    for tap in 0..N_TAPS {
        for iter in 0..500 {
            let v = match iter % 5 {
                0 => rng.scaled(1e-3),
                1 => rng.scaled(1.0),
                2 => rng.scaled(1e3),
                3 => rng.mixed(),
                _ => rng.any_f32(),
            };
            let mut z = z_zeros();
            set_tap2(&mut z, tap, v);
            h.assert_same(
                &format!("C4 block2 slot {tap} v={:08x}", v.to_bits()),
                &z,
                2,
                &prefill,
                PCM_MID,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C5 / C6 / C7 / C8 / C10 / C11 — magnitude / value-class classes
// ---------------------------------------------------------------------------

fn fill_all_taps(z: &mut [f32], rng: &mut Rng, mk: &mut dyn FnMut(&mut Rng) -> f32) {
    for i in 0..N_TAPS {
        let a = mk(rng);
        let b = mk(rng);
        set_tap1(z, i, a);
        set_tap2(z, i, b);
    }
}

fn magnitude_class_case(
    label: &str,
    seed_salt: u64,
    iters: usize,
    mut mk: impl FnMut(&mut Rng) -> f32,
) {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ seed_salt);
    let prefill = vec![0i16; PCM_LEN];
    for i in 0..iters {
        let mut z = z_zeros();
        fill_all_taps(&mut z, &mut rng, &mut mk);
        h.assert_same(&format!("{label} iter {i}"), &z, 2, &prefill, PCM_MID);
    }
}

#[test]
fn cfg_c5_small_magnitudes_no_clip() {
    magnitude_class_case("C5 small", 0xC5, 4000, |r| r.scaled(1e-2));
}

#[test]
fn cfg_c6_mid_magnitudes_straddle_clip() {
    magnitude_class_case("C6 mid", 0xC6, 4000, |r| r.scaled(0.5));
}

#[test]
fn cfg_c7_large_magnitudes_saturate() {
    magnitude_class_case("C7 large", 0xC7, 4000, |r| r.scaled(1e6));
}

#[test]
fn cfg_c8_huge_products_overflow_to_inf() {
    magnitude_class_case("C8 huge", 0xC8, 4000, |r| r.scaled(1e35));
}

#[test]
fn cfg_c10_subnormal_taps() {
    magnitude_class_case("C10 subnormal", 0xC10, 4000, |r| r.subnormal());
}

#[test]
fn cfg_c11_full_bit_pattern_space() {
    magnitude_class_case("C11 any bits", 0xC11, 20000, |r| r.any_f32());
}

// ---------------------------------------------------------------------------
// C9 — accumulators within 1 ULP of the clip thresholds
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_clip_threshold_neighbourhood() {
    let h = Harness::load();
    let prefill = vec![0i16; PCM_LEN];
    let mut checked = 0usize;
    for base in [32766.5f32, -32767.5f32, 32767.0, -32768.0, 0.5, -0.5] {
        // Walk ±6 ULPs around each interesting accumulator value.
        let mut v = base;
        for _ in 0..6 {
            v = f32::from_bits(v.to_bits().wrapping_sub(1));
        }
        for _ in 0..13 {
            if let Some(z) = z_for_accumulators(v, v) {
                h.assert_same(
                    &format!("C9 accumulator {v} ({:08x})", v.to_bits()),
                    &z,
                    2,
                    &prefill,
                    PCM_MID,
                );
                checked += 1;
            }
            v = f32::from_bits(v.to_bits().wrapping_add(1));
        }
    }
    assert!(checked >= 40, "C9 only exercised {checked} accumulator values");
}

// ---------------------------------------------------------------------------
// C12 — stride must be honoured; filler bytes are irrelevant
// ---------------------------------------------------------------------------

#[test]
fn cfg_c12_poisoned_filler_between_taps() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC12);
    let prefill = vec![0i16; PCM_LEN];
    for i in 0..3000 {
        let mut z = z_zeros();
        poison_filler(&mut z, &mut rng);
        fill_all_taps(&mut z, &mut rng, &mut |r: &mut Rng| r.scaled(0.5));
        h.assert_same(&format!("C12 poisoned filler iter {i}"), &z, 2, &prefill, PCM_MID);
    }
}

// ---------------------------------------------------------------------------
// C13 / C14 / C15 — nch strides
// ---------------------------------------------------------------------------

fn nch_case(label: &str, salt: u64, iters: usize, mut pick_nch: impl FnMut(&mut Rng) -> c_int) {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ salt);
    for i in 0..iters {
        let nch = pick_nch(&mut rng);
        let mut z = z_zeros();
        fill_all_taps(&mut z, &mut rng, &mut |r: &mut Rng| r.mixed());
        let prefill = pcm_sentinel(&mut rng);
        h.assert_same(&format!("{label} iter {i} nch={nch}"), &z, nch, &prefill, PCM_MID);
    }
}

#[test]
fn cfg_c13_mono_stride() {
    nch_case("C13 nch=1", 0xC13, 3000, |_| 1);
}

#[test]
fn cfg_c14_stereo_stride() {
    nch_case("C14 nch=2", 0xC14, 3000, |_| 2);
}

#[test]
fn cfg_c15_arbitrary_positive_stride() {
    nch_case("C15 nch=3..64", 0xC15, 3000, |r| 3 + r.below(62) as c_int);
}

#[test]
fn cfg_c16_nch_zero_collides() {
    nch_case("C16 nch=0", 0xC16, 2000, |_| 0);
}

#[test]
fn cfg_c17_negative_stride() {
    nch_case("C17 nch<0", 0xC17, 3000, |r| -(1 + r.below(64) as c_int));
}

#[test]
fn cfg_c18_nch_int_overflow_wraps() {
    // `16 * nch` overflows `int`. Because 16 * 2^28 == 2^32 == 0 (mod 2^32),
    // nch = k + m*2^28 has the same wrapped offset as nch = k. Pick k small so
    // the (wrapped) store stays inside the buffer.
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC18);
    let mut cases: Vec<c_int> = vec![i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for k in -64i32..=64 {
        for m in [1i32, -1, 7, -8, 3, -5] {
            cases.push(k.wrapping_add(m.wrapping_mul(1 << 28)));
        }
    }
    for &nch in &cases {
        let off = 16i32.wrapping_mul(nch) as isize;
        assert!(
            (PCM_MID as isize + off) >= 0 && (PCM_MID as isize + off) < PCM_LEN as isize,
            "nch={nch} wrapped offset {off} escapes the scratch buffer"
        );
        let mut z = z_zeros();
        fill_all_taps(&mut z, &mut rng, &mut |r: &mut Rng| r.mixed());
        let prefill = pcm_sentinel(&mut rng);
        h.assert_same(&format!("C18 nch={nch} (offset {off})"), &z, nch, &prefill, PCM_MID);
    }
}

// ---------------------------------------------------------------------------
// C19 — exactly two elements written, nothing else touched
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_no_neighbour_clobber() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC19);
    for nch in [0i32, 1, 2, 5, 17, -1, -3] {
        for _ in 0..200 {
            let mut z = z_zeros();
            fill_all_taps(&mut z, &mut rng, &mut |r: &mut Rng| r.scaled(0.5));
            let prefill = pcm_sentinel(&mut rng);
            let out = h.assert_same(
                &format!("C19 nch={nch} clobber check"),
                &z,
                nch,
                &prefill,
                PCM_MID,
            );
            let touched: Vec<usize> = (0..PCM_LEN).filter(|&i| out[i] != prefill[i]).collect();
            let allowed = [PCM_MID, (PCM_MID as isize + 16 * nch as isize) as usize];
            for i in touched {
                assert!(
                    allowed.contains(&i),
                    "C19 nch={nch}: element {i} modified but only {allowed:?} may change"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — pcm aliasing the z allocation
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_pcm_aliases_z() {
    // A single allocation is read as `const float*` and written as
    // `mp3d_sample_t*`. Both stores are aimed at slots that block 2 reads, so
    // the read/write ordering inside the function is observable.
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC20);

    // Byte offset of a block-2 tap: z[2 + i*64] -> (2 + i*64) * 4 bytes.
    // As i16 indices that is (2 + i*64) * 2.
    for tap in [4usize, 8, 12] {
        let pcm_i16_index = (2 + tap * 64) * 2;
        for _ in 0..300 {
            let mut base = z_zeros();
            fill_all_taps(&mut base, &mut rng, &mut |r: &mut Rng| r.scaled(0.5));

            let mut c_buf = base.clone();
            let mut r_buf = base.clone();
            let nch: c_int = 2;
            unsafe {
                let c_pcm = (c_buf.as_mut_ptr() as *mut i16).add(pcm_i16_index);
                let r_pcm = (r_buf.as_mut_ptr() as *mut i16).add(pcm_i16_index);
                // pcm[0] and pcm[32] must stay inside the f32 allocation:
                // (pcm_i16_index + 32) * 2 bytes < Z_LEN * 4 bytes.
                assert!((pcm_i16_index + 32) * 2 < Z_LEN * 4);
                let c_sym = &h;
                let _ = c_sym;
                h.call_raw_c(c_pcm, nch, c_buf.as_ptr());
                h.call_raw_rust(r_pcm, nch, r_buf.as_ptr());
            }
            let c_bits: Vec<u32> = c_buf.iter().map(|v| v.to_bits()).collect();
            let r_bits: Vec<u32> = r_buf.iter().map(|v| v.to_bits()).collect();
            assert_eq!(
                c_bits, r_bits,
                "C20 aliasing divergence at block-2 tap {tap}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C21 — the two accumulators are independent
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_blocks_are_independent() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC21);
    let prefill = vec![0i16; PCM_LEN];
    for i in 0..2000 {
        // Only block-1 slots populated.
        let mut z1 = z_zeros();
        for t in 0..N_TAPS {
            set_tap1(&mut z1, t, rng.scaled(0.5));
        }
        h.assert_same(&format!("C21 block1-only iter {i}"), &z1, 2, &prefill, PCM_MID);

        // Only block-2 slots populated.
        let mut z2 = z_zeros();
        for t in 0..N_TAPS {
            set_tap2(&mut z2, t, rng.scaled(0.5));
        }
        h.assert_same(&format!("C21 block2-only iter {i}"), &z2, 2, &prefill, PCM_MID);
    }
}

// ---------------------------------------------------------------------------
// C22 — drive every mp3d_scale_pcm branch with an exact accumulator value
// ---------------------------------------------------------------------------

#[test]
fn cfg_c22_scale_pcm_branch_sweep() {
    let h = Harness::load();
    let prefill = vec![0i16; PCM_LEN];
    let mut targets: Vec<f32> = vec![
        32766.5,
        -32767.5,
        32766.499,
        -32767.499,
        32767.0,
        -32768.0,
        1e9,
        -1e9,
        0.0,
        0.5,
        -0.5,
        0.4999,
        -0.4999,
        1.5,
        -1.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    // Half-integer and integer accumulators across the whole i16 range: these
    // are the values where `(int16_t)(a + .5f)` and `s -= (s < 0)` interact.
    let mut v = -32768.0f32;
    while v <= 32768.0 {
        targets.push(v);
        targets.push(v + 0.5);
        targets.push(v - 0.5);
        targets.push(v + 0.25);
        targets.push(v - 0.25);
        v += 197.0;
    }
    let mut solved = 0usize;
    for &t in &targets {
        if let Some(z) = z_for_accumulators(t, t) {
            h.assert_same(
                &format!("C22 accumulator target {t} ({:08x})", t.to_bits()),
                &z,
                2,
                &prefill,
                PCM_MID,
            );
            solved += 1;
        }
    }
    assert!(
        solved * 10 >= targets.len() * 8,
        "C22 solved only {solved}/{} accumulator targets",
        targets.len()
    );
}

// ---------------------------------------------------------------------------
// C23 — broad randomized fuzz over the whole input space
// ---------------------------------------------------------------------------

#[test]
fn cfg_c23_full_fuzz() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xC23);
    let iters: usize = std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60_000);
    for i in 0..iters {
        let mut z = vec![0.0f32; Z_LEN];
        for slot in z.iter_mut() {
            *slot = rng.mixed();
        }
        let nch = match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => rng.below(64) as c_int,
            4 => -(rng.below(64) as c_int),
            _ => (rng.below(129) as i32 - 64).wrapping_add(((rng.below(15) as i32) - 7) << 28),
        };
        let off = 16i32.wrapping_mul(nch) as isize;
        if PCM_MID as isize + off < 0 || PCM_MID as isize + off >= PCM_LEN as isize {
            continue;
        }
        let prefill = pcm_sentinel(&mut rng);
        h.assert_same(&format!("C23 fuzz iter {i} nch={nch}"), &z, nch, &prefill, PCM_MID);
    }
}

// ---------------------------------------------------------------------------
// Harness self-validation: the single-tap accumulator solver really does put
// the intended value into the accumulator.
// ---------------------------------------------------------------------------

/// `z_for_accumulators(t, t)` claims the C's two accumulators both end up
/// exactly `t`. Verify that claim against the **C** library: its outputs must be
/// `mp3d_scale_pcm(t)`. If the solver were wrong, every `assert_sentinel` in
/// `tests/errors.rs` would be testing the wrong accumulator value, so this
/// guard is what makes Phase C meaningful.
#[test]
fn harness_solver_hits_the_intended_accumulator() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0x5011);
    let prefill = vec![0i16; PCM_LEN];
    let mut solved = 0usize;
    let mut attempted = 0usize;

    let mut targets: Vec<f32> = Vec::new();
    // Dense sweep of the whole non-clipped range plus both clip regions.
    let mut v = -40000.0f32;
    while v < 40000.0 {
        targets.push(v);
        v += 7.03125; // exactly representable, hits many fractional parts
    }
    for _ in 0..4000 {
        targets.push(rng.scaled(40000.0));
    }
    for &t in &[
        32766.5f32,
        -32767.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        0.0,
    ] {
        targets.push(t);
    }

    for &t in &targets {
        attempted += 1;
        let Some(z) = z_for_accumulators(t, t) else {
            continue;
        };
        solved += 1;
        let (c_pcm, r_pcm) = h.call_both(&z, 2, &prefill, PCM_MID);
        assert_eq!(c_pcm, r_pcm, "solver check: C/Rust diverged at target {t}");
        let want = expected_scale_pcm(t);
        assert_eq!(
            c_pcm[PCM_MID], want,
            "solver check: block-1 accumulator for target {t} ({:08x}) was not realised \
             (C produced {}, mp3d_scale_pcm({t}) == {want})",
            t.to_bits(),
            c_pcm[PCM_MID]
        );
        assert_eq!(
            c_pcm[PCM_MID + 32], want,
            "solver check: block-2 accumulator for target {t} was not realised"
        );
    }
    // The solver must cover essentially the whole target space, otherwise the
    // named boundary rows in ERRORS.md could be silently skipped.
    let pct = solved * 100 / attempted;
    println!("solver coverage: {solved}/{attempted} targets ({pct}%)");
    assert!(
        pct >= 99,
        "single-tap solver only reached {pct}% of accumulator targets"
    );
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so`, checked
/// from inside the test suite as well as by `run_all.sh` (Phase D).
#[test]
fn harness_symbol_parity() {
    use std::process::Command;
    let c = c_so_path();
    let r = rust_so_path();
    let syms = |p: &std::path::Path| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(p)
            .output()
            .expect("nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let c_syms = syms(&c);
    let r_syms = syms(&r);
    assert!(
        c_syms.contains(&"synth_pair".to_string()),
        "sanity: C .so should export synth_pair, got {c_syms:?}"
    );
    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    println!(
        "symbol parity: {} C symbols, all present in the Rust .so",
        c_syms.len()
    );
}
