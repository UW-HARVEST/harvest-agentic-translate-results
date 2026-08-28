//! Phase C — error/rejection-path differential tests, one `#[test]` per row of
//! `ERRORS.md`.
//!
//! The C has no error channel (a single `void` function, no asserts, no checks),
//! so "the same error" means one of:
//!   * the same concrete output bit pattern from the `s == 0` short-circuit or
//!     the `switch` `default:` catch-all, verified against an independently
//!     derived expectation (so the test proves *which* branch was taken rather
//!     than just "both agree"), or
//!   * the same fatal signal, for the undefined-behaviour rows. Those run in a
//!     forked child process (`crash_worker`) and compare
//!     `ExitStatus::signal()` between the C `.so` and the Rust `.so`.

mod common;

use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// independently derived expectations
// ---------------------------------------------------------------------------

/// For `h == k * 60` exactly, `f == 0`, so every arm collapses to a closed form
/// that can be written down without reimplementing the library:
///
/// | arm | (r, g, b) with f = 0 |
/// |-----|----------------------|
/// | 0   | (v, t=v(1-s), p=v(1-s)) |
/// | 1   | (q=v, v, p=v(1-s))      |
/// | 2   | (p=v(1-s), v, t=v(1-s)) |
/// | 3   | (p=v(1-s), q=v, v)      |
/// | 4   | (t=v(1-s), p=v(1-s), v) |
/// | def | (v, p=v(1-s), q=v)      |
fn expected_at_multiple_of_60(k: i32, s: f32, v: f32) -> [u32; 3] {
    let p = v * (1.0 - s);
    let arm = if (0..=4).contains(&k) { k } else { -1 };
    let t = match arm {
        0 => [v, p, p],
        1 => [v, v, p],
        2 => [p, v, p],
        3 => [p, v, v],
        4 => [p, p, v],
        _ => [v, p, v],
    };
    [t[0].to_bits(), t[1].to_bits(), t[2].to_bits()]
}

#[track_caller]
fn assert_both_equal(label: &str, h: f32, s: f32, v: f32, expected: [u32; 3]) {
    let (cc, rr) = run_pair(h, s, v);
    assert_eq!(
        cc,
        expected,
        "{label}: the C did not produce the independently derived result for \
         h={} s={} v={}\n  C        = {}\n  expected = {}",
        show(h),
        show(s),
        show(v),
        show3(&cc),
        show3(&expected)
    );
    assert_eq!(
        rr,
        cc,
        "{label}: C/Rust divergence for h={} s={} v={}\n  C    = {}\n  Rust = {}",
        show(h),
        show(s),
        show(v),
        show3(&cc),
        show3(&rr)
    );
}

// ===========================================================================
// rows 1-4: the `s == 0` short-circuit and what does *not* trigger it
// ===========================================================================

#[test]
fn err01_s_is_positive_zero() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..20_000 {
        let h = rng.any_f32();
        let v = rng.any_f32();
        let (cc, rr) = run_pair(h, 0.0, v);
        let want = [v.to_bits(); 3];
        assert_eq!(cc, want, "C did not short-circuit to [v,v,v] for v={}", show(v));
        assert_eq!(rr, want, "Rust did not short-circuit to [v,v,v] for v={}", show(v));
    }
}

#[test]
fn err02_s_is_negative_zero() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..20_000 {
        let h = rng.any_f32();
        let v = rng.any_f32();
        let (cc, rr) = run_pair(h, -0.0, v);
        let want = [v.to_bits(); 3];
        assert_eq!(cc, want, "-0.0 must also take the short-circuit (v={})", show(v));
        assert_eq!(rr, want, "-0.0 must also take the short-circuit (v={})", show(v));
    }
}

#[test]
fn err03_s_is_nan_takes_main_path() {
    // NaN != 0, so the short-circuit must NOT fire: the result must differ from
    // [v, v, v] in at least one channel (for h == 0, arm 0 gives (v, t, p) with
    // t = p = quiet(s), i.e. channels 1 and 2 carry the NaN payload of `s`).
    for s in nans() {
        let v = 0.75f32;
        let (cc, rr) = run_pair(0.0, s, v);
        assert_eq!(cc, rr, "C/Rust divergence for NaN s={}", show(s));
        assert_ne!(
            cc,
            [v.to_bits(); 3],
            "NaN s={} must not take the s==0 short-circuit",
            show(s)
        );
        let q = quiet_bits(s);
        assert_eq!(
            cc,
            [v.to_bits(), q, q],
            "unexpected NaN propagation for s={}",
            show(s)
        );
    }
}

fn quiet_bits(x: f32) -> u32 {
    x.to_bits() | 0x0040_0000
}

#[test]
fn err04_s_is_signalling_nan() {
    for bits in [0x7F80_0001u32, 0xFF80_0001, 0x7FBF_FFFF, 0xFFBF_FFFF, 0x7F80_0002] {
        let s = f(bits);
        assert!(s.is_nan() && bits & 0x0040_0000 == 0, "not an SNaN: {bits:#x}");
        let v = 0.5f32;
        let (cc, rr) = run_pair(0.0, s, v);
        assert_eq!(cc, rr, "C/Rust divergence for SNaN s={}", show(s));
        // the SNaN must come back *quieted*, with its payload preserved
        assert_eq!(
            cc,
            [v.to_bits(), bits | 0x0040_0000, bits | 0x0040_0000],
            "SNaN {bits:#x} was not quieted the way the C quiets it"
        );
    }
}

// ===========================================================================
// rows 5-12: out-of-domain `switch` selectors and the float->int conversion
// ===========================================================================

#[test]
fn err05_i_one_past_last_case() {
    // i == 5 is the first selector value past `case 4:`
    for &(s, v) in &[(0.5f32, 1.0f32), (1.0, 0.25), (0.75, -2.0), (1e-40, 1.0)] {
        assert_both_equal("err05", 300.0, s, v, expected_at_multiple_of_60(5, s, v));
    }
    let mut rng = Rng::new(0xC005);
    for _ in 0..5_000 {
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(-2.0, 2.0);
        assert_both_equal("err05", 300.0, s, v, expected_at_multiple_of_60(5, s, v));
    }
}

#[test]
fn err06_i_far_above_range() {
    let mut rng = Rng::new(0xC006);
    for k in [6, 7, 8, 12, 100, 1_000, 10_000, 100_000, 1_000_000, 16_777_216] {
        for _ in 0..200 {
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            let v = rng.range(-2.0, 2.0);
            let h = k as f32 * 60.0;
            assert_both_equal("err06", h, s, v, expected_at_multiple_of_60(k, s, v));
        }
    }
}

#[test]
fn err07_i_negative() {
    let mut rng = Rng::new(0xC007);
    for k in [-1, -2, -3, -6, -100, -1_000, -1_000_000, -16_777_216] {
        for _ in 0..200 {
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            let v = rng.range(-2.0, 2.0);
            let h = k as f32 * 60.0;
            assert_both_equal("err07", h, s, v, expected_at_multiple_of_60(k, s, v));
        }
    }
}

#[test]
fn err08_h_nan_gives_int_min() {
    // (int)floorf(NaN) is UB in C; `cvttss2si` yields INT_MIN, which the
    // unsigned `ja` bound check sends to `default:`. So r = v, g = p = v*(1-s)
    // and b = q, which for a NaN `f` is exactly `quiet(h)`.
    for h in nans() {
        for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0), (1e-40, -2.0)] {
            let want = [
                v.to_bits(),
                (v * (1.0 - s)).to_bits(),
                quiet_bits(h),
            ];
            assert_both_equal("err08", h, s, v, want);
        }
    }
}

#[test]
fn err09_h_signalling_nan() {
    for bits in [0x7F80_0001u32, 0xFF80_0001, 0x7FBF_FFFF, 0xFFBF_FFFF] {
        let h = f(bits);
        let (s, v) = (0.5f32, 0.75f32);
        let want = [v.to_bits(), (v * (1.0 - s)).to_bits(), bits | 0x0040_0000];
        assert_both_equal("err09", h, s, v, want);
    }
}

#[test]
fn err10_h_above_int_range() {
    // h/60 >= 2^31 -> cvttss2si returns INT_MIN -> default arm, and
    // f = h/60 - (float)INT_MIN.
    let hs = [
        f32::INFINITY,
        2_147_483_648.0f32 * 60.0,
        f32::MAX,
        1e30,
        3.4e38,
        2_147_483_904.0 * 60.0,
    ];
    for &h in &hs {
        for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0), (1e-40, 1.0)] {
            let hh = h / 60.0;
            let ff = hh - (i32::MIN as f32);
            let want = [
                v.to_bits(),
                (v * (1.0 - s)).to_bits(),
                (v * (1.0 - s * ff)).to_bits(),
            ];
            assert_both_equal("err10", h, s, v, want);
        }
    }
}

#[test]
fn err11_h_below_int_range() {
    let hs = [
        f32::NEG_INFINITY,
        -2_147_484_000.0f32 * 60.0,
        f32::MIN,
        -1e30,
        -3.4e38,
    ];
    for &h in &hs {
        for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0), (1e-40, 1.0)] {
            let hh = h / 60.0;
            let ff = hh - (i32::MIN as f32);
            let want = [
                v.to_bits(),
                (v * (1.0 - s)).to_bits(),
                (v * (1.0 - s * ff)).to_bits(),
            ];
            assert_both_equal("err11", h, s, v, want);
        }
    }
}

#[test]
fn err12_h_at_int_min_boundary() {
    // h/60 == -2^31 exactly: the one out-of-`case`-range selector where the
    // saturating Rust cast and `cvttss2si` happen to agree.
    let h = -2_147_483_648.0f32 * 60.0;
    assert_eq!(h / 60.0, -2_147_483_648.0f32);
    for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0), (1e-40, 1.0), (1.5, -1.0)] {
        // f = -2^31 - (-2^31) = 0 -> the closed form of the default arm applies
        assert_both_equal("err12", h, s, v, expected_at_multiple_of_60(-1, s, v));
    }
    // One step *away* from zero: |h/60| > 2^31, so the conversion is out of
    // range and `cvttss2si` returns INT_MIN, giving f = h/60 + 2^31.
    let below = f(h.to_bits() + 1);
    assert!(below / 60.0 < -2_147_483_648.0f32);
    for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0)] {
        let ff = below / 60.0 - (i32::MIN as f32);
        let want = [
            v.to_bits(),
            (v * (1.0 - s)).to_bits(),
            (v * (1.0 - s * ff)).to_bits(),
        ];
        assert_both_equal("err12 below", below, s, v, want);
    }
    // One step *toward* zero: h/60 lands exactly on -2147483520, which IS
    // representable as an `int`, so the conversion is in range, f == 0, and the
    // negative selector still reaches `default:`.
    let above = f(h.to_bits() - 1);
    assert_eq!(above / 60.0, -2_147_483_520.0f32);
    for &(s, v) in &[(0.5f32, 0.75f32), (1.0, 1.0)] {
        assert_both_equal("err12 above", above, s, v, expected_at_multiple_of_60(-1, s, v));
    }
}

// ===========================================================================
// rows 13-18: unchecked out-of-range `s` / `v`
// ===========================================================================

#[test]
fn err13_s_below_range() {
    let mut rng = Rng::new(0xC013);
    for &s in &[-1e-45f32, -f32::MIN_POSITIVE, -0.5, -1.0, -1.5, -1e30, f32::MIN] {
        for k in -2..=6 {
            let v = 0.75f32;
            // no clamping: p = v*(1-s) must exceed v
            assert_both_equal(
                "err13",
                k as f32 * 60.0,
                s,
                v,
                expected_at_multiple_of_60(k, s, v),
            );
        }
        for _ in 0..1_000 {
            let h = rng.range(-400.0, 400.0);
            let v = rng.range(-2.0, 2.0);
            assert_same("err13 fuzz", h, s, v);
        }
    }
}

#[test]
fn err14_s_above_range() {
    let mut rng = Rng::new(0xC014);
    for &s in &[1.0000001f32, 1.5, 2.0, 1e30, f32::MAX, 16_777_216.0] {
        for k in -2..=6 {
            let v = 0.75f32;
            assert_both_equal(
                "err14",
                k as f32 * 60.0,
                s,
                v,
                expected_at_multiple_of_60(k, s, v),
            );
        }
        for _ in 0..1_000 {
            let h = rng.range(-400.0, 400.0);
            let v = rng.range(-2.0, 2.0);
            assert_same("err14 fuzz", h, s, v);
        }
    }
}

#[test]
fn err15_s_infinite() {
    let mut rng = Rng::new(0xC015);
    for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
        for k in -2..=6 {
            for &v in &[0.75f32, 1.0, -1.0, 0.0, -0.0, f32::INFINITY] {
                // f == 0 at multiples of 60, so s*f = inf*0 = the default QNaN
                assert_same("err15", k as f32 * 60.0, s, v);
            }
        }
        for _ in 0..2_000 {
            let h = rng.range(-400.0, 400.0);
            let v = rng.range(-2.0, 2.0);
            assert_same("err15 fuzz", h, s, v);
        }
        // 1 - inf = -inf, so p = v * -inf
        let (cc, rr) = run_pair(0.0, s, 1.0);
        assert_eq!(cc, rr);
        let p = 1.0f32 * (1.0 - s);
        assert!(p.is_infinite());
    }
}

#[test]
fn err16_v_out_of_range() {
    let mut rng = Rng::new(0xC016);
    for &v in &[-1e-45f32, -0.5, -1.0, -1e30, f32::MIN, 1.5, 1e30, f32::MAX] {
        for k in -2..=6 {
            let s = 0.5f32;
            assert_both_equal(
                "err16",
                k as f32 * 60.0,
                s,
                v,
                expected_at_multiple_of_60(k, s, v),
            );
        }
        for _ in 0..1_000 {
            let h = rng.range(-400.0, 400.0);
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            assert_same("err16 fuzz", h, s, v);
        }
    }
}

#[test]
fn err17_v_inf_or_nan() {
    let mut rng = Rng::new(0xC017);
    let vs: Vec<f32> = [f32::INFINITY, f32::NEG_INFINITY]
        .into_iter()
        .chain(nans())
        .collect();
    for &v in &vs {
        for k in -2..=6 {
            for &s in &[0.25f32, 0.5, 1.0, 1.5, -0.5, 1e-40] {
                assert_same("err17", k as f32 * 60.0, s, v);
            }
        }
        for _ in 0..1_000 {
            let h = rng.range(-400.0, 400.0);
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            assert_same("err17 fuzz", h, s, v);
        }
    }
}

#[test]
fn err18_zero_times_inf_qnan() {
    // v == 0 with s == +-inf makes `v * (1 - s)` an invalid operation, which
    // yields the hardware's default QNaN (0xffc00000), *not* a propagated
    // payload. Assert that exact bit pattern from both objects.
    for &v in &[0.0f32, -0.0f32] {
        for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
            let (cc, rr) = run_pair(0.0, s, v);
            assert_eq!(cc, rr, "C/Rust divergence for s={} v={}", show(s), show(v));
            assert_eq!(
                cc[0],
                v.to_bits(),
                "arm 0 must return r = v for s={} v={}",
                show(s),
                show(v)
            );
            assert_eq!(
                cc[1], 0xFFC0_0000,
                "expected the default QNaN from 0*inf, got {}",
                show(f(cc[1]))
            );
            assert_eq!(cc[2], 0xFFC0_0000);
        }
    }
    // inf - inf is the other invalid operation reachable here (s = inf, f = inf)
    for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
            assert_same("err18 inf-inf", h, s, 1.0);
        }
    }
}

// ===========================================================================
// rows 19-23: undefined behaviour — signal parity, checked in a child process
// ===========================================================================

const SPEC_ENV: &str = "HSV_CRASH_SPEC";
const FILE_ENV: &str = "HSV_CRASH_FILE";

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_SHARED: i32 = 0x01;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;
const O_RDWR: i32 = 2;
const PAGE: usize = 4096;

extern "C" {
    fn mmap(
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
    fn open(path: *const u8, flags: i32, mode: u32) -> i32;
}

/// Reserve `2 * PAGE` bytes with no access rights at all.
unsafe fn reserve_two_pages() -> *mut u8 {
    let p = mmap(
        std::ptr::null_mut(),
        2 * PAGE,
        PROT_NONE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    assert!(p as isize != -1, "mmap failed");
    p
}

/// The body that runs in the child process. Returns normally only if the call
/// did *not* fault.
fn crash_body(spec: &str) {
    let (which, case) = spec.split_once(':').expect("bad spec");
    let imp = match which {
        "c" => c(),
        "rust" => rust(),
        _ => panic!("bad impl {which}"),
    };
    let mut good = [30.0f32, 0.5, 0.75];
    let mut out = [0f32; 3];

    unsafe {
        match case {
            // row 19
            "null_dest_main" => imp.call(std::ptr::null_mut(), good.as_ptr()),
            "null_dest_early" => {
                good[1] = 0.0;
                imp.call(std::ptr::null_mut(), good.as_ptr())
            }
            // row 20
            "null_src" => imp.call(out.as_mut_ptr(), std::ptr::null()),
            // row 21
            "both_null" => imp.call(std::ptr::null_mut(), std::ptr::null()),
            // row 22: src[0] unreadable, src[1..2] readable, s == 0
            "guard_src0_early" | "guard_src0_main" => {
                let base = reserve_two_pages();
                assert_eq!(
                    mprotect(base.add(PAGE), PAGE, PROT_READ | PROT_WRITE),
                    0,
                    "mprotect failed"
                );
                let src = base.add(PAGE - 4) as *mut f32;
                // src[1] and src[2] live in the readable page
                let s = if case == "guard_src0_early" { 0.0f32 } else { 0.5f32 };
                std::ptr::write(src.add(1), s);
                std::ptr::write(src.add(2), 0.75);
                imp.call(out.as_mut_ptr(), src)
            }
            // row 23: dest[2] unwritable, dest[0..1] writable and file-backed so
            // the parent can see the partial write the child committed
            "partial_store_main" | "partial_store_early" => {
                let path = std::env::var(FILE_ENV).expect("no crash file");
                let mut cpath = path.into_bytes();
                cpath.push(0);
                let fd = open(cpath.as_ptr(), O_RDWR, 0);
                assert!(fd >= 0, "open failed");
                let base = reserve_two_pages();
                let mapped = mmap(
                    base,
                    PAGE,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_FIXED,
                    fd,
                    0,
                );
                assert_eq!(mapped, base, "file mmap failed");
                let dest = base.add(PAGE - 8) as *mut f32;
                if case == "partial_store_early" {
                    good[1] = 0.0;
                }
                imp.call(dest, good.as_ptr())
            }
            other => panic!("unknown case {other}"),
        }
    }
    eprintln!("no fault for {spec} (out = {out:?})");
}

#[test]
fn crash_worker() {
    // Only does anything in the child process spawned by `run_child` below.
    let Ok(spec) = std::env::var(SPEC_ENV) else {
        return;
    };
    crash_body(&spec);
    std::process::exit(0);
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_child(spec: &str, file: Option<&str>) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", "crash_worker", "--nocapture", "--test-threads=1"])
        .env(SPEC_ENV, spec)
        .env("RUST_BACKTRACE", "0");
    if let Some(f) = file {
        cmd.env(FILE_ENV, f);
    }
    // make sure the child resolves the same shared objects
    cmd.env("HSV_C_SO", c_so_path()).env("HSV_RUST_SO", rust_so_path());
    let status = cmd.output().expect("spawn crash worker").status;
    Outcome {
        signal: status.signal(),
        code: status.code(),
    }
}

#[track_caller]
fn assert_same_fault(case: &str) -> Outcome {
    let co = run_child(&format!("c:{case}"), None);
    let ro = run_child(&format!("rust:{case}"), None);
    assert_eq!(
        co, ro,
        "{case}: C and Rust terminated differently (C={co:?}, Rust={ro:?})"
    );
    co
}

const SIGSEGV: i32 = 11;

#[test]
fn err19_null_dest_crash_parity() {
    for case in ["null_dest_main", "null_dest_early"] {
        let o = assert_same_fault(case);
        assert_eq!(
            o.signal,
            Some(SIGSEGV),
            "{case}: expected SIGSEGV from the unchecked store, got {o:?}"
        );
    }
}

#[test]
fn err20_null_src_crash_parity() {
    let o = assert_same_fault("null_src");
    assert_eq!(
        o.signal,
        Some(SIGSEGV),
        "expected SIGSEGV from the unchecked load, got {o:?}"
    );
}

#[test]
fn err21_both_null_crash_parity() {
    let o = assert_same_fault("both_null");
    assert_eq!(o.signal, Some(SIGSEGV), "expected SIGSEGV, got {o:?}");
}

#[test]
fn err22_unconditional_h_load_faults() {
    // `float h = src[0];` runs before `if (s == 0)`, so even the early-return
    // path touches src[0]. Both objects must fault in both configurations.
    for case in ["guard_src0_early", "guard_src0_main"] {
        let o = assert_same_fault(case);
        assert_eq!(
            o.signal,
            Some(SIGSEGV),
            "{case}: src[0] is loaded unconditionally, so this must fault ({o:?})"
        );
    }
}

#[test]
fn err23_partial_store_before_fault() {
    for case in ["partial_store_main", "partial_store_early"] {
        let mut results = Vec::new();
        for which in ["c", "rust"] {
            let path = std::env::temp_dir().join(format!("hsv_{case}_{which}.bin"));
            std::fs::write(&path, vec![0xAAu8; PAGE]).expect("write temp file");
            let o = run_child(
                &format!("{which}:{case}"),
                Some(path.to_str().expect("utf8 path")),
            );
            let bytes = std::fs::read(&path).expect("read temp file");
            let _ = std::fs::remove_file(&path);
            results.push((o, bytes[PAGE - 8..PAGE].to_vec()));
        }
        assert_eq!(
            results[0].0, results[1].0,
            "{case}: different termination (C={:?}, Rust={:?})",
            results[0].0, results[1].0
        );
        assert_eq!(
            results[0].0.signal,
            Some(SIGSEGV),
            "{case}: expected SIGSEGV on the third store, got {:?}",
            results[0].0
        );
        assert_eq!(
            results[0].1, results[1].1,
            "{case}: the prefix of `dest` committed before the fault differs\n  \
             C    = {:02x?}\n  Rust = {:02x?}",
            results[0].1, results[1].1
        );
        // and it must really be a *partial* write: dest[0]/dest[1] changed
        assert_ne!(
            results[0].1,
            vec![0xAAu8; 8],
            "{case}: expected dest[0] and dest[1] to be committed before the fault"
        );
    }
}

// ===========================================================================
// rows 24-25: the out-of-range selector sweep and misaligned pointers
// ===========================================================================

#[test]
fn err24_selector_sweep() {
    // Every `i` reachable from a float hue that is *not* one of the five
    // `case` labels must land in `default:` and never read past the jump table.
    let mut ks: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -16_777_216,
        -1_000_000,
        -1_000,
        -2,
        -1,
        5,
        6,
        7,
        1_000,
        1_000_000,
        16_777_216,
    ];
    for k in -80..=80 {
        ks.push(k);
    }
    ks.sort_unstable();
    ks.dedup();
    for &k in &ks {
        // hues whose floor(h/60) == k (exactly representable for |k| <= 2^24)
        let h = (k as f32) * 60.0;
        let s = 0.5f32;
        let v = 0.75f32;
        if k.unsigned_abs() <= 16_777_216 && (h / 60.0) == k as f32 {
            assert_both_equal("err24", h, s, v, expected_at_multiple_of_60(k, s, v));
        } else {
            assert_same("err24", h, s, v);
        }
    }
    // INT_MIN specifically (reached through NaN and through overflow)
    for h in nans().chain([f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN]) {
        assert_same("err24 int_min", h, 0.5, 0.75);
    }
}

#[test]
fn err25_misaligned_pointers() {
    let mut rng = Rng::new(0xC025);
    for _ in 0..2_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        for src_off in 0..8usize {
            for dst_off in 0..8usize {
                assert_same_shaped("err25", src, src_off, 24 + dst_off);
            }
        }
    }
    // aligned and misaligned must agree with each other too, not just C vs Rust
    for _ in 0..2_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        let (aligned, _) = run_shaped(src, 0, 32);
        let (mis, _) = run_shaped(src, 1, 33);
        assert_eq!(
            aligned[32..44],
            mis[33..45],
            "misalignment changed the C's result for {src:?}"
        );
    }
}
