//! Phase C — error/rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md` (E1 .. E18). Every test constructs the
//! exact invalid input/condition and asserts that BOTH `.so`s produce the SAME
//! rejection — the same `int` return, the same sentinel double bit pattern, or
//! (for the rows where the C's behaviour is a hard crash) the same manner of
//! death, observed in a child process.
//!
//! Rows whose C behaviour is a crash are NOT quietly skipped: the crash itself
//! is asserted, because that pins the C's real behaviour and would flag a
//! regression if it ever changed.

mod common;
use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// Child-process probe: lets us call inputs on which the C library crashes
// without taking the whole test runner down.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Outcome {
    /// The call returned; payload is the value it printed.
    Returned(String),
    /// The process was killed by this signal (11 = SIGSEGV, 6 = SIGABRT).
    Signal(i32),
    /// The process exited non-zero without a signal.
    ExitCode(i32),
}

impl Outcome {
    fn died(&self) -> bool {
        !matches!(self, Outcome::Returned(_))
    }
}

/// Re-exec this very test binary in "probe" mode.
fn probe(which: &str, func: &str, n: c_int, th: f64, buf_elems: usize) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "zz_probe_child", "--ignored", "--nocapture"])
        .env("PROBE_WHICH", which)
        .env("PROBE_FN", func)
        .env("PROBE_N", n.to_string())
        .env("PROBE_TH", th.to_bits().to_string())
        .env("PROBE_BUF", buf_elems.to_string())
        .output()
        .expect("spawn probe child");

    if let Some(sig) = out.status.signal() {
        return Outcome::Signal(sig);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(line) = stdout.lines().find(|l| l.starts_with("PROBE_RESULT=")) {
        return Outcome::Returned(line["PROBE_RESULT=".len()..].to_string());
    }
    Outcome::ExitCode(out.status.code().unwrap_or(-1))
}

/// The probe body. `#[ignore]`d so a normal `cargo test` run never executes it;
/// `probe()` invokes it explicitly with `--ignored --exact`.
#[test]
#[ignore]
fn zz_probe_child() {
    let which = std::env::var("PROBE_WHICH").unwrap_or_default();
    if which.is_empty() {
        return;
    }
    let func = std::env::var("PROBE_FN").unwrap();
    let n: c_int = std::env::var("PROBE_N").unwrap().parse().unwrap();
    let th = f64::from_bits(std::env::var("PROBE_TH").unwrap().parse::<u64>().unwrap());
    let buf_elems: usize = std::env::var("PROBE_BUF").unwrap().parse().unwrap();

    let p = load();
    let im = match which.as_str() {
        "c" => &p.c,
        "rs" => &p.rs,
        other => panic!("bad PROBE_WHICH {other}"),
    };

    let mut a: Vec<f64> = (0..buf_elems.max(1)).map(|i| 1.0 + i as f64).collect();
    let mut b: Vec<f64> = (0..buf_elems.max(1)).map(|i| 2.0 + i as f64).collect();

    let result = match func.as_str() {
        "match" => {
            let r = unsafe { (im.match_fn)(a.as_mut_ptr(), b.as_mut_ptr(), n, th) };
            format!("int:{r}")
        }
        "match_null" => {
            let r = unsafe { (im.match_fn)(std::ptr::null_mut(), std::ptr::null_mut(), n, th) };
            format!("int:{r}")
        }
        "spectral" => {
            let r = unsafe {
                (im.spectral_fn)(a.as_mut_ptr() as *mut f32, b.as_mut_ptr() as *mut f32, n)
            };
            format!("f64:{:016x}", r.to_bits())
        }
        "spectral_null" => {
            let r = unsafe { (im.spectral_fn)(std::ptr::null_mut(), std::ptr::null_mut(), n) };
            format!("f64:{:016x}", r.to_bits())
        }
        other => panic!("bad PROBE_FN {other}"),
    };
    println!("PROBE_RESULT={result}");
}

// ===========================================================================
// E1 — the energy gate is taken: the library's only explicit `return 0`
// ===========================================================================

#[test]
fn e1_energy_gate_returns_zero() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 8, 16, 17, 33, 64] {
        for _ in 0..50 {
            let r: Vec<f64> = (0..bins).map(|_| rng.range(0.5, 1.0)).collect();
            // total(test) is strictly below 1.0 * total(reference)
            let t: Vec<f64> = r.iter().map(|x| x * 1e-9).collect();
            for th in [1.0, 0.999, 0.5, 1e-6] {
                // Both must return exactly 0, and agree.
                let run = |im: &Impl| {
                    let mut bt = t.clone();
                    let mut br = r.clone();
                    unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, th) }
                };
                let rc = run(&p.c);
                let rr = run(&p.rs);
                assert_eq!(rc, rr, "E1 bins={bins} th={th}: C={rc} Rust={rr}");
                assert_eq!(rc, 0, "E1 expected the gate to reject: bins={bins} th={th}");
            }
        }
    }
}

// ===========================================================================
// E2 — the gate comparison is UNORDERED (NaN) => it must NOT reject
// ===========================================================================

#[test]
fn e2_gate_unordered_does_not_reject() {
    let p = load();
    let bins = 8i32;
    // reference energy hugely larger than test energy: the gate WOULD reject...
    let t = vec![1e-30f64; bins as usize];
    let r = vec![1e30f64; bins as usize];

    // ...but a NaN threshold makes the comparison unordered, so it must not.
    for nan in [
        f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
    ] {
        let run = |im: &Impl| {
            let mut bt = t.clone();
            let mut br = r.clone();
            unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, nan) }
        };
        let rc = run(&p.c);
        let rr = run(&p.rs);
        assert_eq!(rc, rr, "E2 th={:016x}: C={rc} Rust={rr}", nan.to_bits());
        // It fell through the gate, then `contrast >= NaN` is false => 0.
        assert_eq!(rc, 0, "E2 final compare must also be unordered => 0");
    }

    // total(test) itself NaN, with a threshold that would otherwise reject.
    for pos in 0..bins as usize {
        let mut tn = t.clone();
        tn[pos] = f64::NAN;
        let run = |im: &Impl| {
            let mut bt = tn.clone();
            let mut br = r.clone();
            unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, 1.0) }
        };
        assert_eq!(run(&p.c), run(&p.rs), "E2 NaN total at {pos}");
    }
}

// ===========================================================================
// E3 — inf * 0 in the gate => x86 indefinite QNaN => unordered
// ===========================================================================

#[test]
fn e3_gate_inf_times_zero() {
    let p = load();
    for bins in [1i32, 2, 8, 17] {
        let t: Vec<f64> = (0..bins).map(|i| 1.0 + i as f64).collect();
        let zeros = vec![0.0f64; bins as usize];
        let negzeros = vec![-0.0f64; bins as usize];
        for th in [f64::INFINITY, f64::NEG_INFINITY] {
            for r in [&zeros, &negzeros] {
                let run = |im: &Impl| {
                    let mut bt = t.clone();
                    let mut br = r.clone();
                    unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, th) }
                };
                let rc = run(&p.c);
                let rr = run(&p.rs);
                assert_eq!(rc, rr, "E3 bins={bins} th={th}: C={rc} Rust={rr}");
            }
        }
        // and the mirror: total(test)=0, threshold=inf, reference finite
        for th in [f64::INFINITY, f64::NEG_INFINITY] {
            let run = |im: &Impl| {
                let mut bt = zeros.clone();
                let mut br = t.clone();
                unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, th) }
            };
            assert_eq!(run(&p.c), run(&p.rs), "E3' bins={bins} th={th}");
        }
    }
}

// ===========================================================================
// E4 — the final ordered comparison rejects (false or unordered)
// ===========================================================================

#[test]
fn e4_final_comparison_rejects() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 2, 3, 16, 17, 33] {
        for _ in 0..40 {
            let t: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 1.0)).collect();
            let r: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 1.0)).collect();
            // threshold = +inf: gate is `total < inf*total_ref`; make total_ref
            // positive so the product is +inf and the gate DOES reject...
            // so use -inf-safe values: threshold slightly above any possible
            // contrast (contrast <= ~1 for normalized vectors) but negative
            // energy ratio so the gate passes.
            for th in [-1.0e-300, -0.0, 0.0] {
                let run = |im: &Impl| {
                    let mut bt = t.clone();
                    let mut br = r.clone();
                    unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, th) }
                };
                assert_eq!(run(&p.c), run(&p.rs), "E4 bins={bins} th={th}");
            }
        }
    }
    // Explicitly pin the unordered case. NOTE (measured, not assumed): a
    // *constant* spectrum does NOT preprocess to all zeros — `smoothen`'s
    // clamped tail (E18) breaks the constancy, e.g. `[7.5, 7.5]` becomes
    // `[0.9375, 0.46875]`, then `[-0.46875, 0]`, then `[-0.029296875, 0]`, so
    // the magnitude is non-zero and the contrast is finite. An *all-zero*
    // spectrum is what actually reaches the zero-magnitude path, since every
    // stage maps zeros to zeros.
    for bins in [1i32, 2, 3, 16, 17, 33, 64] {
        let z = vec![0.0f64; bins as usize];
        let run = |im: &Impl, th: f64| {
            let mut bt = z.clone();
            let mut br = z.clone();
            unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), bins, th) }
        };
        for th in [-1e300, -1.0, -0.0, 0.0, 0.5, 1.0] {
            let rc = run(&p.c, th);
            let rr = run(&p.rs, th);
            assert_eq!(rc, rr, "E4 NaN-contrast bins={bins} th={th}");
            assert_eq!(
                rc, 0,
                "E4 a NaN contrast must fail `>= {th}` (unordered): bins={bins}"
            );
        }
    }
}

// ===========================================================================
// E5 — match(bins == 0): differentiate's unguarded `v[-1] = 0` overwrites
//      preprocess's return address => SIGSEGV in C.
// ===========================================================================

#[test]
fn e5_match_bins_zero_crashes_in_c() {
    // Every threshold class, since the gate can never fire first.
    for th in [0.0f64, 0.5, 1.0, -1.0, f64::INFINITY, f64::NAN] {
        let c = probe("c", "match", 0, th, 4);
        assert_eq!(
            c,
            Outcome::Signal(11),
            "E5: C match(bins=0, th={th}) must die with SIGSEGV (the `v[-1] = 0` \
             store lands on preprocess's return address); got {c:?}"
        );
        // The Rust translation deliberately guards that out-of-bounds store
        // (`if length >= 1`) instead of corrupting its caller's stack, so it
        // returns instead of crashing. There is no C value to match here: the
        // C's behaviour is undefined and observably fatal.
        let r = probe("rs", "match", 0, th, 4);
        assert!(
            !r.died(),
            "E5: Rust must not crash on bins=0 (it guards the OOB store); got {r:?}"
        );
    }
    // Same with NULL pointers (E14): still fatal in C, for the same reason —
    // the crash has nothing to do with the pointers.
    let c = probe("c", "match_null", 0, 0.5, 0);
    assert_eq!(c, Outcome::Signal(11), "E5/E14: C match(NULL,NULL,0) => SIGSEGV; got {c:?}");
}

// ===========================================================================
// E6 — spectral_contrast(length == 0) => +0.0, buffers untouched
// ===========================================================================

#[test]
fn e6_spectral_length_zero() {
    let p = load();
    let mut rng = Rng::new();
    for _ in 0..100 {
        let a: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
        let b: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
        diff_spectral(&p, "E6", &a, &b, 0, Alias::Disjoint);
    }
    // Pin the actual sentinel, not merely "they agree".
    let mut a = Buf::f32_len(8);
    let mut b = Buf::f32_len(8);
    for im in [&p.c, &p.rs] {
        let r = unsafe { (im.spectral_fn)(a.as_f32_ptr(), b.as_f32_ptr(), 0) };
        assert_eq!(
            r.to_bits(),
            0u64,
            "E6 {}: spectral_contrast(_,_,0) must be +0.0, got {:016x}",
            im.name,
            r.to_bits()
        );
    }
}

// ===========================================================================
// E7 — spectral_contrast(length < 0) => +0.0, buffers untouched, no crash
// ===========================================================================

#[test]
fn e7_spectral_negative_length() {
    let p = load();
    let mut rng = Rng::new();
    for len in [-1i32, -2, -3, -16, -17, -1000, -65536, i32::MIN, i32::MIN + 1] {
        for _ in 0..20 {
            let a: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
            let b: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
            diff_spectral(&p, &format!("E7 len={len}"), &a, &b, len, Alias::Disjoint);
        }
        let mut a = Buf::f32_len(8);
        let mut b = Buf::f32_len(8);
        for im in [&p.c, &p.rs] {
            let r = unsafe { (im.spectral_fn)(a.as_f32_ptr(), b.as_f32_ptr(), len) };
            assert_eq!(
                r.to_bits(),
                0u64,
                "E7 {} len={len}: must be +0.0, got {:016x}",
                im.name,
                r.to_bits()
            );
        }
    }
}

// ===========================================================================
// E8 — match(bins < 0): preprocess's `memcpy(v, source, length*sizeof(*v))`
//      converts a negative int to size_t => ~2^64 byte copy => SIGSEGV in C.
// ===========================================================================

#[test]
fn e8_match_negative_bins_crashes_in_c() {
    for bins in [-1i32, -2, -7, -1000] {
        let c = probe("c", "match", bins, 0.5, 8);
        assert_eq!(
            c,
            Outcome::Signal(11),
            "E8: C match(bins={bins}) must die with SIGSEGV (memcpy of \
             (size_t)(bins * 8) bytes); got {c:?}"
        );
        let r = probe("rs", "match", bins, 0.5, 8);
        assert!(
            !r.died(),
            "E8: Rust must not crash on bins={bins}; got {r:?}"
        );
    }
}

// ===========================================================================
// E9 — zero magnitude: the unguarded `v[i] /= magnitude`
// ===========================================================================

#[test]
fn e9_zero_magnitude_division() {
    let p = load();
    let mut rng = Rng::new();

    // (a) directly through spectral_contrast: an all-zero vector
    for len in [1usize, 2, 3, 15, 16, 17, 33, 64] {
        let zeros = vec![0.0f32; len];
        let negz = vec![-0.0f32; len];
        let finite: Vec<f32> = (0..len).map(|_| rng.range(0.5, 2.0) as f32).collect();
        for (an, a) in [("+0", &zeros), ("-0", &negz)] {
            diff_spectral(&p, &format!("E9a {an}/{an} len={len}"), a, a, len as c_int, Alias::Disjoint);
            diff_spectral(&p, &format!("E9a {an}/fin len={len}"), a, &finite, len as c_int, Alias::Disjoint);
            diff_spectral(&p, &format!("E9a fin/{an} len={len}"), &finite, a, len as c_int, Alias::Disjoint);
        }
        // Pin the sentinel: 0/0 gives the x86 indefinite QNaN in f32.
        let mut ba = Buf::from_f32(&zeros);
        let mut bb = Buf::from_f32(&zeros);
        for im in [&p.c, &p.rs] {
            let mut a2 = ba.clone();
            let mut b2 = bb.clone();
            let r = unsafe { (im.spectral_fn)(a2.as_f32_ptr(), b2.as_f32_ptr(), len as c_int) };
            assert!(
                r.is_nan(),
                "E9 {} len={len}: 0/0 must poison the result to NaN, got {r}",
                im.name
            );
            assert_eq!(
                a2.as_f32(len)[0].to_bits(),
                0xFFC0_0000,
                "E9 {} len={len}: 0.0/0.0 must store the f32 indefinite QNaN",
                im.name
            );
        }
        let _ = (&mut ba, &mut bb);
    }

    // (b) through the whole match pipeline: constant spectra differentiate to
    //     all zeros, so `normalize` always divides by zero.
    for bins in [2i32, 3, 15, 16, 17, 33, 64, 128] {
        for _ in 0..20 {
            let c1 = rng.range(-100.0, 100.0);
            let c2 = rng.range(-100.0, 100.0);
            let t = vec![c1; bins as usize];
            let r = vec![c2; bins as usize];
            for th in [-1.0, -0.0, 0.0, 0.5, 1.0] {
                diff_match(&p, &format!("E9b bins={bins} th={th}"), &t, &r, bins, th, false);
            }
        }
    }

    // (c) non-zero elements whose squares underflow to zero => magnitude 0
    for len in [1usize, 2, 8, 16, 17] {
        let sub: Vec<f32> = (0..len)
            .map(|_| f32::from_bits((rng.next_u32() & 0x007F_FFFF) | 1))
            .collect();
        diff_spectral(&p, &format!("E9c len={len}"), &sub, &sub, len as c_int, Alias::Disjoint);
    }
}

// ===========================================================================
// E10 — sqrt of NaN / propagation out of dot_product
// ===========================================================================

#[test]
fn e10_sqrt_of_nan_magnitude() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1usize, 2, 3, 8, 16, 17, 33] {
        for _ in 0..40 {
            // +inf and -inf in the same vector => inf + (-inf) inside
            // dot_product... but a self dot product squares, so build the NaN
            // directly instead, and also exercise inf**2 = inf => sqrt(inf).
            let mut a: Vec<f32> = (0..len).map(|_| rng.range(-2.0, 2.0) as f32).collect();
            a[rng.below(len)] = f32::NAN;
            let mut b: Vec<f32> = (0..len).map(|_| rng.range(-2.0, 2.0) as f32).collect();
            b[rng.below(len)] = f32::INFINITY;
            diff_spectral(&p, &format!("E10 len={len}"), &a, &b, len as c_int, Alias::Disjoint);

            let inf: Vec<f32> = vec![f32::INFINITY; len];
            diff_spectral(&p, &format!("E10 inf len={len}"), &inf, &inf, len as c_int, Alias::Disjoint);
            let mixed: Vec<f32> = (0..len)
                .map(|i| if i % 2 == 0 { f32::INFINITY } else { f32::NEG_INFINITY })
                .collect();
            diff_spectral(&p, &format!("E10 mix len={len}"), &mixed, &inf, len as c_int, Alias::Disjoint);
        }
    }
}

// ===========================================================================
// E11 — signalling NaN inputs (quieting through mulss / cvtss2sd / divsd)
// ===========================================================================

#[test]
fn e11_signalling_nan_inputs() {
    let p = load();
    let mut rng = Rng::new();
    let snans: Vec<u32> = vec![
        0x7F80_0001, 0x7FBF_FFFF, 0xFF80_0001, 0xFFBF_FFFF, 0x7F80_0002, 0xFFA0_0001,
    ];
    for len in [1usize, 2, 3, 8, 16, 17, 33] {
        for _ in 0..40 {
            let a: Vec<f32> = (0..len)
                .map(|_| f32::from_bits(snans[rng.below(snans.len())]))
                .collect();
            let b: Vec<f32> = (0..len)
                .map(|_| {
                    if rng.below(2) == 0 {
                        f32::from_bits(snans[rng.below(snans.len())])
                    } else {
                        rng.range(-2.0, 2.0) as f32
                    }
                })
                .collect();
            diff_spectral(&p, &format!("E11 len={len}"), &a, &b, len as c_int, Alias::Disjoint);
        }
    }
    // sNaN doubles fed to `match` (they also decode to odd f32 halves).
    for bins in [1i32, 2, 3, 16, 17, 33] {
        for &bits in &[
            0x7FF0_0000_0000_0001u64,
            0xFFF0_0000_0000_0001,
            0x7FF7_FFFF_FFFF_FFFF,
        ] {
            let t = vec![f64::from_bits(bits); bins as usize];
            let r: Vec<f64> = (0..bins).map(|i| 1.0 + i as f64).collect();
            for th in [-1.0, 0.0, 0.5, 1.0] {
                diff_match(&p, &format!("E11 match bins={bins} th={th}"), &t, &r, bins, th, false);
                diff_match(&p, &format!("E11 match' bins={bins} th={th}"), &r, &t, bins, th, false);
            }
        }
    }
}

// ===========================================================================
// E12 — fully aliased arguments (a == b), permitted: no `restrict`
// ===========================================================================

#[test]
fn e12_fully_aliased_arguments() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1usize, 2, 3, 15, 16, 17, 33, 64] {
        for _ in 0..40 {
            let a: Vec<f32> = (0..len).map(|_| rng.range(-4.0, 4.0) as f32).collect();
            diff_spectral(&p, &format!("E12 len={len}"), &a, &a, len as c_int, Alias::Same);
            let raw: Vec<f32> = (0..len).map(|_| rng.raw_f32()).collect();
            diff_spectral(&p, &format!("E12 raw len={len}"), &raw, &raw, len as c_int, Alias::Same);
        }
    }
    // and `match(v, v, bins, th)`
    for bins in [1i32, 2, 3, 16, 17, 33] {
        for _ in 0..40 {
            let t: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 1.0)).collect();
            for th in [-1.0, 0.0, 0.5, 1.0, 2.0] {
                diff_match(&p, &format!("E12 match bins={bins} th={th}"), &t, &t, bins, th, true);
            }
        }
    }
}

// ===========================================================================
// E13 — partially overlapping arguments
// ===========================================================================

#[test]
fn e13_partially_overlapping_arguments() {
    let p = load();
    let mut rng = Rng::new();
    for len in [1usize, 2, 3, 8, 16, 17, 33] {
        for k in [1usize, 2, 3, 5] {
            for _ in 0..20 {
                let a: Vec<f32> = (0..len + k).map(|_| rng.range(-4.0, 4.0) as f32).collect();
                diff_spectral(&p, &format!("E13 B len={len} k={k}"), &a, &a, len as c_int, Alias::OffsetB(k));
                diff_spectral(&p, &format!("E13 A len={len} k={k}"), &a, &a, len as c_int, Alias::OffsetA(k));
                let raw: Vec<f32> = (0..len + k).map(|_| rng.raw_f32()).collect();
                diff_spectral(&p, &format!("E13 Braw len={len} k={k}"), &raw, &raw, len as c_int, Alias::OffsetB(k));
                diff_spectral(&p, &format!("E13 Araw len={len} k={k}"), &raw, &raw, len as c_int, Alias::OffsetA(k));
            }
        }
    }
}

// ===========================================================================
// E14 — NULL pointers
// ===========================================================================

#[test]
fn e14_null_pointers() {
    let p = load();
    // spectral_contrast never dereferences when length <= 0.
    for len in [0i32, -1, -1000, i32::MIN] {
        let rc = unsafe { (p.c.spectral_fn)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        let rr = unsafe { (p.rs.spectral_fn)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        assert_eq!(
            rc.to_bits(),
            rr.to_bits(),
            "E14 spectral(NULL,NULL,{len}): C={:016x} Rust={:016x}",
            rc.to_bits(),
            rr.to_bits()
        );
        assert_eq!(rc.to_bits(), 0, "E14 spectral(NULL,NULL,{len}) must be +0.0");
    }
    // `match` with NULL: bins <= 0 still dies in C (E5/E8) for reasons
    // unrelated to the pointers; bins >= 1 dereferences NULL in `total`.
    for (bins, expect) in [(0i32, 11), (-1i32, 11), (1i32, 11), (16i32, 11)] {
        let c = probe("c", "match_null", bins, 0.5, 0);
        assert_eq!(
            c,
            Outcome::Signal(expect),
            "E14 C match(NULL,NULL,{bins}) expected SIGSEGV, got {c:?}"
        );
    }
    // Rust must reject NULL with bins >= 1 the same fatal way (it dereferences
    // the same null pointer in `total`), so this pair really does agree.
    for bins in [1i32, 16] {
        let c = probe("c", "match_null", bins, 0.5, 0);
        let r = probe("rs", "match_null", bins, 0.5, 0);
        assert_eq!(
            c, r,
            "E14 match(NULL,NULL,{bins}): C={c:?} Rust={r:?} must agree"
        );
    }
    for bins in [1i32, 16, 1000] {
        let c = probe("c", "spectral_null", bins, 0.5, 0);
        let r = probe("rs", "spectral_null", bins, 0.5, 0);
        assert_eq!(
            c, r,
            "E14 spectral(NULL,NULL,{bins}): C={c:?} Rust={r:?} must agree"
        );
        assert_eq!(c, Outcome::Signal(11), "E14 spectral NULL len>0 => SIGSEGV");
    }
}

// ===========================================================================
// E15 — out-of-range "enum"/int values across the FFI boundary.
//
// The API declares no `enum`, so the exhaustive integer surface is `int bins` /
// `int length`. Every value is accepted without validation; these are the
// boundaries.
// ===========================================================================

#[test]
fn e15_integer_boundaries() {
    let p = load();
    let mut rng = Rng::new();

    // spectral_contrast: the whole non-positive range is safe and must agree.
    for len in [i32::MIN, i32::MIN + 1, -2, -1, 0] {
        let a: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
        let b: Vec<f32> = (0..8).map(|_| rng.raw_f32()).collect();
        diff_spectral(&p, &format!("E15 spectral len={len}"), &a, &b, len, Alias::Disjoint);
    }
    // ...and 1 is the first value that does any work.
    for _ in 0..50 {
        let a: Vec<f32> = (0..1).map(|_| rng.raw_f32()).collect();
        let b: Vec<f32> = (0..1).map(|_| rng.raw_f32()).collect();
        diff_spectral(&p, "E15 spectral len=1", &a, &b, 1, Alias::Disjoint);
    }

    // spectral_contrast with a length far past the buffer: both must fault.
    let c = probe("c", "spectral", i32::MAX, 0.0, 8);
    let r = probe("rs", "spectral", i32::MAX, 0.0, 8);
    assert!(c.died(), "E15 C spectral(len=INT_MAX) must fault, got {c:?}");
    assert_eq!(
        c, r,
        "E15 spectral(len=INT_MAX): C={c:?} Rust={r:?} must agree"
    );

    // match: bins == 1 is the smallest working value and must agree exactly.
    for _ in 0..50 {
        let t: Vec<f64> = (0..1).map(|_| rng.raw_f64()).collect();
        let rr: Vec<f64> = (0..1).map(|_| rng.raw_f64()).collect();
        diff_match(&p, "E15 match bins=1", &t, &rr, 1, rng.range(-2.0, 2.0), false);
    }

    // match: bins <= 0 is fatal in C (E5/E8), asserted there.
    for bins in [i32::MIN, i32::MIN + 1, -1, 0] {
        let c = probe("c", "match", bins, 0.5, 8);
        assert_eq!(
            c,
            Outcome::Signal(11),
            "E15 C match(bins={bins}) must die with SIGSEGV, got {c:?}"
        );
    }

    // match: a `bins` big enough to exhaust the stack via the two VLAs is fatal
    // in C. The Rust puts those buffers on the heap, so it survives; that is a
    // deliberate, documented difference on an input the C cannot handle.
    let big = 4_000_000i32; // 2 x 32 MB of VLA, well past the 8 MB stack limit
    let c = probe("c", "match", big, 0.5, big as usize);
    assert!(
        c.died(),
        "E15 C match(bins={big}) must exhaust the stack and die, got {c:?}"
    );
}

// ===========================================================================
// E16 — match(bins == 1): differentiate zeroes the only element
// ===========================================================================

#[test]
fn e16_match_bins_one_zeroed_by_differentiate() {
    let p = load();
    let mut rng = Rng::new();
    for _ in 0..200 {
        let t = vec![rng.raw_f64()];
        let r = vec![rng.raw_f64()];
        for th in threshold_classes() {
            diff_match(&p, "E16", &t, &r, 1, th, false);
        }
    }
    // Pin the outcome: after `differentiate`, both buffers are all-zero, so the
    // contrast is NaN and `>= threshold` is false => 0 (unless the gate already
    // returned 0). Either way the answer is 0 for every threshold.
    for _ in 0..50 {
        let t = vec![rng.range(0.0, 10.0)];
        let r = vec![rng.range(0.0, 10.0)];
        for th in [-1e300, -1.0, -0.0, 0.0, 0.5, 1.0, 1e300] {
            let run = |im: &Impl| {
                let mut bt = t.clone();
                let mut br = r.clone();
                unsafe { (im.match_fn)(bt.as_mut_ptr(), br.as_mut_ptr(), 1, th) }
            };
            let rc = run(&p.c);
            assert_eq!(rc, run(&p.rs), "E16 th={th}");
            assert_eq!(rc, 0, "E16 bins=1 always rejects (NaN contrast), th={th}");
        }
    }
}

// ===========================================================================
// E17 — odd `bins`: the f32 view reads the low half of one extra double
// ===========================================================================

#[test]
fn e17_odd_bins_f32_view_boundary() {
    let p = load();
    let mut rng = Rng::new();
    for bins in [1i32, 3, 5, 7, 9, 15, 17, 33, 65, 127] {
        for _ in 0..40 {
            let t: Vec<f64> = (0..bins).map(|_| rng.raw_f64()).collect();
            let r: Vec<f64> = (0..bins).map(|_| rng.raw_f64()).collect();
            diff_match(&p, &format!("E17 bins={bins}"), &t, &r, bins, rng.range(-2.0, 2.0), false);
        }
    }
    // The same boundary through the exported low-level entry point: an odd
    // `length` reads exactly `length` f32 slots, never the padding one.
    for len in [1i32, 3, 5, 7, 15, 17, 33, 65] {
        for _ in 0..40 {
            let a: Vec<f32> = (0..len).map(|_| rng.raw_f32()).collect();
            let b: Vec<f32> = (0..len).map(|_| rng.raw_f32()).collect();
            diff_spectral(&p, &format!("E17 spectral len={len}"), &a, &b, len, Alias::Disjoint);
        }
    }
}

// ===========================================================================
// E18 — smoothen's clamped window with an unchanged N_SMOOTH divisor
// ===========================================================================

#[test]
fn e18_smoothen_attenuated_tail() {
    let p = load();
    let mut rng = Rng::new();
    // For length < 16 EVERY output is attenuated; at exactly 16 only element 0
    // gets a full window; above 16 there is a growing full-window region.
    for bins in 1..=40i32 {
        for _ in 0..20 {
            let t: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 1.0)).collect();
            let r: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 1.0)).collect();
            for th in [-1.0, 0.0, 0.5, 1.0] {
                diff_match(&p, &format!("E18 bins={bins} th={th}"), &t, &r, bins, th, false);
            }
        }
        // A single spike, so the attenuation profile is directly visible in the
        // preprocessed data and hence in the contrast.
        for pos in 0..(bins as usize).min(6) {
            let mut t = vec![0.0f64; bins as usize];
            t[pos] = 1.0;
            let mut r = vec![0.0f64; bins as usize];
            r[(pos + 1) % bins as usize] = 1.0;
            diff_match(&p, &format!("E18 spike bins={bins} pos={pos}"), &t, &r, bins, -1.0, false);
        }
    }
}
