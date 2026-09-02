//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row.
//!
//! `normalize` returns `void`, so there is no error code to compare. Each row
//! is therefore asserted on the observable effect:
//!
//! * non-trapping rows — the exact bits of the whole scratch buffer after the
//!   call, for BOTH `.so`s, plus an assertion on what the C is *documented* to
//!   do (zero-filled / left untouched), so a "both wrong the same way" pass is
//!   not possible for the documented cases;
//! * trapping rows — the exact termination signal of a forked child that makes
//!   the call, for BOTH `.so`s.

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// A buffer of `n` distinct sentinels, so "nothing was written" is observable.
fn painted(n: usize) -> Vec<f32> {
    (0..n).map(sentinel).collect()
}

/// Assert C and Rust behave identically for a call in which one or both
/// pointers are NULL and the call is expected NOT to trap.
fn diff_null_call(what: &str, dest: Option<&mut [f32]>, src_null: bool, size: i32) {
    let l = libs();
    let (cf, rf) = (l.c(), l.rust());

    match dest {
        None => {
            // dest == NULL as well; nothing to compare but "returned normally".
            assert!(src_null, "{what}: dest==NULL cases here always pass src==NULL");
            unsafe { cf(std::ptr::null_mut(), std::ptr::null(), size) };
            unsafe { rf(std::ptr::null_mut(), std::ptr::null(), size) };
        }
        Some(base) => {
            let mut bc = base.to_vec();
            let mut br = base.to_vec();
            let sc: *const f32 = if src_null { std::ptr::null() } else { bc.as_ptr() };
            let sr: *const f32 = if src_null { std::ptr::null() } else { br.as_ptr() };
            unsafe { cf(bc.as_mut_ptr(), sc, size) };
            unsafe { rf(br.as_mut_ptr(), sr, size) };
            assert_eq!(bits(&bc), bits(&br), "{what}: C and Rust disagree");
            assert_eq!(
                bits(&bc),
                bits(base),
                "{what}: expected nothing to be written, but the buffer changed"
            );
        }
    }
}

/// Name of the hidden worker test that performs one trapping call.
const WORKER: &str = "zz_crash_worker";

/// Re-exec this test binary so that it makes exactly one trapping call, and
/// report `(exit code, terminating signal)`.
fn run_crash_child(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", WORKER, "--test-threads=1", "--nocapture"])
        .env("CRASH_CASE", case)
        .env("CRASH_IMPL", which)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn crash worker");
    (status.code(), status.signal())
}

/// Differential trap test: both implementations must die the same way.
fn diff_crash(row: &str, case: &str) {
    let c = run_crash_child(case, "c");
    let r = run_crash_child(case, "rust");
    assert_eq!(
        c, r,
        "[{row}/{case}] C and Rust terminated differently: C=(code {:?}, signal {:?}) \
         Rust=(code {:?}, signal {:?})",
        c.0, c.1, r.0, r.1
    );
    assert!(
        c.1.is_some(),
        "[{row}/{case}] expected both to be killed by a signal, but the child exited \
         normally with code {:?} (the trapping condition did not reproduce)",
        c.0
    );
    // On x86-64 Linux all of these rows fault on an unmapped page.
    assert_eq!(c.1, Some(11 /* SIGSEGV */), "[{row}/{case}] unexpected signal {:?}", c.1);
}

/// The trapping call itself, executed only in the re-exec'd child.
#[test]
fn zz_crash_worker() {
    let Ok(case) = std::env::var("CRASH_CASE") else {
        // Normal test run: nothing to do. The parent tests drive this via
        // CRASH_CASE / CRASH_IMPL.
        return;
    };
    let which = std::env::var("CRASH_IMPL").expect("CRASH_IMPL");
    let l = libs();
    let f = match which.as_str() {
        "c" => l.c(),
        "rust" => l.rust(),
        other => panic!("bad CRASH_IMPL {other}"),
    };

    // Keep buffers alive for the whole call.
    let mut dest = vec![0.0f32; 64];
    let src_nonzero = vec![3.5f32; 64];
    let src_zero = vec![0.0f32; 64];
    let mut inplace = vec![1.25f32; 64];

    unsafe {
        match case.as_str() {
            // E4: negative size, dest != src -> memset of ~2^64 bytes.
            "e4" => f(dest.as_mut_ptr(), src_nonzero.as_ptr(), -1),
            // E6: INT_MIN size, dest != src -> memset of 0xFFFFFFFE00000000 bytes.
            "e6" => f(dest.as_mut_ptr(), src_nonzero.as_ptr(), i32::MIN),
            // E8: src == NULL with size > 0 -> read fault.
            "e8" => f(dest.as_mut_ptr(), std::ptr::null(), 16),
            // E11: dest == NULL, sum > 0 -> write fault in the scaling loop.
            "e11" => f(std::ptr::null_mut(), src_nonzero.as_ptr(), 16),
            // E12: dest == NULL, sum == 0 -> fault inside memset.
            "e12" => f(std::ptr::null_mut(), src_zero.as_ptr(), 16),
            // E13: both NULL, size > 0 -> read fault.
            "e13" => f(std::ptr::null_mut(), std::ptr::null(), 16),
            // E23: size == INT_MAX -> the accumulation loop runs off the buffer.
            //      Done in-place so no memset can mask the read fault.
            "e23" => f(inplace.as_mut_ptr(), inplace.as_ptr(), i32::MAX),
            other => panic!("unknown CRASH_CASE {other}"),
        }
    }

    // Reaching here means the call did not trap; make that visible to the
    // parent as a distinct, non-signal exit.
    println!("NOCRASH");
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// E1 / E2 — size == 0
// ---------------------------------------------------------------------------

#[test]
fn err_e1_size0_disjoint() {
    // dest != src, size == 0 -> memset(dest, 0, 0): nothing written.
    let n = 0usize;
    let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
    paint_untouched(&mut buf, &[]);
    let out = diff_expect("E1", &buf, dest_off, src_off, 0);
    assert_eq!(bits(&out), bits(&buf), "E1: memset(_,_,0) must write nothing");
}

#[test]
fn err_e2_size0_inplace() {
    let (mut buf, off, _) = layout(0, Alias::InPlace);
    paint_untouched(&mut buf, &[]);
    let out = diff_expect("E2", &buf, off, off, 0);
    assert_eq!(bits(&out), bits(&buf), "E2: nothing may be written");
}

// ---------------------------------------------------------------------------
// E3 / E5 — negative size with dest == src (safe: memset is not reached)
// ---------------------------------------------------------------------------

#[test]
fn err_e3_negative_size_inplace() {
    let base = painted(64);
    for size in [-1i32, -2, -7, -64, -1000, -0x4000_0000] {
        let out = diff_expect(&format!("E3(size={size})"), &base, 0, 0, size);
        assert_eq!(
            bits(&out),
            bits(&base),
            "E3(size={size}): loops are skipped and dest == src, so nothing may be written"
        );
    }
}

#[test]
fn err_e5_int_min_inplace() {
    let base = painted(64);
    let out = diff_expect("E5", &base, 0, 0, i32::MIN);
    assert_eq!(bits(&out), bits(&base), "E5: nothing may be written");
}

// ---------------------------------------------------------------------------
// E4 / E6 — negative size with dest != src: huge memset
// ---------------------------------------------------------------------------

#[test]
fn err_e4_negative_size_disjoint_crash() {
    diff_crash("E4", "e4");
}

#[test]
fn err_e6_int_min_disjoint_crash() {
    diff_crash("E6", "e6");
}

// ---------------------------------------------------------------------------
// E7 / E9 / E10 — NULL pointers that are NOT dereferenced
// ---------------------------------------------------------------------------

#[test]
fn err_e7_null_src_size0() {
    let mut base = painted(32);
    diff_null_call("E7", Some(&mut base), true, 0);
}

#[test]
fn err_e9_both_null_size0() {
    diff_null_call("E9", None, true, 0);
}

#[test]
fn err_e10_both_null_negative_size() {
    for size in [-1i32, -8, i32::MIN] {
        diff_null_call("E10", None, true, size);
    }
}

// ---------------------------------------------------------------------------
// E8 / E11 / E12 / E13 — NULL pointers that ARE dereferenced
// ---------------------------------------------------------------------------

#[test]
fn err_e8_null_src_positive_size_crash() {
    diff_crash("E8", "e8");
}

#[test]
fn err_e11_null_dest_sum_positive_crash() {
    diff_crash("E11", "e11");
}

#[test]
fn err_e12_null_dest_sum_zero_crash() {
    diff_crash("E12", "e12");
}

#[test]
fn err_e13_both_null_positive_size_crash() {
    diff_crash("E13", "e13");
}

// ---------------------------------------------------------------------------
// E14 / E15 — NaN input is rejected by `sum > 0.0f`
// ---------------------------------------------------------------------------

#[test]
fn err_e14_nan_rejected_to_zero() {
    let mut rng = Rng::new(SEED ^ 0xE14);
    for _ in 0..500 {
        let n = pick_size(&mut rng) as usize;
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[src_off + i] = rng.unit();
        }
        // Plant at least one NaN at a random index, with a random payload.
        let k = rng.below(n);
        let payload = (rng.next_u32() & 0x007F_FFFF).max(1);
        let sign = (rng.next_u32() & 1) << 31;
        buf[src_off + k] = f32::from_bits(sign | 0x7F80_0000 | payload);

        let out = diff_expect("E14", &buf, dest_off, src_off, n as i32);
        // C result: sum is NaN, `NaN > 0.0f` is false, dest != src -> zero-fill.
        for i in 0..n {
            assert_eq!(
                out[dest_off + i].to_bits(),
                0x0000_0000,
                "E14: dest[{i}] must be +0.0 (NaN input takes the zero-fill branch)"
            );
        }
    }
}

#[test]
fn err_e15_nan_inplace_untouched() {
    let mut rng = Rng::new(SEED ^ 0xE15);
    for _ in 0..500 {
        let n = pick_size(&mut rng) as usize;
        let (mut buf, off, _) = layout(n, Alias::InPlace);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[off + i] = rng.unit();
        }
        let k = rng.below(n);
        let payload = (rng.next_u32() & 0x007F_FFFF).max(1);
        let nan_bits = 0x7F80_0000 | payload; // positive quiet/signalling NaN
        buf[off + k] = f32::from_bits(nan_bits);

        let out = diff_expect("E15", &buf, off, off, n as i32);
        assert_eq!(
            bits(&out),
            bits(&buf),
            "E15: dest == src so no branch writes; the NaN payload must survive"
        );
        assert_eq!(out[off + k].to_bits(), nan_bits, "E15: exact NaN bits preserved");
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — all-zero input
// ---------------------------------------------------------------------------

#[test]
fn err_e16_all_zero_disjoint() {
    let mut rng = Rng::new(SEED ^ 0xE16);
    for _ in 0..500 {
        let n = pick_size(&mut rng) as usize;
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[src_off + i] = if rng.next_u64() & 1 == 0 { 0.0 } else { -0.0 };
        }
        let out = diff_expect("E16", &buf, dest_off, src_off, n as i32);
        for i in 0..n {
            assert_eq!(
                out[dest_off + i].to_bits(),
                0x0000_0000,
                "E16: zero-fill yields +0.0 even where src[{i}] was -0.0"
            );
        }
        // src must be untouched (disjoint).
        for i in 0..n {
            assert_eq!(out[src_off + i].to_bits(), buf[src_off + i].to_bits(), "E16: src changed");
        }
    }
}

#[test]
fn err_e17_all_zero_inplace() {
    let mut rng = Rng::new(SEED ^ 0xE17);
    for _ in 0..500 {
        let n = (pick_size(&mut rng) as usize).max(1);
        let (mut buf, off, _) = layout(n, Alias::InPlace);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[off + i] = if rng.next_u64() & 1 == 0 { 0.0 } else { -0.0 };
        }
        // Guarantee at least one -0.0 so the assertion has teeth.
        buf[off] = -0.0;
        let out = diff_expect("E17", &buf, off, off, n as i32);
        assert_eq!(bits(&out), bits(&buf), "E17: dest == src, so nothing is written");
        assert_eq!(
            out[off].to_bits(),
            0x8000_0000,
            "E17: -0.0 must NOT be normalised to +0.0 in the in-place case"
        );
    }
}

// ---------------------------------------------------------------------------
// E18 / E19 — non-zero input whose squares underflow to +0.0
// ---------------------------------------------------------------------------

fn underflowing(rng: &mut Rng) -> f32 {
    // |x| < 2^-75  =>  x*x rounds to +0.0 in f32 (min subnormal is 2^-149).
    let v = gen_elem(rng, Pop::TinyUnderflow);
    assert_eq!(v * v, 0.0, "test bug: {v:e} squared is not 0");
    assert_ne!(v, 0.0, "test bug: element must be non-zero");
    v
}

#[test]
fn err_e18_underflow_to_zero_disjoint() {
    let mut rng = Rng::new(SEED ^ 0xE18);
    for _ in 0..500 {
        let n = (pick_size(&mut rng) as usize).max(1);
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[src_off + i] = underflowing(&mut rng);
        }
        let out = diff_expect("E18", &buf, dest_off, src_off, n as i32);
        for i in 0..n {
            assert_eq!(
                out[dest_off + i].to_bits(),
                0x0000_0000,
                "E18: sum underflows to +0.0, so dest is zero-filled despite non-zero src"
            );
        }
    }
}

#[test]
fn err_e19_underflow_to_zero_inplace() {
    let mut rng = Rng::new(SEED ^ 0xE19);
    for _ in 0..500 {
        let n = (pick_size(&mut rng) as usize).max(1);
        let (mut buf, off, _) = layout(n, Alias::InPlace);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[off + i] = underflowing(&mut rng);
        }
        let out = diff_expect("E19", &buf, off, off, n as i32);
        assert_eq!(bits(&out), bits(&buf), "E19: nothing written; the tiny values survive");
    }
}

// ---------------------------------------------------------------------------
// E20 / E21 — sum overflows to +inf
// ---------------------------------------------------------------------------

#[test]
fn err_e20_sum_overflow_to_inf() {
    let mut rng = Rng::new(SEED ^ 0xE20);
    for _ in 0..500 {
        let n = (pick_size(&mut rng) as usize).max(2);
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            // |x| >= 2^70 with n >= 2 guarantees the f32 accumulation overflows.
            buf[src_off + i] = rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(120);
        }
        let out = diff_expect("E20", &buf, dest_off, src_off, n as i32);
        for i in 0..n {
            let want = if buf[src_off + i].is_sign_negative() { 0x8000_0000 } else { 0x0000_0000 };
            assert_eq!(
                out[dest_off + i].to_bits(),
                want,
                "E20: scale is 1/sqrt(+inf) == +0.0, so dest[{i}] must be signed zero, \
                 not a unit vector component"
            );
        }
    }
}

#[test]
fn err_e21_inf_input_produces_nan() {
    let mut rng = Rng::new(SEED ^ 0xE21);
    for _ in 0..500 {
        let n = (pick_size(&mut rng) as usize).max(1);
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        for i in 0..n {
            buf[src_off + i] = rng.unit();
        }
        let k = rng.below(n);
        buf[src_off + k] = rng.sign() * f32::INFINITY;

        let out = diff_expect("E21", &buf, dest_off, src_off, n as i32);
        for i in 0..n {
            let s = buf[src_off + i];
            let d = out[dest_off + i];
            if s.is_infinite() {
                assert!(d.is_nan(), "E21: inf * 0.0 must be NaN, got {:#010x}", d.to_bits());
            } else {
                let want = if s.is_sign_negative() { 0x8000_0000 } else { 0x0000_0000 };
                assert_eq!(d.to_bits(), want, "E21: finite * 0.0 must be signed zero");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E22 — sum lands in the denormal range
// ---------------------------------------------------------------------------

#[test]
fn err_e22_denormal_sum() {
    let mut rng = Rng::new(SEED ^ 0xE22);
    let mut saw_denormal_sum = 0usize;
    for _ in 0..1000 {
        let n = [1usize, 2, 3, 4][rng.below(4)];
        let (mut buf, dest_off, src_off) = layout(n, Alias::Disjoint);
        paint_untouched(&mut buf, &[]);
        let mut sum = 0.0f32;
        for i in 0..n {
            let v = gen_elem(&mut rng, Pop::DenormalSum);
            buf[src_off + i] = v;
            sum += v * v;
        }
        if sum > 0.0 && sum < f32::MIN_POSITIVE {
            saw_denormal_sum += 1;
        }
        // Bit-exact agreement is the assertion; the value itself is whatever
        // 1.0f/sqrtf(denormal) happens to produce.
        diff_expect("E22", &buf, dest_off, src_off, n as i32);
    }
    assert!(
        saw_denormal_sum > 50,
        "E22: only {saw_denormal_sum} iterations actually produced a denormal sum — \
         the generator no longer exercises this row"
    );
}

// ---------------------------------------------------------------------------
// E23 — size == INT_MAX
// ---------------------------------------------------------------------------

#[test]
fn err_e23_int_max_size_crash() {
    diff_crash("E23", "e23");
}

// ---------------------------------------------------------------------------
// E24 — there is no enum / flag surface to pass an out-of-range value through
// ---------------------------------------------------------------------------

/// Mechanically re-verify the claim in `ERRORS.md` row E24: the public API has
/// no enum, flag or mode parameter, so "out-of-range enum value across the FFI
/// boundary" has no instance here. The only non-pointer parameter is `int size`,
/// whose out-of-range space is covered by E1-E6 and E23; this test additionally
/// sweeps it densely for agreement.
#[test]
fn err_e24_no_enum_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("c_src");
    let mut sources = vec![];
    for sub in ["include", "src"] {
        for e in std::fs::read_dir(root.join(sub)).unwrap() {
            sources.push(e.unwrap().path());
        }
    }
    assert!(!sources.is_empty());
    for p in &sources {
        let text = std::fs::read_to_string(p).unwrap();
        for kw in ["enum", "switch", "#ifdef"] {
            assert!(
                !text.contains(kw),
                "ERRORS.md row E24 is stale: `{kw}` now appears in {}",
                p.display()
            );
        }
    }

    // Dense sweep of the `size` domain that does not trap (dest == src keeps
    // every negative value safe, and non-negative values stay in bounds).
    let base = painted(96);
    for size in -256i32..=64 {
        diff_expect(&format!("E24(size={size})"), &base, 0, 0, size.min(64));
    }
    for &size in &[i32::MIN, i32::MIN + 1, -0x4000_0000, -70_000, -1] {
        let out = diff_expect(&format!("E24(size={size})"), &base, 0, 0, size);
        assert_eq!(bits(&out), bits(&base));
    }
}

// ---------------------------------------------------------------------------
// harness self-check: the trap detector must not report success vacuously
// ---------------------------------------------------------------------------

/// If the crash worker ever stopped trapping, `diff_crash` would still see two
/// equal results. This guards against that by asserting the worker exits with a
/// signal (not code 0) for a known-trapping case, and that an unknown case
/// makes the worker fail loudly rather than silently succeed.
#[test]
fn err_zz_crash_detector_is_not_vacuous() {
    let (code, sig) = run_crash_child("e8", "c");
    assert_eq!(sig, Some(11), "detector broken: expected SIGSEGV, got code={code:?} sig={sig:?}");

    let (code, sig) = run_crash_child("no-such-case", "c");
    assert_eq!(sig, None, "unknown case should panic, not fault");
    assert_ne!(code, Some(0), "unknown case must not report success");
}
