//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `synth_pair` returns `void` and the C library has no error channel at all
//! (no error enum, no sentinel return, no `assert`, no null check — see
//! `ERRORS.md` for the mechanical grep). Its entire rejection surface is the
//! saturating clip in `mp3d_scale_pcm`, so "same error" here means "the same
//! saturated sentinel value (`INT16_MAX` / `INT16_MIN`) in the same output
//! slot", and for the unchecked-UB rows it means "the same fault signal".

mod common;

use common::*;
use std::ffi::c_int;

const PCM_LEN: usize = 4096;
const PCM_MID: usize = 2048;

fn prefill() -> Vec<i16> {
    vec![0x1234i16; PCM_LEN]
}

/// Assert C and Rust agree *and* that the shared result is the specific
/// sentinel the C source mandates for this rejection.
fn assert_sentinel(h: &Harness, label: &str, a1: f32, a2: f32, want1: i16, want2: i16) {
    let z = z_for_accumulators_exact(a1, a2);
    let pre = prefill();
    let out = h.assert_same(label, &z, 2, &pre, PCM_MID);
    assert_eq!(out[PCM_MID], want1, "{label}: pcm[0] sentinel");
    assert_eq!(out[PCM_MID + 32], want2, "{label}: pcm[16*nch] sentinel");
    // And the independently derived model of the C helper agrees.
    assert_eq!(expected_scale_pcm(a1), want1, "{label}: model pcm[0]");
    assert_eq!(expected_scale_pcm(a2), want2, "{label}: model pcm[16*nch]");
}

// ---------------------------------------------------------------------------
// E1–E4 — the two explicit range checks, in both output slots
// ---------------------------------------------------------------------------

#[test]
fn err_e1_e2_e3_e4_saturation() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE1);

    // E1 + E3: both accumulators over the max threshold.
    assert_sentinel(&h, "E1/E3 positive clip", 1.0e6, 1.0e6, 32767, 32767);
    // E2 + E4: both under the min threshold.
    assert_sentinel(&h, "E2/E4 negative clip", -1.0e6, -1.0e6, -32768, -32768);
    // E1 + E4 and E2 + E3: opposite clips in the two slots, so a swapped
    // accumulator would be caught.
    assert_sentinel(&h, "E1/E4 mixed clip", 1.0e6, -1.0e6, 32767, -32768);
    assert_sentinel(&h, "E2/E3 mixed clip", -1.0e6, 1.0e6, -32768, 32767);

    // Randomized: many magnitudes past each threshold.
    for _ in 0..2000 {
        let mag = 32766.5f32 + rng.scaled(1e7).abs();
        assert_sentinel(&h, "E1/E3 random over-max", mag, mag, 32767, 32767);
        let mag = -32767.5f32 - rng.scaled(1e7).abs();
        assert_sentinel(&h, "E2/E4 random under-min", mag, mag, -32768, -32768);
    }
}

// ---------------------------------------------------------------------------
// E5 / E6 — the thresholds are INCLUSIVE (`>=` and `<=`)
// ---------------------------------------------------------------------------

#[test]
fn err_e5_e6_inclusive_clip_boundaries() {
    let h = Harness::load();
    // Exactly on the boundary: the comparison is `>=` / `<=`, so these clip.
    assert_sentinel(&h, "E5 a == 32766.5 exactly", 32766.5, 32766.5, 32767, 32767);
    assert_sentinel(&h, "E6 a == -32767.5 exactly", -32767.5, -32767.5, -32768, -32768);
    // One ULP past the boundary, still clipped.
    let up = f32::from_bits(32766.5f32.to_bits() + 1);
    let dn = f32::from_bits((-32767.5f32).to_bits() + 1); // more negative
    assert_sentinel(&h, "E5 one ULP above 32766.5", up, up, 32767, 32767);
    assert_sentinel(&h, "E6 one ULP below -32767.5", dn, dn, -32768, -32768);
}

// ---------------------------------------------------------------------------
// E7 / E8 — one representable step INSIDE the range: must NOT clip
// ---------------------------------------------------------------------------

#[test]
fn err_e7_e8_one_step_inside_range() {
    let h = Harness::load();
    let just_below = f32::from_bits(32766.5f32.to_bits() - 1); // 32766.498046875
    let just_above = f32::from_bits((-32767.5f32).to_bits() - 1); // -32767.498046875
    assert_eq!(just_below, 32766.498046875);
    assert_eq!(just_above, -32767.498046875);
    // E7: falls through, truncates to 32766, s >= 0 so no decrement.
    assert_sentinel(&h, "E7 largest f32 below 32766.5", just_below, just_below, 32766, 32766);
    // E8: falls through, truncates to -32766, s < 0 so decrement to -32767.
    assert_sentinel(&h, "E8 smallest f32 above -32767.5", just_above, just_above, -32767, -32767);
}

// ---------------------------------------------------------------------------
// E9 / E10 — the `s -= (s < 0)` conditional
// ---------------------------------------------------------------------------

#[test]
fn err_e9_e10_negative_decrement() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE9);

    // E9: negative, non-clipped -> `(int16_t)(a + .5f)` truncates TOWARD ZERO,
    // and only then does `s -= (s < 0)` subtract one. So e.g. a = -1.0 gives
    // trunc(-0.5) == 0, `s < 0` is false, and the result is 0 -- NOT -1.
    for &(a, want) in &[
        (-1.5f32, -2i16),
        (-2.0, -2),
        (-2.5, -3),
        (-100.25, -100),
        (-100.75, -101),
        (-32766.0, -32766),
        (-32767.0, -32767),
    ] {
        assert_sentinel(&h, &format!("E9 a={a}"), a, a, want, want);
    }

    // E10: negative but truncating to 0 -> `s < 0` is false, no decrement.
    for &a in &[-0.25f32, -0.4999, -0.1, -0.5, -1.0, -1.4999] {
        assert_sentinel(&h, &format!("E10 a={a} -> 0"), a, a, 0, 0);
    }
    // -0.0 cannot be produced by a single tap; the all-negative-zero sign
    // pattern that does produce it is covered by
    // `differential::cfg_c2b_negative_zero_accumulator`.

    // Randomized sweep over the whole non-clipped range.
    for _ in 0..4000 {
        let a = rng.scaled(32760.0);
        let want = expected_scale_pcm(a);
        if let Some(z) = z_for_accumulators(a, a) {
            let pre = prefill();
            let out = h.assert_same(&format!("E9/E10 random a={a}"), &z, 2, &pre, PCM_MID);
            assert_eq!(out[PCM_MID], want, "E9/E10 random a={a}");
            assert_eq!(out[PCM_MID + 32], want, "E9/E10 random a={a}");
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — NaN falls through BOTH range checks (the UB cast)
// ---------------------------------------------------------------------------

#[test]
fn err_e11_nan() {
    let h = Harness::load();
    // Every NaN comparison is false, so neither clip fires; GCC's
    // `cvttss2si` -> 0x80000000 narrows to int16_t 0.
    assert_sentinel(&h, "E11 quiet NaN", f32::NAN, f32::NAN, 0, 0);

    // NaN with assorted payloads and both signs, through both blocks.
    let pre = prefill();
    for bits in [
        0x7FC0_0000u32,
        0xFFC0_0000,
        0x7F80_0001,
        0xFF80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x7FAA_5555,
    ] {
        let nan = f32::from_bits(bits);
        assert!(nan.is_nan());
        let z = z_for_accumulators_exact(nan, nan);
        let out = h.assert_same(&format!("E11 NaN payload {bits:08x}"), &z, 2, &pre, PCM_MID);
        assert_eq!(out[PCM_MID], 0, "E11 NaN {bits:08x} pcm[0]");
        assert_eq!(out[PCM_MID + 32], 0, "E11 NaN {bits:08x} pcm[16*nch]");
    }

    // NaN injected at each individual tap, so it propagates through every
    // add/sub/multiply pairing rather than only the last term.
    for tap in 0..N_TAPS {
        for &blk2 in &[false, true] {
            let mut z = z_zeros();
            if blk2 {
                set_tap2(&mut z, tap, f32::NAN);
            } else {
                set_tap1(&mut z, tap, f32::NAN);
            }
            h.assert_same(
                &format!("E11 NaN at tap {tap} block{}", if blk2 { 2 } else { 1 }),
                &z,
                2,
                &pre,
                PCM_MID,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 — infinities
// ---------------------------------------------------------------------------

#[test]
fn err_e12_e13_infinities() {
    let h = Harness::load();
    assert_sentinel(&h, "E12 +Inf", f32::INFINITY, f32::INFINITY, 32767, 32767);
    assert_sentinel(&h, "E13 -Inf", f32::NEG_INFINITY, f32::NEG_INFINITY, -32768, -32768);
    assert_sentinel(&h, "E12/E13 mixed", f32::INFINITY, f32::NEG_INFINITY, 32767, -32768);

    // Inf at each tap: subtraction pairings turn Inf - Inf into NaN, which is a
    // different downstream branch again.
    let pre = prefill();
    for tap in 0..N_TAPS {
        for inf in [f32::INFINITY, f32::NEG_INFINITY] {
            let mut z = z_zeros();
            set_tap1(&mut z, tap, inf);
            set_tap2(&mut z, tap, inf);
            h.assert_same(&format!("E12/E13 Inf at tap {tap} ({inf})"), &z, 2, &pre, PCM_MID);
        }
    }
    // Both operands of every subtraction pair infinite -> Inf - Inf == NaN.
    for &(hi, lo) in &[(14usize, 0usize), (12, 2), (10, 4), (8, 6)] {
        for inf in [f32::INFINITY, f32::NEG_INFINITY] {
            let mut z = z_zeros();
            set_tap1(&mut z, hi, inf);
            set_tap1(&mut z, lo, inf);
            h.assert_same(&format!("E12/E13 Inf-Inf pair ({hi},{lo})"), &z, 2, &pre, PCM_MID);
        }
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 — null pointers (unchecked UB) — compared OUT OF PROCESS
// ---------------------------------------------------------------------------

/// Child mode: performs one deliberately faulting call and is expected to die.
/// Selected by the `HARVEST_FAULT` env var; a normal test run sets nothing and
/// this returns immediately.
#[test]
fn fault_child() {
    let mode = match std::env::var("HARVEST_FAULT") {
        Ok(m) => m,
        Err(_) => return, // not the child; nothing to do
    };
    let h = Harness::load();
    let z = z_zeros();
    let mut pcm = vec![0i16; 4096];
    unsafe {
        match mode.as_str() {
            "c_null_pcm" => h.call_raw_c(std::ptr::null_mut(), 2, z.as_ptr()),
            "r_null_pcm" => h.call_raw_rust(std::ptr::null_mut(), 2, z.as_ptr()),
            "c_null_z" => h.call_raw_c(pcm.as_mut_ptr(), 2, std::ptr::null()),
            "r_null_z" => h.call_raw_rust(pcm.as_mut_ptr(), 2, std::ptr::null()),
            "c_null_both" => h.call_raw_c(std::ptr::null_mut(), 2, std::ptr::null()),
            "r_null_both" => h.call_raw_rust(std::ptr::null_mut(), 2, std::ptr::null()),
            other => panic!("unknown fault mode {other}"),
        }
    }
    // If we get here the call did not fault; report it so the parent sees a
    // clean exit code rather than a signal.
    println!("survived {mode}");
    std::process::exit(0);
}

/// Outcome of a child run: `Ok(())` = exited 0, `Err(sig)` = killed by signal.
fn run_fault_child(mode: &str) -> Result<(), i32> {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "fault_child", "--nocapture", "--test-threads=1"])
        .env("HARVEST_FAULT", mode)
        .output()
        .expect("spawn fault child");
    match out.status.code() {
        Some(0) => Ok(()),
        Some(c) => Err(-c), // non-signal failure, distinguished from signals
        None => {
            // Killed by a signal; `ExitStatusExt` gives us which one.
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    }
}

#[test]
fn err_e14_e15_null_pointers() {
    // E14: pcm == NULL. E15: z == NULL. Neither is checked by the C, so both
    // must fault, and the Rust must fault the SAME way (same signal).
    for (c_mode, r_mode, row) in [
        ("c_null_pcm", "r_null_pcm", "E14 pcm==NULL"),
        ("c_null_z", "r_null_z", "E15 z==NULL"),
        ("c_null_both", "r_null_both", "E14+E15 both NULL"),
    ] {
        let c = run_fault_child(c_mode);
        let r = run_fault_child(r_mode);
        assert_eq!(
            c, r,
            "{row}: C outcome {c:?} != Rust outcome {r:?} (expected both SIGSEGV = Err(11))"
        );
        assert_eq!(
            c,
            Err(11),
            "{row}: expected SIGSEGV from the unchecked null dereference, got {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E16 — nch == 0: the two stores collide, second wins
// ---------------------------------------------------------------------------

#[test]
fn err_e16_nch_zero_second_store_wins() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE16);
    for _ in 0..1000 {
        // Make the two accumulators land on distinguishable sentinels so the
        // "which store wins" question has an observable answer.
        let (a1, a2) = (1.0e6f32, -1.0e6f32);
        let z = z_for_accumulators_exact(a1, a2);
        let pre: Vec<i16> = (0..PCM_LEN).map(|_| rng.next_u32() as i16).collect();
        let out = h.assert_same("E16 nch=0", &z, 0, &pre, PCM_MID);
        assert_eq!(
            out[PCM_MID], -32768,
            "E16: with nch=0 the pcm[16*nch] store must overwrite pcm[0]"
        );
    }
}

// ---------------------------------------------------------------------------
// E17 — negative nch: signed pointer arithmetic, must not wrap through usize
// ---------------------------------------------------------------------------

#[test]
fn err_e17_negative_nch() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE17);
    for nch in -64i32..0 {
        let (a1, a2) = (1.0e6f32, -1.0e6f32);
        let z = z_for_accumulators_exact(a1, a2);
        let pre: Vec<i16> = (0..PCM_LEN).map(|_| rng.next_u32() as i16).collect();
        let out = h.assert_same(&format!("E17 nch={nch}"), &z, nch, &pre, PCM_MID);
        let idx = (PCM_MID as isize + 16 * nch as isize) as usize;
        assert_eq!(out[PCM_MID], 32767, "E17 nch={nch}: pcm[0]");
        assert_eq!(out[idx], -32768, "E17 nch={nch}: pcm[16*nch] at index {idx}");
    }
}

// ---------------------------------------------------------------------------
// E18 — `16 * nch` overflows int
// ---------------------------------------------------------------------------

#[test]
fn err_e18_nch_int_overflow() {
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE18);
    // 16 * 2^28 == 0 (mod 2^32), so nch and nch + m*2^28 share a wrapped
    // offset. INT_MAX -> -16 elements; INT_MIN -> 0 elements.
    let cases: [(c_int, isize); 6] = [
        (i32::MAX, -16),
        (i32::MIN, 0),
        (0x0FFF_FFFF, -16),
        (0x1000_0000, 0),
        (0x1000_0001, 16),
        (-0x0FFF_FFFF, 16),
    ];
    for &(nch, want_off) in &cases {
        assert_eq!(
            16i32.wrapping_mul(nch) as isize,
            want_off,
            "E18 model: 16*{nch} should wrap to {want_off}"
        );
        let z = z_for_accumulators_exact(1.0e6, -1.0e6);
        let pre: Vec<i16> = (0..PCM_LEN).map(|_| rng.next_u32() as i16).collect();
        let out = h.assert_same(&format!("E18 nch={nch}"), &z, nch, &pre, PCM_MID);
        let idx = (PCM_MID as isize + want_off) as usize;
        if idx == PCM_MID {
            assert_eq!(out[idx], -32768, "E18 nch={nch}: collided store");
        } else {
            assert_eq!(out[PCM_MID], 32767, "E18 nch={nch}: pcm[0]");
            assert_eq!(out[idx], -32768, "E18 nch={nch}: wrapped store at {idx}");
        }
    }
}

// ---------------------------------------------------------------------------
// E19 — z buffer sized EXACTLY to the taps that are read
// ---------------------------------------------------------------------------

#[test]
fn err_e19_short_z_buffer() {
    // The highest float the C reads is z[2 + 14*64] = z[898], so 899 floats is
    // the exact minimum. Allocating exactly that (and no more) means any
    // over-read walks off the end of the allocation, which a tighter allocator
    // or ASAN would trap. Both sides must read the same taps and no more.
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE19);
    const EXACT: usize = 899;
    for _ in 0..2000 {
        let mut z = vec![0.0f32; EXACT];
        for i in 0..N_TAPS {
            z[i * 64] = rng.mixed();
            z[2 + i * 64] = rng.mixed();
        }
        let pre = prefill();
        let mut c_pcm = pre.clone();
        let mut r_pcm = pre.clone();
        unsafe {
            h.call_raw_c(c_pcm.as_mut_ptr().add(PCM_MID), 2, z.as_ptr());
            h.call_raw_rust(r_pcm.as_mut_ptr().add(PCM_MID), 2, z.as_ptr());
        }
        assert_eq!(c_pcm, r_pcm, "E19 exact-size z buffer divergence");
    }
    // Zero-length / one-element `z` is unconditionally an out-of-bounds read in
    // the C (there is no length parameter to check), so it is covered as a
    // fault row rather than a value row; see E15 for the null case.
}

// ---------------------------------------------------------------------------
// E20 — no enums exist; every int is a valid nch. Sweep the full int range.
// ---------------------------------------------------------------------------

#[test]
fn err_e20_full_int_range_nch() {
    // `include/lib.h` declares no enum, so the "out-of-range enum value"
    // boundary maps onto `int nch`, which is completely unvalidated. Sweep
    // representative values from the entire int range, restricted to those
    // whose (wrapped) store offset stays inside the scratch buffer.
    let h = Harness::load();
    let mut rng = Rng::new(SEED ^ 0xE20);
    let mut tried = 0usize;
    let mut nch_values: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 27,
        -(1 << 27),
        1 << 30,
        -(1 << 30),
    ];
    for _ in 0..4000 {
        nch_values.push(rng.next_u32() as c_int);
    }
    // Values whose *wrapped* offset lands inside the scratch buffer: because
    // 16 * 2^28 == 0 (mod 2^32), nch = k + m*2^28 has the same store offset as
    // nch = k. This is how the full int range gets meaningfully sampled instead
    // of being skipped for faulting out of bounds.
    for k in -64i32..=64 {
        for m in -7i32..=7 {
            nch_values.push(k.wrapping_add(m.wrapping_mul(1 << 28)));
        }
    }
    for &nch in &nch_values {
        let off = 16i32.wrapping_mul(nch) as isize;
        let idx = PCM_MID as isize + off;
        if idx < 0 || idx >= PCM_LEN as isize {
            continue; // would fault; the wrapped offset itself is checked above
        }
        let z = z_for_accumulators_exact(1.0e6, -1.0e6);
        let pre: Vec<i16> = (0..PCM_LEN).map(|_| rng.next_u32() as i16).collect();
        let out = h.assert_same(&format!("E20 nch={nch}"), &z, nch, &pre, PCM_MID);
        let idx = idx as usize;
        if idx == PCM_MID {
            assert_eq!(out[idx], -32768);
        } else {
            assert_eq!(out[PCM_MID], 32767);
            assert_eq!(out[idx], -32768);
        }
        tried += 1;
    }
    assert!(tried >= 1000, "E20 only exercised {tried} nch values");
}
