//! Phase C -- error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`.  `synth_pair` has no error channel, so its
//! "rejections" are the two saturation branches of `mp3d_scale_pcm`, the
//! fall-through for values neither branch catches (`NaN`), the `s -= (s < 0)`
//! correction, and the undefined behaviour reachable through its unvalidated
//! parameters.  Each test asserts the *same* concrete sentinel value (not merely
//! "both did something"), and the UB rows assert the same fatal signal.

mod harness;

use harness::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The two samples the C writes, read back out of a buffer.
fn outputs(buf: &PcmBuf, nch: i32) -> (i16, i16) {
    let off = second_store_offset(nch);
    (
        buf.data[buf.base],
        buf.data[(buf.base as isize + off) as usize],
    )
}

/// `z` that puts `chain`'s accumulator approximately at `target`, using that
/// chain's largest-magnitude coefficient.
fn z_approx(chain: Chain, target: f32) -> Vec<f32> {
    let (idx, w) = match chain {
        Chain::Lo => CHAIN0[7],  // z[7*64]      * 75038
        Chain::Hi => CHAIN1[4],  // z[2 + 8*64]  * 64019
    };
    z_single(idx, target / w)
}

/// `z` that puts each chain's accumulator *exactly* on the requested value.
#[track_caller]
fn z_exact_two(t0: Option<f32>, t1: Option<f32>) -> Vec<f32> {
    let mut z = z_zero();
    if let Some(t) = t0 {
        let (i, v) = find_single_tap_exact(Chain::Lo, t)
            .unwrap_or_else(|| panic!("chain0 cannot reach {t:e} exactly"));
        z[i] = v;
    }
    if let Some(t) = t1 {
        let (i, v) = find_single_tap_exact(Chain::Hi, t)
            .unwrap_or_else(|| panic!("chain1 cannot reach {t:e} exactly"));
        z[i] = v;
    }
    z
}

/// Differential call plus an assertion on the concrete sample the C produced.
#[track_caller]
fn assert_sample(label: &str, chain: Chain, nch: i32, z: &[f32], expected: i16) {
    let buf = assert_same(label, nch, z);
    let (o0, o1) = outputs(&buf, nch);
    let got = match chain {
        Chain::Lo => o0,
        Chain::Hi => o1,
    };
    assert_eq!(got, expected, "{label}: expected sample {expected}, got {got}");
}

// ---------------------------------------------------------------------------
// rows 1-3: clamp high on pcm[0]
// ---------------------------------------------------------------------------

#[test]
fn err01_clamp_high_out0() {
    // `if (sample >= 32766.5) return 32767;`
    for t in [
        32766.5f32, 32767.0, 32768.0, 40000.0, 1e6, 1e12, 1e30, f32::MAX,
    ] {
        let z = z_approx(Chain::Lo, t);
        assert_sample(&format!("err01 t={t:e}"), Chain::Lo, 2, &z, 32767);
    }
}

#[test]
fn err02_clamp_high_boundary_exact() {
    // The comparison is `>=`, so exactly 32766.5 clamps.
    let z = z_exact_two(Some(32766.5), None);
    assert_sample("err02", Chain::Lo, 2, &z, 32767);
}

#[test]
fn err03_clamp_high_one_ulp_below() {
    // One ULP below the boundary must NOT clamp: 32766.498046875 + 0.5
    // truncates to 32766.
    let a = nudge(32766.5, -1);
    assert_eq!(a, 32766.498046875);
    let z = z_exact_two(Some(a), None);
    assert_sample("err03", Chain::Lo, 2, &z, 32766);
}

// ---------------------------------------------------------------------------
// rows 4-6: clamp low on pcm[0]
// ---------------------------------------------------------------------------

#[test]
fn err04_clamp_low_out0() {
    // `if (sample <= -32767.5) return -32768;`
    for t in [
        -32767.5f32, -32768.0, -40000.0, -1e6, -1e12, -1e30, -f32::MAX,
    ] {
        let z = z_approx(Chain::Lo, t);
        assert_sample(&format!("err04 t={t:e}"), Chain::Lo, 2, &z, -32768);
    }
}

#[test]
fn err05_clamp_low_boundary_exact() {
    // The comparison is `<=`, so exactly -32767.5 clamps.
    let z = z_exact_two(Some(-32767.5), None);
    assert_sample("err05", Chain::Lo, 2, &z, -32768);
}

#[test]
fn err06_clamp_low_one_ulp_above() {
    // -32767.498046875 + 0.5 = -32766.998..., truncates to -32766, then
    // `s -= (s < 0)` makes it -32767.
    let a = nudge(-32767.5, 1);
    assert_eq!(a, -32767.498046875);
    let z = z_exact_two(Some(a), None);
    assert_sample("err06", Chain::Lo, 2, &z, -32767);
}

// ---------------------------------------------------------------------------
// rows 7-8: the same clamps on the second store site
// ---------------------------------------------------------------------------

#[test]
fn err07_clamp_high_out1() {
    for t in [32766.5f32, 40000.0, 1e20, f32::MAX] {
        let z = z_approx(Chain::Hi, t);
        for nch in [1, 2, -2] {
            assert_sample(&format!("err07 t={t:e} nch={nch}"), Chain::Hi, nch, &z, 32767);
        }
    }
    let z = z_exact_two(None, Some(32766.5));
    assert_sample("err07-exact", Chain::Hi, 2, &z, 32767);
}

#[test]
fn err08_clamp_low_out1() {
    for t in [-32767.5f32, -40000.0, -1e20, -f32::MAX] {
        let z = z_approx(Chain::Hi, t);
        for nch in [1, 2, -2] {
            assert_sample(&format!("err08 t={t:e} nch={nch}"), Chain::Hi, nch, &z, -32768);
        }
    }
    let z = z_exact_two(None, Some(-32767.5));
    assert_sample("err08-exact", Chain::Hi, 2, &z, -32768);
    // One ULP above the boundary on this chain too.
    let z = z_exact_two(None, Some(nudge(-32767.5, 1)));
    assert_sample("err08-ulp", Chain::Hi, 2, &z, -32767);
}

// ---------------------------------------------------------------------------
// rows 9-10: infinities
// ---------------------------------------------------------------------------

#[test]
fn err09_plus_infinity() {
    // A `+inf` tap drives the accumulator to `sign(coefficient) * inf`.
    for chain in [Chain::Lo, Chain::Hi] {
        for &(idx, w) in chain.taps() {
            let expected = if w > 0.0 { 32767 } else { -32768 };
            let z = z_single(idx, f32::INFINITY);
            assert_sample(
                &format!("err09 {chain:?} idx={idx} w={w}"),
                chain,
                2,
                &z,
                expected,
            );
        }
    }
}

#[test]
fn err10_minus_infinity() {
    for chain in [Chain::Lo, Chain::Hi] {
        for &(idx, w) in chain.taps() {
            let expected = if w > 0.0 { -32768 } else { 32767 };
            let z = z_single(idx, f32::NEG_INFINITY);
            assert_sample(
                &format!("err10 {chain:?} idx={idx} w={w}"),
                chain,
                2,
                &z,
                expected,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 11: NaN falls through BOTH range checks
// ---------------------------------------------------------------------------

#[test]
fn err11_nan_accumulator() {
    // `comiss` leaves CF=1 for unordered compares, so both `if`s are false and
    // the C-undefined `(int16_t)(NaN + .5f)` conversion runs.  On x86-64
    // `cvttss2si` yields 0x80000000, whose low 16 bits are 0, and `s < 0` is
    // then false -- the sample is 0.
    for nan in [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FFF_FFFF),
    ] {
        for chain in [Chain::Lo, Chain::Hi] {
            let (idx, _) = chain.taps()[0];
            let z = z_single(idx, nan);
            assert_sample(
                &format!("err11 {chain:?} nan=0x{:08X}", nan.to_bits()),
                chain,
                2,
                &z,
                0,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 12: NaN in each individual tap
// ---------------------------------------------------------------------------

#[test]
fn err12_nan_in_each_tap() {
    let mut rng = Rng::new(0xC12);
    for chain in [Chain::Lo, Chain::Hi] {
        for &(idx, _) in chain.taps() {
            for _ in 0..50 {
                // Random elsewhere, NaN at `idx`: the owning chain must yield 0.
                let mut z = z_from(|_| rng.sym(0.3));
                z[idx] = f32::NAN;
                assert_sample(&format!("err12 {chain:?} idx={idx}"), chain, 2, &z, 0);
            }
        }
    }
    // Every read index at once.
    let z = z_const(f32::NAN);
    assert_sample("err12-all-lo", Chain::Lo, 2, &z, 0);
    assert_sample("err12-all-hi", Chain::Hi, 2, &z, 0);
}

// ---------------------------------------------------------------------------
// row 13: inf - inf == NaN inside the accumulation
// ---------------------------------------------------------------------------

#[test]
fn err13_inf_minus_inf_is_nan() {
    for chain in [Chain::Lo, Chain::Hi] {
        let taps: Vec<(usize, f32)> = chain.taps().to_vec();
        for i in 0..taps.len() {
            for j in 0..taps.len() {
                if i == j {
                    continue;
                }
                let mut z = z_zero();
                // Both terms are driven to +inf and -inf respectively.
                z[taps[i].0] = f32::INFINITY * taps[i].1.signum();
                z[taps[j].0] = f32::NEG_INFINITY * taps[j].1.signum();
                assert_sample(&format!("err13 {chain:?} {i}/{j}"), chain, 2, &z, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 14-15: the `s -= (s < 0)` correction and its boundary
// ---------------------------------------------------------------------------

#[test]
fn err14_negative_zero_region() {
    // For a in (-1.5, 0.5), `(int16_t)(a + .5f)` is 0, so `s < 0` is FALSE and
    // no decrement happens: the sample is 0, not -1.
    for t in [
        0.0f32, -0.0, -0.25, -0.5, -0.75, -1.0, -1.25, -1.4375, 0.25, 0.375,
    ] {
        let z = z_exact_two(Some(t), Some(t));
        assert_sample(&format!("err14 lo t={t}"), Chain::Lo, 2, &z, 0);
        assert_sample(&format!("err14 hi t={t}"), Chain::Hi, 2, &z, 0);
    }
}

#[test]
fn err15_negative_decrement_branch() {
    // Expected values read straight off the C:
    //   s = (int16_t)(a + .5f);  s -= (s < 0);
    let cases: [(f32, i16); 14] = [
        (-1.5, -2),
        (-2.0, -2),
        (-2.5, -3),
        (-3.0, -3),
        (-3.5, -4),
        (-4.0, -4),
        (-100.5, -101),
        (-100.0, -100),
        (0.5, 1),
        (1.5, 2),
        (2.5, 3),
        (2.0, 2),
        (100.0, 100),
        (100.5, 101),
    ];
    for (t, expected) in cases {
        let z = z_exact_two(Some(t), Some(t));
        assert_sample(&format!("err15 lo a={t}"), Chain::Lo, 2, &z, expected);
        assert_sample(&format!("err15 hi a={t}"), Chain::Hi, 2, &z, expected);
    }
}

// ---------------------------------------------------------------------------
// row 16: nch == 0 aliases the two stores
// ---------------------------------------------------------------------------

#[test]
fn err16_nch_zero_aliases_store() {
    // Two clearly distinguishable samples; the second store must win.
    let z = z_exact_two(Some(1000.0), Some(-2000.0));
    let buf = assert_same("err16", 0, &z);
    assert_eq!(buf.data[buf.base], -2000, "second store must overwrite pcm[0]");
    for i in buf.touched() {
        assert_eq!(i, buf.base, "nch=0 must touch only pcm[0]");
    }
    // Sanity: with nch=1 both samples are visible and distinct.
    let buf = assert_same("err16-ref", 1, &z);
    assert_eq!(outputs(&buf, 1), (1000, -2000));
}

// ---------------------------------------------------------------------------
// row 17: negative nch writes before pcm
// ---------------------------------------------------------------------------

#[test]
fn err17_negative_nch() {
    let z = z_exact_two(Some(777.0), Some(-888.0));
    for nch in [-1, -2, -3, -8, -64, -1000] {
        let buf = assert_same(&format!("err17 nch={nch}"), nch, &z);
        let off = second_store_offset(nch);
        assert_eq!(off, 16 * nch as isize);
        assert_eq!(buf.data[buf.base], 777);
        assert_eq!(buf.data[(buf.base as isize + off) as usize], -888);
    }
}

// ---------------------------------------------------------------------------
// rows 18 & 22: `16 * nch` overflows `int`
// ---------------------------------------------------------------------------

#[test]
fn err22_nch_int_overflow_wrap_semantics() {
    // gcc computes `16 * nch` in `int` with two's-complement wraparound; these
    // nch values all wrap to a small offset, so the store stays in our buffer
    // and the wrap semantics are directly observable.
    let z = z_exact_two(Some(1234.0), Some(-4321.0));
    let mut cases: Vec<(i32, isize)> = vec![
        (i32::MAX, -16),          // 16 * 0x7FFFFFFF == -16 (mod 2^32)
        (i32::MIN, 0),            // 16 * -2^31      == 0
        (0x1000_0000, 0),         // 16 * 2^28       == 2^32 == 0
        (-0x1000_0000, 0),
        (0x0FFF_FFFF, -16),
        (i32::MAX - 1, -32),
        (0x1000_0001, 16),
    ];
    // nch = off/16 + k * 2^28 must all give the same wrapped offset.
    for k in -4i32..=4 {
        for off in [-4096i32, -256, -16, 0, 16, 256, 4096] {
            let nch = (off / 16).wrapping_add(k.wrapping_mul(0x1000_0000));
            cases.push((nch, off as isize));
        }
    }
    for (nch, want_off) in cases {
        assert_eq!(
            second_store_offset(nch),
            want_off,
            "wrapped offset for nch={nch}"
        );
        let buf = assert_same(&format!("err22 nch={nch}"), nch, &z);
        if want_off == 0 {
            assert_eq!(buf.data[buf.base], -4321, "aliased store for nch={nch}");
        } else {
            assert_eq!(buf.data[buf.base], 1234);
            assert_eq!(buf.data[(buf.base as isize + want_off) as usize], -4321);
        }
    }
}

#[test]
fn err18_nch_int_overflow_parity() {
    // These wrap to +/-2^31, i.e. a store gigabytes away from `pcm`: undefined
    // behaviour in C.  Both libraries must fail in exactly the same way.
    for nch in [0x0800_0000i32, -0x0800_0000, 0x0800_0001, 0x1800_0000] {
        ub_parity(&format!("nch_overflow:{nch}"));
    }
}

// ---------------------------------------------------------------------------
// rows 19-20: null pointers (there is no null check anywhere in the C)
// ---------------------------------------------------------------------------

#[test]
fn err19_null_pcm_crash_parity() {
    ub_parity("null_pcm");
}

#[test]
fn err20_null_z_crash_parity() {
    ub_parity("null_z");
    ub_parity("null_both");
}

// ---------------------------------------------------------------------------
// row 21: a `z` shorter than 899 floats -- both must read the same indices
// ---------------------------------------------------------------------------

#[test]
fn err21_short_z_reads_same_indices() {
    // The C reads up to z[898] unconditionally.  For every "logical length" the
    // caller might have, both libraries must read exactly the same slots, so
    // filling the tail with distinguishable garbage must not desynchronise them.
    let mut rng = Rng::new(0xC21);
    for l in [
        1usize, 2, 3, 64, 65, 66, 128, 129, 449, 450, 512, 640, 897, 898, 899,
    ] {
        for _ in 0..40 {
            let mut z = vec![0f32; Z_LEN];
            for i in 0..Z_LEN {
                z[i] = if i < l {
                    rng.sym(2.0)
                } else {
                    // "past the end": hostile values.
                    *rng.pick(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30])
                };
            }
            assert_same(&format!("err21 len={l}"), 2, &z);
        }
    }
}

// ---------------------------------------------------------------------------
// Extra generic FFI boundaries
// ---------------------------------------------------------------------------

#[test]
fn extra_nch_exhaustive_small_and_random_bit_patterns() {
    // `int nch` has no valid-range documentation and no validation, so every
    // bit pattern is a real input.  All values whose wrapped offset lands in a
    // safe window are exercised in-process.
    let z = z_exact_two(Some(-5.0), Some(9.0));
    for nch in -256i32..=256 {
        let buf = assert_same(&format!("extra nch={nch}"), nch, &z);
        let off = second_store_offset(nch);
        if off == 0 {
            assert_eq!(buf.data[buf.base], 9);
        } else {
            assert_eq!(buf.data[buf.base], -5);
            assert_eq!(buf.data[(buf.base as isize + off) as usize], 9);
        }
    }
    // Random bit patterns, normalised into the safe window while preserving the
    // low bits that decide the wrapped offset.
    let mut rng = Rng::new(0xC99);
    for _ in 0..2000 {
        let raw = rng.next_u32() as i32;
        let nch = ((raw & 0xFF) - 128).wrapping_add((raw >> 8).wrapping_mul(0x1000_0000));
        if second_store_offset(nch).unsigned_abs() > 1 << 12 {
            continue;
        }
        assert_same(&format!("extra rand nch={nch}"), nch, &z);
    }
}

#[test]
fn extra_pcm_and_z_at_allocation_edges() {
    // `z` exactly 899 floats long (no slack at all) and `pcm` exactly two
    // samples long: any over-read/over-write would be a real OOB access.
    let mut rng = Rng::new(0xC98);
    for _ in 0..300 {
        let z: Vec<f32> = (0..Z_LEN).map(|_| rng.sym(1.0)).collect();
        assert_eq!(z.len(), Z_LEN);
        let mut c_buf = vec![PCM_POISON; 17];
        let mut r_buf = vec![PCM_POISON; 17];
        let c = c_synth_pair();
        let r = rust_synth_pair();
        unsafe {
            c(c_buf.as_mut_ptr(), 1, z.as_ptr());
            r(r_buf.as_mut_ptr(), 1, z.as_ptr());
        }
        assert_eq!(c_buf, r_buf, "tight-allocation mismatch");
    }
}

// ---------------------------------------------------------------------------
// Crash-parity plumbing: run the UB case in a child process, once per library,
// and require identical termination status.
// ---------------------------------------------------------------------------

const UB_CASE_ENV: &str = "SP_UB_CASE";
const UB_IMPL_ENV: &str = "SP_UB_IMPL";

#[track_caller]
fn ub_parity(case: &str) {
    // Make sure both libraries are built/loadable in the parent first, so a
    // build failure is not mistaken for a crash.
    let _ = c_library_path();
    let _ = rust_library_path();

    let exe = std::env::current_exe().expect("current_exe");
    let mut results = Vec::new();
    for which in ["c", "rust"] {
        let out = Command::new(&exe)
            .args([
                "--exact",
                "ub_child",
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(UB_CASE_ENV, case)
            .env(UB_IMPL_ENV, which)
            .output()
            .expect("spawn ub_child");
        results.push((out.status.code(), out.status.signal(), out));
    }
    let (c_code, c_sig, c_out) = &results[0];
    let (r_code, r_sig, r_out) = &results[1];
    assert_eq!(
        (c_code, c_sig),
        (r_code, r_sig),
        "UB case {case}: C exited (code={c_code:?}, signal={c_sig:?}) but Rust \
         exited (code={r_code:?}, signal={r_sig:?})\n--- C stderr ---\n{}\n\
         --- RUST stderr ---\n{}",
        String::from_utf8_lossy(&c_out.stderr),
        String::from_utf8_lossy(&r_out.stderr),
    );
    eprintln!("UB case {case}: both terminated with code={c_code:?} signal={c_sig:?}");
    // A silent success would mean the case did not actually reach the UB.
    assert!(
        c_sig.is_some() || c_code == &Some(0),
        "UB case {case}: unexpected termination"
    );
}

/// Child-process helper: performs one undefined-behaviour call against one of
/// the two libraries.  Ignored by default; only ever run by [`ub_parity`].
#[test]
#[ignore = "child-process helper for the crash-parity tests"]
fn ub_child() {
    let Ok(case) = std::env::var(UB_CASE_ENV) else {
        return; // invoked directly with --ignored; nothing to do
    };
    let which = match std::env::var(UB_IMPL_ENV).unwrap_or_default().as_str() {
        "c" => Impl::C,
        _ => Impl::Rust,
    };
    let f = impl_fn(which);
    let z = z_from(|_| 0.25);
    let mut pcm = vec![0i16; 64];

    eprintln!("ub_child: case={case} impl={which:?}");
    if let Some(rest) = case.strip_prefix("nch_overflow:") {
        let nch: i32 = rest.parse().expect("nch");
        unsafe { f(pcm.as_mut_ptr().add(32), nch, z.as_ptr()) };
    } else {
        match case.as_str() {
            "null_pcm" => unsafe { f(std::ptr::null_mut(), 2, z.as_ptr()) },
            "null_z" => unsafe { f(pcm.as_mut_ptr(), 2, std::ptr::null()) },
            "null_both" => unsafe { f(std::ptr::null_mut(), 2, std::ptr::null()) },
            other => panic!("unknown UB case {other}"),
        }
    }
    // Reached only if the UB happened not to fault.
    eprintln!("ub_child: survived case={case}");
    std::process::exit(0);
}
