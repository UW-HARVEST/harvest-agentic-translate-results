//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C function has no error return channel
//! at all (it is `void` with no status out-parameter and no `errno` write), so
//! "same error/rejection" means: for every invalid or boundary input, C and Rust
//! must take the SAME branch and produce the SAME bit-exact output words — or,
//! for the null-pointer rows, die with the SAME signal.

mod common;
use common::*;

/// Row 1 — `s == +0.0` exactly: the achromatic early return is the only
/// rejection-shaped branch in the function.
#[test]
fn err_01_s_exactly_zero() {
    for &h in ARM_HUES {
        for &v in SPECIAL_V {
            let src = [h, 0.0, v];
            assert_same("err01", &src);
            // Assert the branch really was taken: dest == (v, v, v).
            let out = call(c_impl(), &src);
            assert_eq!(out[0], v.to_bits(), "err01: dest[0] must be v verbatim");
            assert_eq!(out[1], v.to_bits(), "err01: dest[1] must be v verbatim");
            assert_eq!(out[2], v.to_bits(), "err01: dest[2] must be v verbatim");
        }
    }
}

/// Row 2 — `s == -0.0`: `-0.0 == 0.0` is true, so the branch IS taken.
#[test]
fn err_02_s_negative_zero() {
    for &h in ARM_HUES {
        for &v in SPECIAL_V {
            let src = [h, -0.0, v];
            assert_same("err02", &src);
            let out = call(c_impl(), &src);
            assert_eq!(
                (out[0], out[1], out[2]),
                (v.to_bits(), v.to_bits(), v.to_bits()),
                "err02: -0.0 saturation must still take the achromatic branch"
            );
        }
    }
}

/// Row 3 — smallest positive subnormal `s`: one step past exact zero, so the
/// branch must NOT be taken.
#[test]
fn err_03_s_smallest_subnormal() {
    let s = f32::from_bits(1);
    assert!(s != 0.0);
    for &h in ARM_HUES {
        for &v in SPECIAL_V {
            let src = [h, s, v];
            assert_same("err03", &src);
        }
    }
    // Confirm the chromatic path really was taken. For a subnormal `s` the two
    // paths agree for ordinary inputs (`1 - 1e-45` rounds back to exactly
    // `1.0`, so `p`, `q` and `t` all collapse to `v`), so the witness has to be
    // an input where `s` is amplified: with `h = +inf`, `f` is `+inf`, so
    // `q = v * (1 - s*inf) = v * -inf`, which the achromatic path can never
    // produce.
    let out = call(c_impl(), &[f32::INFINITY, s, 1.0]); // default arm: b = q
    assert_eq!(
        f32::from_bits(out[2]),
        f32::NEG_INFINITY,
        "err03: subnormal s must not be treated as zero"
    );
    assert_same("err03", &[f32::INFINITY, s, 1.0]);
    // Negative smallest subnormal too.
    for &h in ARM_HUES {
        assert_same("err03", &[h, -f32::from_bits(1), 1.0]);
    }
}

/// Row 4 — `s = NaN`: `NaN == 0` is false, so the chromatic path runs and
/// `p`/`q`/`t` become NaN.
#[test]
fn err_04_s_nan() {
    for &n in &[
        f32::NAN,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_5555),
        f32::from_bits(0x7F80_0001), // signalling
    ] {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("err04", &[h, n, v]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5 & 6 — null pointers. Undefined behaviour in both languages; the
// observable, comparable result is the fatal signal. Each call runs in a
// forked child with core dumps disabled, and the wait status is compared.
// ---------------------------------------------------------------------------

/// Outcome of running one call in a child process.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_in_child(imp: Impl, dest_null: bool, src_null: bool) -> Outcome {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: no core dumps, then perform the UB call.
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            let mut dest = [0f32; 8];
            let src = [30.0f32, 0.5, 0.75];
            let dp = if dest_null { std::ptr::null_mut() } else { dest.as_mut_ptr() };
            let sp = if src_null { std::ptr::null() } else { src.as_ptr() };
            (imp.f)(dp, sp);
            // Reached only if the call somehow survived.
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let r = libc::waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else {
            Outcome::Exit(libc::WEXITSTATUS(status))
        }
    }
}

/// Row 5 — `dest == NULL`.
#[test]
fn err_05_null_dest() {
    let c = run_in_child(c_impl(), true, false);
    assert_eq!(c, Outcome::Signal(libc::SIGSEGV), "C must fault on null dest, got {c:?}");
    for r in rust_impls() {
        let got = run_in_child(*r, true, false);
        assert_eq!(got, c, "err05: {} must fault identically to C", r.name);
    }
}

/// Row 6 — `src == NULL`.
#[test]
fn err_06_null_src() {
    let c = run_in_child(c_impl(), false, true);
    assert_eq!(c, Outcome::Signal(libc::SIGSEGV), "C must fault on null src, got {c:?}");
    for r in rust_impls() {
        let got = run_in_child(*r, false, true);
        assert_eq!(got, c, "err06: {} must fault identically to C", r.name);
    }
    // Both pointers null as well.
    let cb = run_in_child(c_impl(), true, true);
    for r in rust_impls() {
        assert_eq!(run_in_child(*r, true, true), cb, "err06: both-null mismatch in {}", r.name);
    }
}

/// Row 7 — `h = NaN`: `(int)floorf(NaN)` is UB; the reference build yields
/// `INT_MIN`, selecting the `default:` arm.
#[test]
fn err_07_h_nan() {
    for &n in &[
        f32::NAN,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0BAD),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0002),
    ] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err07", &[n, s, v]);
            }
        }
    }
    // The default arm is r=v, g=p, b=q. With NaN hue, f is NaN so q and t are
    // NaN; assert the arm selection matches that shape in the C reference.
    let out = call(c_impl(), &[f32::NAN, 1.0, 1.0]);
    assert_eq!(out[0], 1.0f32.to_bits(), "err07: default arm sets r = v");
    assert_eq!(out[1], 0.0f32.to_bits(), "err07: default arm sets g = p = v*(1-s)");
    assert!(f32::from_bits(out[2]).is_nan(), "err07: default arm b = q is NaN here");
}

/// Row 8 — `h = +INFINITY`.
#[test]
fn err_08_h_pos_inf() {
    for &s in SPECIAL_S {
        for &v in SPECIAL_V {
            assert_same("err08", &[f32::INFINITY, s, v]);
        }
    }
    let out = call(c_impl(), &[f32::INFINITY, 1.0, 1.0]);
    assert_eq!(out[0], 1.0f32.to_bits(), "err08: default arm r = v");
}

/// Row 9 — `h = -INFINITY`.
#[test]
fn err_09_h_neg_inf() {
    for &s in SPECIAL_S {
        for &v in SPECIAL_V {
            assert_same("err09", &[f32::NEG_INFINITY, s, v]);
        }
    }
}

/// Row 10 — `h` large enough that `h/60 >= 2^31` (out of `int` range).
#[test]
fn err_10_h_huge_positive() {
    for &h in &[1e30f32, 1e20, f32::MAX, 3.0e38, 2147483648.0 * 60.0, 1.3e11] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err10", &[h, s, v]);
            }
        }
    }
}

/// Row 11 — `h` negative enough that `h/60 < -2^31`.
#[test]
fn err_11_h_huge_negative() {
    for &h in &[-1e30f32, -1e20, f32::MIN, -3.0e38, -2147483648.0 * 60.0 * 2.0, -1.3e11] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err11", &[h, s, v]);
            }
        }
    }
}

/// Row 12 — `h/60` exactly `2^31`, the first value past `INT_MAX`.
#[test]
fn err_12_h_div60_exactly_2pow31() {
    const TWO31: f32 = 2147483648.0;
    // h chosen so that h/60 lands on / around 2^31 as exactly as binary32 allows.
    let candidates = [
        TWO31 * 60.0,
        f32::from_bits(TWO31.to_bits() - 1) * 60.0,
        f32::from_bits(TWO31.to_bits() + 1) * 60.0,
        f32::from_bits((TWO31 * 60.0).to_bits() - 1),
        f32::from_bits((TWO31 * 60.0).to_bits() + 1),
    ];
    for &h in &candidates {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err12", &[h, s, v]);
            }
        }
    }
    // Sanity: h/60 for the first candidate is at or above 2^31.
    assert!(candidates[0] / 60.0 >= TWO31);
}

/// Row 13 — `h/60` exactly `-2^31`, the last value still IN `int` range.
#[test]
fn err_13_h_div60_exactly_neg_2pow31() {
    const NEG_TWO31: f32 = -2147483648.0;
    let candidates = [
        NEG_TWO31 * 60.0,
        f32::from_bits((NEG_TWO31 * 60.0).to_bits() - 1),
        f32::from_bits((NEG_TWO31 * 60.0).to_bits() + 1),
        f32::from_bits(NEG_TWO31.to_bits() - 1) * 60.0,
        f32::from_bits(NEG_TWO31.to_bits() + 1) * 60.0,
    ];
    for &h in &candidates {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err13", &[h, s, v]);
            }
        }
    }
    assert!(candidates[0] / 60.0 <= NEG_TWO31);
}

/// Row 14 — `i >= 5`: hue past the documented `[0,360)` range, no clamping and
/// no wraparound, so `default:` is taken.
#[test]
fn err_14_i_ge_5_default_arm() {
    let mut rng = Rng::new(SEED ^ 0xE14);
    for &h in &[300.0f32, 330.0, 359.999, 360.0, 361.0, 420.0, 719.0, 1000.0, 1e6] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err14", &[h, s, v]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("err14", &[rng.range(300.0, 1e6), rng.range(0.001, 1.0), rng.unit()]);
    }
    // Verify the C reference does NOT wrap: h = 30 and h = 390 differ.
    let a = call(c_impl(), &[30.0, 1.0, 1.0]);
    let b = call(c_impl(), &[390.0, 1.0, 1.0]);
    assert_ne!(a, b, "err14: C must not normalise hue into [0,360)");
}

/// Row 15 — negative hue giving negative `i`, one step past the low end of the
/// valid range.
#[test]
fn err_15_negative_hue_negative_i() {
    let mut rng = Rng::new(SEED ^ 0xE15);
    for &h in &[-f32::from_bits(1), -1e-30f32, -0.001, -1.0, -30.0, -59.999, -60.0, -61.0, -360.0, -1e6] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("err15", &[h, s, v]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("err15", &[rng.range(-1e6, 0.0), rng.range(0.001, 1.0), rng.unit()]);
    }
    // h = -0.0 divides to -0.0, floorf(-0.0) = -0.0, (int)(-0.0) = 0 -> case 0.
    assert_same("err15", &[-0.0, 1.0, 1.0]);
    let out = call(c_impl(), &[-0.0, 1.0, 1.0]);
    let case0 = call(c_impl(), &[0.0, 1.0, 1.0]);
    assert_eq!(out, case0, "err15: -0.0 hue must behave as case 0, like +0.0");
}

/// Row 16 — `s > 1.0`, outside the documented range and unchecked.
#[test]
fn err_16_s_above_one() {
    let mut rng = Rng::new(SEED ^ 0xE16);
    for &s in &[f32::from_bits(1.0f32.to_bits() + 1), 1.5f32, 2.0, 100.0, 1e30, f32::MAX, f32::INFINITY] {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("err16", &[h, s, v]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("err16", &[rng.range(-720.0, 1080.0), rng.range(1.0, 1e6), rng.range(-100.0, 100.0)]);
    }
    // No rejection: p = v*(1-s) must actually go negative for s > 1.
    let out = call(c_impl(), &[30.0, 2.0, 1.0]); // case 0: b = p
    assert_eq!(f32::from_bits(out[2]), -1.0, "err16: p must be v*(1-s) = -1, unclamped");
}

/// Row 17 — `s < 0.0`, outside the documented range and unchecked.
#[test]
fn err_17_s_below_zero() {
    let mut rng = Rng::new(SEED ^ 0xE17);
    for &s in &[-f32::from_bits(1), -1e-30f32, -0.5, -1.0, -100.0, -1e30, f32::MIN, f32::NEG_INFINITY] {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("err17", &[h, s, v]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("err17", &[rng.range(-720.0, 1080.0), rng.range(-1e6, -1e-6), rng.range(-100.0, 100.0)]);
    }
    let out = call(c_impl(), &[30.0, -1.0, 1.0]); // case 0: b = p = 1*(1-(-1)) = 2
    assert_eq!(f32::from_bits(out[2]), 2.0, "err17: p must be v*(1-s) = 2, unclamped");
}

/// Row 18 — `v` outside `[0,1]`: NaN, `±inf`, negative, `-0.0`.
#[test]
fn err_18_v_out_of_range() {
    let mut rng = Rng::new(SEED ^ 0xE18);
    let vs = [
        f32::NAN,
        f32::from_bits(0x7FC0_9999),
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0f32,
        -1.0,
        -1e30,
        f32::MIN,
        f32::MAX,
        1.0000001,
        2.0,
        1e30,
    ];
    for &v in &vs {
        for &h in ARM_HUES {
            for &s in SPECIAL_S {
                assert_same("err18", &[h, s, v]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("err18", &[rng.range(-720.0, 1080.0), rng.range(-2.0, 3.0), rng.range(-1e6, 1e6)]);
    }
    // `inf * 0 = NaN`: s = 1 makes (1-s) exactly 0, so p = inf*0 must be NaN.
    let out = call(c_impl(), &[30.0, 1.0, f32::INFINITY]);
    assert!(f32::from_bits(out[2]).is_nan(), "err18: inf*0 must yield NaN for p");
}

/// Row 19 — the out-of-range-enum class is structurally absent from this API.
/// This test proves the claim mechanically rather than asserting it in prose.
#[test]
fn err_19_enum_surface_absent() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for rel in ["c_src/src/lib.c", "c_src/include/lib.h"] {
        let text = std::fs::read_to_string(root.join(rel)).expect("read C source");
        assert!(
            !text.contains("enum"),
            "{rel} contains an enum; ERRORS.md row 19 must be replaced by real \
             out-of-range-enum differential tests"
        );
    }
    // The only exported symbol takes two pointers and no integer/enum
    // parameter, so there is no integral value to push out of range.
    let sig = std::fs::read_to_string(root.join("c_src/include/lib.h")).unwrap();
    assert!(
        sig.contains("void hsv_to_rgb(float *dest, const float *src)"),
        "public signature changed; re-derive ERRORS.md: {sig}"
    );
}

/// Row 20 — the zero/oversized-length class is structurally absent: the API has
/// no length, count or size parameter. Proven mechanically.
#[test]
fn err_20_length_surface_absent() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let sig = std::fs::read_to_string(root.join("c_src/include/lib.h")).unwrap();
    for kw in ["size_t", "unsigned", "int ", "len", "count", "n)"] {
        assert!(
            !sig.contains(kw),
            "public header mentions `{kw}`; a length axis may exist and ERRORS.md \
             row 20 must be replaced by real length-boundary tests: {sig}"
        );
    }
    // The fixed arity of 3 is nevertheless verified not to be exceeded: the
    // canary slots in `call` are checked on every comparison, and here we
    // assert the C reference leaves dest[3..] untouched for a chromatic input.
    let out = call(c_impl(), &[210.0, 0.5, 0.75]);
    for (i, w) in out.iter().enumerate().skip(3) {
        assert_eq!(*w, 0x7F80_1D0Du32, "C wrote past dest[2] at index {i}");
    }
    for r in rust_impls() {
        let got = call(*r, &[210.0, 0.5, 0.75]);
        for (i, w) in got.iter().enumerate().skip(3) {
            assert_eq!(*w, 0x7F80_1D0Du32, "{} wrote past dest[2] at index {i}", r.name);
        }
    }
}
