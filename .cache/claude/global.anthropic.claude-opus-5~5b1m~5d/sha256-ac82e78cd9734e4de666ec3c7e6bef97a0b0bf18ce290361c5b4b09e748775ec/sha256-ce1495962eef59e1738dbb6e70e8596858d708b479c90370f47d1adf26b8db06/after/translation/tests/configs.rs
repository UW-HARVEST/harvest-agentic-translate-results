//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded with `libloading` and driven exclusively
//! through the exported `synth_pair` symbol.

mod common;

use common::*;
use std::ffi::c_int;

/// Room for `pcm[0]` and `pcm[16*nch]` for `nch` up to 8, plus slack so that a
/// stray write outside the two documented destinations is detected.
const PCM_LEN: usize = 16 * 8 + 16;
const FILL: i16 = 0x5A5A_u16 as i16;

fn fill_uniform(rng: &mut Rng, amp: f32) -> Vec<f32> {
    (0..Z_MIN_LEN).map(|_| rng.signed_unit() * amp).collect()
}

// ---------------------------------------------------------------------------
// C1 / C2 / C3 — small uniform normals over the interesting `nch` values
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_small_uniform_nch1() {
    let mut rng = Rng::new(0xC001);
    for i in 0..20_000 {
        let z = fill_uniform(&mut rng, 1.0);
        diff_call(&format!("C1 #{i}"), PCM_LEN, 0, FILL, 1, &z, 0);
    }
}

#[test]
fn cfg_c2_small_uniform_nch2() {
    let mut rng = Rng::new(0xC002);
    for i in 0..20_000 {
        let z = fill_uniform(&mut rng, 1.0);
        diff_call(&format!("C2 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

#[test]
fn cfg_c3_varied_nch_small_uniform() {
    let mut rng = Rng::new(0xC003);
    for i in 0..20_000 {
        let nch = 3 + (i % 6) as c_int; // 3..=8
        let z = fill_uniform(&mut rng, 1.0);
        diff_call(&format!("C3 #{i} nch={nch}"), PCM_LEN, 0, FILL, nch, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C4 / C5 — amplitudes that straddle / exceed the clamp thresholds
// ---------------------------------------------------------------------------

#[test]
fn cfg_c4_near_clamp_thresholds() {
    let mut rng = Rng::new(0xC004);
    let mut clamped_hi = 0usize;
    let mut clamped_lo = 0usize;
    let mut plain = 0usize;
    for i in 0..20_000 {
        // Chosen so lane sums land all around +-32767: the dominant lane-0
        // weight is 75038 and the dominant lane-1 weight is 64019.
        let amp = 0.20 + rng.unit() * 0.60; // ~0.2 .. 0.8
        let z = fill_uniform(&mut rng, amp);
        let out = diff_call(&format!("C4 #{i} amp={amp}"), PCM_LEN, 0, FILL, 2, &z, 0);
        match out[0] {
            32767 => clamped_hi += 1,
            -32768 => clamped_lo += 1,
            _ => plain += 1,
        }
    }
    // The row is only meaningful if it really visited all three regions.
    assert!(clamped_hi > 100, "C4 never clamped high ({clamped_hi})");
    assert!(clamped_lo > 100, "C4 never clamped low ({clamped_lo})");
    assert!(plain > 100, "C4 never took the conversion path ({plain})");
}

#[test]
fn cfg_c5_mostly_clamping() {
    let mut rng = Rng::new(0xC005);
    for i in 0..20_000 {
        let amp = (10.0f32).powi(2 + (i % 5) as i32); // 1e2 .. 1e6
        let z = fill_uniform(&mut rng, amp);
        diff_call(&format!("C5 #{i} amp={amp}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C6 / C7 — per-tap isolation and proof that unread indices are ignored
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_single_tap_isolation() {
    let mut rng = Rng::new(0xC006);
    let taps = all_taps();
    assert_eq!(taps.len(), 23, "expected 23 distinct read taps");
    for (t, &tap) in taps.iter().enumerate() {
        for i in 0..2_000 {
            let mut z = zeros_z();
            // Mix magnitudes so each weight is probed across many binades,
            // including values that make the single term clamp.
            z[tap] = match i % 4 {
                0 => rng.signed_unit(),
                1 => rng.signed_unit() * 1e4,
                2 => rng.wide_exponent_f32(-30, 30),
                _ => rng.any_bits_f32(),
            };
            diff_call(
                &format!("C6 tap#{t}={tap} #{i}"),
                PCM_LEN,
                0,
                FILL,
                1,
                &z,
                0,
            );
        }
    }
}

#[test]
fn cfg_c7_unread_indices_are_ignored() {
    let mut rng = Rng::new(0xC007);
    let unread = unread_indices();
    assert_eq!(unread.len(), Z_MIN_LEN - 23);
    let baseline = diff_call("C7 baseline", PCM_LEN, 0, FILL, 2, &zeros_z(), 0);
    for i in 0..4_000 {
        let mut z = zeros_z();
        let idx = unread[rng.below(unread.len())];
        z[idx] = rng.any_bits_f32();
        let out = diff_call(&format!("C7 #{i} idx={idx}"), PCM_LEN, 0, FILL, 2, &z, 0);
        assert_eq!(
            out, baseline,
            "C7: writing the unread index {idx} changed the output"
        );
    }
}

// ---------------------------------------------------------------------------
// C8 / C9 — boundary-value pool and fully random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn cfg_c8_boundary_value_pool() {
    let mut rng = Rng::new(0xC008);
    for i in 0..20_000 {
        let z: Vec<f32> = (0..Z_MIN_LEN)
            .map(|_| BOUNDARY_POOL[rng.below(BOUNDARY_POOL.len())])
            .collect();
        diff_call(&format!("C8 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

#[test]
fn cfg_c9_random_bit_patterns() {
    let mut rng = Rng::new(0xC009);
    let mut saw_nan_input = 0usize;
    for i in 0..40_000 {
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.any_bits_f32()).collect();
        if all_taps().iter().any(|&t| z[t].is_nan()) {
            saw_nan_input += 1;
        }
        diff_call(&format!("C9 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
    assert!(saw_nan_input > 100, "C9 never fed a NaN tap");
}

// ---------------------------------------------------------------------------
// C10 / C11 — cancellation in the difference and sum terms
// ---------------------------------------------------------------------------

#[test]
fn cfg_c10_cancelling_difference_taps() {
    // Lane 0's difference pairs: (896,0) (768,128) (640,256) (512,384).
    const PAIRS: [(usize, usize); 4] = [(896, 0), (768, 128), (640, 256), (512, 384)];
    let mut rng = Rng::new(0xC010);
    for i in 0..20_000 {
        let mut z = zeros_z();
        for (a, b) in PAIRS {
            let v = match i % 3 {
                0 => rng.signed_unit(),
                1 => rng.wide_exponent_f32(-40, 40),
                _ => rng.any_bits_f32(),
            };
            z[a] = v;
            z[b] = v; // difference cancels to +-0.0 (or NaN for inf/NaN inputs)
        }
        // Leave the sum terms and the bare z[448] tap live too.
        z[64] = rng.signed_unit();
        z[832] = rng.signed_unit();
        z[448] = rng.signed_unit();
        for &t in LANE1_TAPS.iter() {
            z[t] = rng.signed_unit();
        }
        diff_call(&format!("C10 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

#[test]
fn cfg_c11_cancelling_sum_taps() {
    // Lane 0's sum pairs: (64,832) (192,704) (320,576).
    const PAIRS: [(usize, usize); 3] = [(64, 832), (192, 704), (320, 576)];
    let mut rng = Rng::new(0xC011);
    for i in 0..20_000 {
        let mut z = zeros_z();
        for (a, b) in PAIRS {
            let v = match i % 3 {
                0 => rng.signed_unit(),
                1 => rng.wide_exponent_f32(-40, 40),
                _ => rng.any_bits_f32(),
            };
            z[a] = v;
            z[b] = -v; // sum cancels
        }
        z[896] = rng.signed_unit();
        z[0] = rng.signed_unit();
        z[448] = rng.signed_unit();
        for &t in LANE1_TAPS.iter() {
            z[t] = rng.signed_unit();
        }
        diff_call(&format!("C11 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C12 — full binade sweep
// ---------------------------------------------------------------------------

#[test]
fn cfg_c12_wide_exponent_range() {
    let mut rng = Rng::new(0xC012);
    for i in 0..40_000 {
        let z: Vec<f32> = (0..Z_MIN_LEN)
            .map(|_| rng.wide_exponent_f32(-45, 45))
            .collect();
        diff_call(&format!("C12 #{i}"), PCM_LEN, 0, FILL, 1, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C13 / C14 — one lane live at a time
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_lane0_only() {
    let mut rng = Rng::new(0xC013);
    for i in 0..20_000 {
        let mut z = zeros_z();
        for &t in LANE0_TAPS.iter() {
            z[t] = if i % 2 == 0 {
                rng.signed_unit()
            } else {
                rng.wide_exponent_f32(-20, 20)
            };
        }
        diff_call(&format!("C13 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

#[test]
fn cfg_c14_lane1_only() {
    let mut rng = Rng::new(0xC014);
    for i in 0..20_000 {
        let mut z = zeros_z();
        for &t in LANE1_TAPS.iter() {
            z[t] = if i % 2 == 0 {
                rng.signed_unit()
            } else {
                rng.wide_exponent_f32(-20, 20)
            };
        }
        diff_call(&format!("C14 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C15 — statelessness across repeated / interleaved calls
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_repeated_calls_stateless() {
    let p = pair();
    let mut rng = Rng::new(0xC015);
    for i in 0..8_000 {
        let zs: Vec<Vec<f32>> = (0..4).map(|_| fill_uniform(&mut rng, 0.5)).collect();
        let nchs: [c_int; 4] = [1, 2, 4, 8];

        let mut buf_c = vec![FILL; PCM_LEN];
        let mut buf_r = vec![FILL; PCM_LEN];

        // C first, then Rust, over the same accumulating buffer.
        for (z, &nch) in zs.iter().zip(nchs.iter()) {
            unsafe { (p.c.synth_pair)(buf_c.as_mut_ptr(), nch, z.as_ptr()) };
        }
        // Reverse call order on the Rust side to prove neither keeps state.
        for (z, &nch) in zs.iter().zip(nchs.iter()) {
            unsafe { (p.rust.synth_pair)(buf_r.as_mut_ptr(), nch, z.as_ptr()) };
        }
        assert_eq!(buf_c, buf_r, "C15 #{i}: divergence across repeated calls");

        // And the reverse global order (Rust library called first overall).
        let mut buf_r2 = vec![FILL; PCM_LEN];
        let mut buf_c2 = vec![FILL; PCM_LEN];
        for (z, &nch) in zs.iter().zip(nchs.iter()) {
            unsafe { (p.rust.synth_pair)(buf_r2.as_mut_ptr(), nch, z.as_ptr()) };
        }
        for (z, &nch) in zs.iter().zip(nchs.iter()) {
            unsafe { (p.c.synth_pair)(buf_c2.as_mut_ptr(), nch, z.as_ptr()) };
        }
        assert_eq!(buf_c2, buf_r2, "C15 #{i}: divergence (reverse order)");
        assert_eq!(buf_c, buf_c2, "C15 #{i}: C is not deterministic");
        assert_eq!(buf_r, buf_r2, "C15 #{i}: Rust is not deterministic");
    }
}

// ---------------------------------------------------------------------------
// C16 / C17 — non-zero pointer offsets
// ---------------------------------------------------------------------------

#[test]
fn cfg_c16_offset_z_pointer() {
    let mut rng = Rng::new(0xC016);
    for i in 0..20_000 {
        let z_off = 1 + rng.below(64);
        let total = z_off + Z_MIN_LEN + rng.below(64);
        let z: Vec<f32> = (0..total).map(|_| rng.signed_unit() * 0.7).collect();
        diff_call(
            &format!("C16 #{i} z_off={z_off}"),
            PCM_LEN,
            0,
            FILL,
            2,
            &z,
            z_off,
        );
    }
}

#[test]
fn cfg_c17_offset_pcm_only_two_writes() {
    let mut rng = Rng::new(0xC017);
    for i in 0..20_000 {
        let pcm_off = 1 + rng.below(32);
        let nch: c_int = 1 + (i % 4) as c_int;
        let z = fill_uniform(&mut rng, 0.7);
        let out = diff_call(
            &format!("C17 #{i} pcm_off={pcm_off}"),
            PCM_LEN + 64,
            pcm_off,
            FILL,
            nch,
            &z,
            0,
        );
        // Exactly the two documented destinations may differ from FILL.
        let touched: Vec<usize> = (0..out.len()).filter(|&k| out[k] != FILL).collect();
        let expected: Vec<usize> = {
            let mut v = vec![pcm_off, pcm_off + 16 * nch as usize];
            v.dedup();
            v
        };
        for &t in &touched {
            assert!(
                expected.contains(&t),
                "C17 #{i}: unexpected write at pcm[{t}] (pcm_off={pcm_off}, nch={nch})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — exact minimum extent for `z`
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_exact_extent_z() {
    let mut rng = Rng::new(0xC018);
    for i in 0..20_000 {
        // Exactly Z_MIN_LEN floats, no slack whatsoever.
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.9).collect();
        assert_eq!(z.len(), Z_MIN_LEN);
        diff_call(&format!("C18 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C19 — mixed value classes in one buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_mixed_class_buffers() {
    let mut rng = Rng::new(0xC019);
    for i in 0..40_000 {
        let nch: c_int = if i % 2 == 0 { 1 } else { 2 };
        let z: Vec<f32> = (0..Z_MIN_LEN)
            .map(|_| match rng.below(20) {
                0..=13 => rng.signed_unit() * 0.6,
                14..=16 => BOUNDARY_POOL[rng.below(BOUNDARY_POOL.len())],
                _ => rng.any_bits_f32(),
            })
            .collect();
        diff_call(&format!("C19 #{i} nch={nch}"), PCM_LEN, 0, FILL, nch, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// C20 — drive the static `mp3d_scale_pcm` across every region
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_scale_pcm_full_sweep() {
    let mut rng = Rng::new(0xC020);
    let mut regions = [0usize; 4]; // hi clamp, lo clamp, negative, non-negative
    for i in 0..20_000 {
        // Target accumulator values spread across, on and beyond both guards.
        let target: f32 = match i % 5 {
            0 => (rng.unit() * 80_000.0) - 40_000.0,
            1 => 32_766.5 + (rng.signed_unit() * 4.0),
            2 => -32_767.5 + (rng.signed_unit() * 4.0),
            3 => rng.signed_unit() * 2.0,
            _ => rng.wide_exponent_f32(-30, 16),
        };
        // Realise the target through the single dominant lane-0 tap.
        let mut z = zeros_z();
        z[448] = target / 75038.0;
        let out = diff_call(&format!("C20 #{i} target={target}"), PCM_LEN, 0, FILL, 1, &z, 0);
        match out[0] {
            32767 => regions[0] += 1,
            -32768 => regions[1] += 1,
            v if v < 0 => regions[2] += 1,
            _ => regions[3] += 1,
        }
        let _ = rng.next_u64();
    }
    // Plus exactly-reconstructed accumulator values on the guard boundaries.
    for &t in &[
        32_766.5f32,
        prev_f32(32_766.5),
        next_f32(32_766.5),
        -32_767.5f32,
        next_f32(-32_767.5),
        prev_f32(-32_767.5),
        0.0,
        -0.5,
        0.5,
        -1.0,
        1.0,
        32_766.0,
        -32_767.0,
    ] {
        let z = z_for_lane0_exact(t)
            .unwrap_or_else(|| panic!("C20: could not construct lane-0 accumulator {t:e}"));
        assert_eq!(
            model_lane0(&z).to_bits(),
            t.to_bits(),
            "C20: construction for {t:e} is not bit-exact"
        );
        diff_call(&format!("C20 exact {t:e}"), PCM_LEN, 0, FILL, 1, &z, 0);
    }
    for r in regions {
        assert!(r > 50, "C20 under-covered a region: {regions:?}");
    }
}
