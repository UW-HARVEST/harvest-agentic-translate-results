//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `hsl_to_rgb` returns `void` and contains no `assert`, no range check and no
//! null check (see the greps in `ERRORS.md`), so "the same error/rejection"
//! means one of:
//!
//!   * the same three output words, bit for bit (for the degenerate/exceptional
//!     *value* conditions — the library's only way of "reporting" anything), or
//!   * the same fatal signal, raised in a forked child process (for the
//!     undefined-behaviour conditions: null pointers).
//!
//! Every row asserts the *specific* result, not merely "both did something".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// E1 / E2 — the only `return` in the library: s == 0
// ---------------------------------------------------------------------------

/// The early return must copy `l` verbatim into all three slots, for every hue
/// (including NaN/Inf hues, which never even reach `fmodf`).
fn assert_early_return_copies_l(ctx: &str, s_zero: f32) {
    let pool = specials_and_nans();
    let mut rng = Rng::new(0xE001 ^ s_zero.to_bits() as u64);

    for &h in &pool {
        for &l in &pool {
            assert_same(ctx, h, s_zero, l);
            // Pin the *expected* C behaviour, so the row cannot pass by both
            // sides being wrong in the same way.
            let out = run(c_lib(), h, s_zero, l);
            assert_eq!(
                out,
                [l.to_bits(); 3],
                "[{ctx}] C did not copy l verbatim for h={:#010x} l={:#010x}",
                h.to_bits(),
                l.to_bits()
            );
        }
    }
    for _ in 0..50_000 {
        let h = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same(ctx, h, s_zero, l);
        assert_eq!(run(c_lib(), h, s_zero, l), [l.to_bits(); 3]);
    }
    for &h in HUE_BOUNDARIES {
        for &l in &pool {
            assert_same(ctx, h, s_zero, l);
        }
    }
}

#[test]
fn e1_s_is_positive_zero() {
    assert_eq!(0.0f32.to_bits(), 0x0000_0000);
    assert_early_return_copies_l("E1 s=+0.0", 0.0);
}

/// `-0.0f == 0` is *true* in C, so `-0.0` takes the same early return. A
/// translation that compared bit patterns instead of values would fail here.
#[test]
fn e2_s_is_negative_zero() {
    let nz = -0.0f32;
    assert_eq!(nz.to_bits(), 0x8000_0000);
    assert_early_return_copies_l("E2 s=-0.0", nz);
}

// ---------------------------------------------------------------------------
// E3 — NaN saturation does NOT take the early return
// ---------------------------------------------------------------------------

#[test]
fn e3_s_is_nan() {
    let pool = specials_and_nans();
    for &s in &nan_floats() {
        // Confirm the C really falls through: with a NaN saturation and a hue in
        // sector B1, dest[2] is `m`, which is NaN — whereas the early return
        // would have produced three copies of `l`.
        let out = run(c_lib(), 30.0, s, 0.25);
        assert!(
            f32::from_bits(out[2]).is_nan(),
            "expected the NaN-saturation path to poison m"
        );
        assert_ne!(
            out,
            [0.25f32.to_bits(); 3],
            "NaN saturation must NOT take the s==0 early return"
        );

        for &h in &pool {
            for &l in &pool {
                assert_same("E3 s=NaN", h, s, l);
            }
        }
        for &h in HUE_BOUNDARIES {
            for &l in &pool {
                assert_same("E3 s=NaN boundary", h, s, l);
            }
        }
        let mut rng = Rng::new(0xE003 ^ s.to_bits() as u64);
        for _ in 0..20_000 {
            let h = rng.bits_f32();
            let l = rng.bits_f32();
            assert_same("E3 s=NaN fuzz", h, s, l);
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — the [120, 180) hole created by the `h < 120 && h < 180` typo
// ---------------------------------------------------------------------------

#[test]
fn e4_hue_hole_120_to_180() {
    // Pin the C behaviour: grey, i.e. three copies of m.
    let out = run(c_lib(), 150.0, 1.0, 0.5);
    assert_eq!(out[0], out[1]);
    assert_eq!(out[1], out[2]);
    // m for s=1, l=0.5 is 0.5 - 0.5*1 = 0.0
    assert_eq!(f32::from_bits(out[0]), 0.0);

    let mut rng = Rng::new(0xE004);
    for _ in 0..50_000 {
        let h = rng.range(120.0, 180.0);
        let s = {
            let v = rng.range(0.0, 1.0);
            if v == 0.0 { 1.0 } else { v }
        };
        let l = rng.range(0.0, 1.0);
        assert_same("E4 hue hole", h, s, l);
        // the grey property must hold for the C on every one of these
        let o = run(c_lib(), h, s, l);
        assert_eq!(o[0], o[1], "hole should be grey at h={h}");
        assert_eq!(o[1], o[2], "hole should be grey at h={h}");
    }
    // Boundaries of the hole, exactly and one ULP either side.
    for &h in &[
        120.0f32,
        next_up(120.0),
        next_down(120.0),
        next_up(180.0),
        next_down(180.0),
        180.0,
    ] {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("E4 hole edges", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — negative hue (one step past the low end of the documented range)
// ---------------------------------------------------------------------------

#[test]
fn e5_negative_hue() {
    // Pin: h = -30 must reach branch 3 => {m, c+m, x+m}, and x is negative
    // because fmodf(-0.5, 2) = -0.5 => 1 - |-1.5| = -0.5.
    let (s, l) = (1.0f32, 0.5f32);
    let out = run(c_lib(), -30.0, s, l);
    let (r, g, b) = (
        f32::from_bits(out[0]),
        f32::from_bits(out[1]),
        f32::from_bits(out[2]),
    );
    assert_eq!(r, 0.0, "branch 3 stores m in dest[0]");
    assert_eq!(g, 1.0, "branch 3 stores c+m in dest[1]");
    assert_eq!(b, -0.5, "x is negative for a negative hue");

    let mut rng = Rng::new(0xE005);
    for _ in 0..50_000 {
        let h = -rng.log_uniform().abs();
        let s = {
            let v = rng.range(0.0, 1.0);
            if v == 0.0 { 1.0 } else { v }
        };
        let l = rng.range(0.0, 1.0);
        assert_same("E5 negative hue", h, s, l);
    }
    for &h in &[
        next_down(0.0),
        next_down(-0.0),
        -f32::MIN_POSITIVE,
        -1e-45f32,
        -f32::MAX,
        f32::NEG_INFINITY,
        -60.0,
        -120.0,
        -180.0,
        -360.0,
    ] {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("E5 negative hue specials", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — h >= 360 (one step past the high end); no wrap-around
// ---------------------------------------------------------------------------

#[test]
fn e6_hue_at_or_above_360() {
    // 360 itself is grey: the last test is the strict `h < 360.0f`.
    let out = run(c_lib(), 360.0, 1.0, 0.5);
    assert_eq!(out[0], out[1]);
    assert_eq!(out[1], out[2]);
    // ... whereas one ULP below 360 is *not* grey.
    let out2 = run(c_lib(), next_down(360.0), 1.0, 0.5);
    assert_ne!(out2[0], out2[1], "h just below 360 must take branch 6");

    let mut rng = Rng::new(0xE006);
    for _ in 0..50_000 {
        let h = 360.0 + rng.log_uniform().abs();
        let s = {
            let v = rng.range(0.0, 1.0);
            if v == 0.0 { 1.0 } else { v }
        };
        let l = rng.range(0.0, 1.0);
        assert_same("E6 h>=360", h, s, l);
    }
    for &h in &[
        360.0f32,
        next_up(360.0),
        720.0,
        1e30,
        f32::MAX,
        f32::INFINITY,
    ] {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("E6 h>=360 specials", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — NaN hue: all twelve comparisons unordered => the final else
// ---------------------------------------------------------------------------

#[test]
fn e7_hue_is_nan() {
    for &h in &nan_floats() {
        // Pin: grey (three copies of m), and m is finite when s/l are.
        let out = run(c_lib(), h, 1.0, 0.5);
        assert_eq!(out[0], out[1], "NaN hue must fall into the grey else");
        assert_eq!(out[1], out[2], "NaN hue must fall into the grey else");
        assert_eq!(f32::from_bits(out[0]), 0.0, "m = 0.5 - 0.5*1 = 0");

        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("E7 h=NaN", h, s, l);
            }
        }
        let mut rng = Rng::new(0xE007 ^ h.to_bits() as u64);
        for _ in 0..20_000 {
            let s = rng.bits_f32();
            let l = rng.bits_f32();
            assert_same("E7 h=NaN fuzz", h, s, l);
        }
    }
}

// ---------------------------------------------------------------------------
// E8 — h = +/-Inf drives fmodf into its domain-error path
// ---------------------------------------------------------------------------

#[test]
fn e8_hue_is_infinite() {
    // Pin: fmodf(+/-Inf, 2) is the libm domain error (x*y)/(x*y) = Inf/Inf,
    // i.e. the default quiet NaN, so `x` is NaN.
    // +Inf falls into the grey else; -Inf takes branch 3 => {m, c+m, x+m}.
    let (s, l) = (1.0f32, 0.5f32);

    let pos = run(c_lib(), f32::INFINITY, s, l);
    assert_eq!(pos[0], pos[1]);
    assert_eq!(pos[1], pos[2]);
    assert_eq!(f32::from_bits(pos[0]), 0.0, "+Inf hue => grey m");

    let neg = run(c_lib(), f32::NEG_INFINITY, s, l);
    assert_eq!(f32::from_bits(neg[0]), 0.0, "-Inf hue => branch 3, dest[0]=m");
    assert_eq!(f32::from_bits(neg[1]), 1.0, "-Inf hue => dest[1]=c+m");
    assert!(
        f32::from_bits(neg[2]).is_nan(),
        "-Inf hue => dest[2]=x+m is NaN because fmodf(-Inf,2) is NaN"
    );
    assert_eq!(
        neg[2], 0x7fc0_0000,
        "expected the default quiet NaN from Inf/Inf"
    );

    // Full differential coverage; this is the row that exercises glibc's fmodf
    // against the statically linked compiler_builtins fmodf.
    for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("E8 h=+/-Inf", h, s, l);
            }
        }
        let mut rng = Rng::new(0xE008 ^ h.to_bits() as u64);
        for _ in 0..20_000 {
            let s = rng.bits_f32();
            let l = rng.bits_f32();
            assert_same("E8 h=+/-Inf fuzz", h, s, l);
        }
    }
}

// ---------------------------------------------------------------------------
// E9 — l = +/-Inf: Inf - Inf produces the default quiet NaN inside m
// ---------------------------------------------------------------------------

#[test]
fn e9_lightness_is_infinite() {
    // Pin: l=+Inf, s=1  =>  2l-1 = Inf, |.| = Inf, 1-Inf = -Inf, c = -Inf,
    // m = Inf - 0.5*(-Inf) = Inf + Inf = Inf (not NaN);
    // l=+Inf, s=-1 =>  c = +Inf, m = Inf - Inf = NaN.
    let m_inf = run(c_lib(), 30.0, 1.0, f32::INFINITY);
    assert_eq!(
        f32::from_bits(m_inf[2]),
        f32::INFINITY,
        "l=+Inf, s=1 => m = +Inf"
    );

    let m_nan = run(c_lib(), 30.0, -1.0, f32::INFINITY);
    assert!(
        f32::from_bits(m_nan[2]).is_nan(),
        "l=+Inf, s=-1 => m = Inf - Inf = NaN"
    );
    // x86's "QNaN floating-point indefinite" — the NaN an SSE instruction
    // *generates* for an invalid operation — is 0xffc00000: the sign bit is
    // SET, unlike Rust's `f32::NAN` (0x7fc00000). Any translation that
    // synthesised a NaN with `f32::NAN` instead of letting the hardware produce
    // it would fail here.
    assert_eq!(
        m_nan[2], 0xffc0_0000,
        "x86 indefinite quiet NaN from Inf - Inf"
    );

    for &l in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &s in &specials_and_nans() {
            for &h in &specials_and_nans() {
                assert_same("E9 l=+/-Inf", h, s, l);
            }
            for &h in HUE_BOUNDARIES {
                assert_same("E9 l=+/-Inf boundary hue", h, s, l);
            }
        }
        let mut rng = Rng::new(0xE009 ^ l.to_bits() as u64);
        for _ in 0..20_000 {
            let h = random_hue_any_sector(&mut rng);
            let s = rng.bits_f32();
            assert_same("E9 l=+/-Inf fuzz", h, s, l);
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — 0 * Inf inside the chroma term
// ---------------------------------------------------------------------------

#[test]
fn e10_zero_times_infinite_chroma() {
    // l in {0, 1} makes `1 - |2l-1|` exactly 0, so an infinite saturation gives
    // c = 0 * Inf = the default quiet NaN, which poisons m and x.
    for &l in &[0.0f32, 1.0f32, -0.0f32] {
        for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
            let out = run(c_lib(), 30.0, s, l);
            for (i, &w) in out.iter().enumerate() {
                assert!(
                    f32::from_bits(w).is_nan(),
                    "l={l} s={s}: dest[{i}] should be NaN (0*Inf), got {:#010x}",
                    w
                );
            }
            // Again the x86 indefinite QNaN, sign bit set (see E9).
            assert_eq!(
                out[2], 0xffc0_0000,
                "m should be the x86 indefinite quiet NaN produced by 0*Inf"
            );

            for &h in &specials_and_nans() {
                assert_same("E10 0*Inf chroma", h, s, l);
            }
            for &h in HUE_BOUNDARIES {
                assert_same("E10 0*Inf chroma boundary", h, s, l);
            }
            let mut rng = Rng::new(0xE010 ^ s.to_bits() as u64 ^ l.to_bits() as u64);
            for _ in 0..20_000 {
                let h = if rng.below(2) == 0 {
                    random_hue_any_sector(&mut rng)
                } else {
                    rng.bits_f32()
                };
                assert_same("E10 0*Inf chroma fuzz", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E11 / E12 / E13 — null pointers: undefined behaviour, no null check anywhere.
// Both libraries must fault with the same signal. Run in a forked child.
// ---------------------------------------------------------------------------

/// Which null-pointer scenario a crash child should perform.
const CRASH_ENV: &str = "HSL_CRASH_CASE";

/// The child entry point. Ignored by default so a normal run never executes it;
/// the parent invokes it explicitly with `--ignored --exact`.
#[test]
#[ignore = "crash child: only run by the null-pointer parent tests"]
fn crash_child() {
    let case = std::env::var(CRASH_ENV).expect("crash child invoked without a case");
    let (which, scenario) = case.split_once(':').unwrap();
    let lib: &Lib = match which {
        "c" => c_lib(),
        name => rust_libs()
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("no such library variant: {name}")),
    };
    // Flush so the parent sees we got this far before the fault.
    eprintln!("child: {case}");
    let src_ok: [f32; 3] = [30.0, 1.0, 0.5];
    let src_zero_sat: [f32; 3] = [30.0, 0.0, 0.5];
    let mut dest = [0.0f32; 3];
    unsafe {
        match scenario {
            // E11: src == NULL
            "null_src" => (lib.f)(dest.as_mut_ptr(), std::ptr::null()),
            // E12: dest == NULL, general path
            "null_dest" => (lib.f)(std::ptr::null_mut(), src_ok.as_ptr()),
            // E13: dest == NULL on the s == 0 early-return path
            "null_dest_zero_sat" => (lib.f)(std::ptr::null_mut(), src_zero_sat.as_ptr()),
            // E11/E12 combined
            "both_null" => (lib.f)(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("unknown scenario {other}"),
        }
    }
    // If we get here the library performed a null check, which the C provably
    // does not. Report it as a distinguishable, non-crashing outcome.
    eprintln!("child: survived");
    std::process::exit(66);
}

/// Outcome of a crash child: either a fatal signal number, or an exit code.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_crash_child(which: &str, scenario: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([
            "--ignored",
            "--exact",
            "crash_child",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CRASH_ENV, format!("{which}:{scenario}"))
        // Keep the child from spewing a backtrace/abort message.
        .env("RUST_BACKTRACE", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("spawn crash child");
    if let Some(sig) = out.status.signal() {
        Outcome::Signal(sig)
    } else {
        Outcome::Exit(out.status.code().unwrap_or(-1))
    }
}

fn assert_null_case_matches(row: &str, scenario: &str) {
    const SIGSEGV: i32 = 11;
    const SIGBUS: i32 = 7;
    let c = run_crash_child("c", scenario);
    assert!(
        c == Outcome::Signal(SIGSEGV) || c == Outcome::Signal(SIGBUS),
        "[{row}] expected the C to fault on a null pointer (it has no null check), got {c:?}"
    );
    for r in rust_libs() {
        let got = run_crash_child(&r.name, scenario);
        assert_eq!(
            got, c,
            "[{row}] {} did not fault the same way as the C for scenario `{scenario}`",
            r.name
        );
    }
}

#[test]
fn e11_null_src_faults_in_both() {
    assert_null_case_matches("E11", "null_src");
}

#[test]
fn e12_null_dest_faults_in_both() {
    assert_null_case_matches("E12", "null_dest");
}

#[test]
fn e13_null_dest_zero_sat_faults_in_both() {
    assert_null_case_matches("E13", "null_dest_zero_sat");
    assert_null_case_matches("E13 both null", "both_null");
}

// ---------------------------------------------------------------------------
// E14 / E15 — exactly three words read and exactly three written
// ---------------------------------------------------------------------------

#[test]
fn e14_no_out_of_bounds_write() {
    // A generous buffer with canaries on both sides: `assert_same_layout`
    // compares the *whole* allocation, so any extra store diverges (and the
    // explicit canary check below pins the expected C behaviour).
    let lay = Layout::new(24, 0, 10);
    let mut rng = Rng::new(0xE014);
    for _ in 0..20_000 {
        let h = if rng.below(2) == 0 {
            random_hue_any_sector(&mut rng)
        } else {
            rng.bits_f32()
        };
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same_layout("E14 canaries", lay, h, s, l);

        let buf = run_layout(c_lib(), lay, h, s, l);
        for (i, &w) in buf.iter().enumerate() {
            let touched = (lay.src_off..lay.src_off + 3).contains(&i)
                || (lay.dst_off..lay.dst_off + 3).contains(&i);
            if !touched {
                assert_eq!(w, CANARY, "C clobbered word {i} outside src/dest");
            }
        }
    }
    // Same with the early-return path, which stores through a different code path.
    for _ in 0..5_000 {
        let h = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same_layout("E14 canaries, s=0", lay, h, 0.0, l);
    }
}

#[test]
fn e15_tight_buffer_no_overrun() {
    // Exactly six words exist: 3 for src, 3 for dest, no slack at all. If either
    // library read or wrote a fourth word it would be reading the other array,
    // which would show up as a divergence and/or a corrupted input.
    let mut rng = Rng::new(0xE015);
    for &lay in &[Layout::new(6, 0, 3), Layout::new(6, 3, 0), Layout::new(3, 0, 0)] {
        for _ in 0..20_000 {
            let h = if rng.below(2) == 0 {
                random_hue_any_sector(&mut rng)
            } else {
                rng.bits_f32()
            };
            let s = rng.bits_f32();
            let l = rng.bits_f32();
            assert_same_layout("E15 tight buffer", lay, h, s, l);
        }
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — aliasing: all reads precede all writes in the C
// ---------------------------------------------------------------------------

#[test]
fn e16_full_aliasing_dest_eq_src() {
    let lay = Layout::new(9, 3, 3);
    let mut rng = Rng::new(0xE016);
    for _ in 0..20_000 {
        let h = if rng.below(2) == 0 {
            random_hue_any_sector(&mut rng)
        } else {
            rng.bits_f32()
        };
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same_layout("E16 dest==src", lay, h, s, l);

        // Aliasing must not change the answer: the C caches h/s/l first.
        let aliased = run_layout(c_lib(), lay, h, s, l);
        let disjoint = run(c_lib(), h, s, l);
        assert_eq!(
            [aliased[3], aliased[4], aliased[5]],
            disjoint,
            "aliased result differs from the disjoint result"
        );
    }
    for &h in &specials_and_nans() {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same_layout("E16 specials", lay, h, s, l);
            }
        }
    }
}

#[test]
fn e17_partial_overlap() {
    let mut rng = Rng::new(0xE017);
    for (name, lay) in [
        ("dest==src+1", Layout::new(9, 3, 4)),
        ("dest==src-1", Layout::new(9, 3, 2)),
        ("dest==src+2", Layout::new(9, 3, 5)),
        ("dest==src-2", Layout::new(9, 3, 1)),
    ] {
        for _ in 0..20_000 {
            let h = if rng.below(2) == 0 {
                random_hue_any_sector(&mut rng)
            } else {
                rng.bits_f32()
            };
            let s = rng.bits_f32();
            let l = rng.bits_f32();
            assert_same_layout(&format!("E17 {name}"), lay, h, s, l);

            // The three written words must equal the disjoint answer, proving
            // no read happened after a write.
            let over = run_layout(c_lib(), lay, h, s, l);
            let disjoint = run(c_lib(), h, s, l);
            assert_eq!(
                [over[lay.dst_off], over[lay.dst_off + 1], over[lay.dst_off + 2]],
                disjoint,
                "[{name}] overlapping result differs from the disjoint result"
            );
        }
        for &h in &specials_and_nans() {
            for &s in &specials_and_nans() {
                for &l in &specials_and_nans() {
                    assert_same_layout(&format!("E17 {name} specials"), lay, h, s, l);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E18 — signalling NaNs: quieted by arithmetic, NOT quieted when merely copied
// ---------------------------------------------------------------------------

#[test]
fn e18_signalling_nan() {
    let snans: Vec<f32> = [0x7f80_0001u32, 0xff80_0001, 0x7fbf_ffff, 0xffbf_ffff]
        .iter()
        .map(|&b| f32::from_bits(b))
        .collect();

    // (a) A signalling NaN that is only *copied* (the s == 0 early return) must
    //     come out unchanged - still signalling.
    for &l in &snans {
        let out = run(c_lib(), 30.0, 0.0, l);
        assert_eq!(
            out,
            [l.to_bits(); 3],
            "the early return must copy a signalling NaN verbatim"
        );
        assert_same("E18 sNaN copied", 30.0, 0.0, l);
    }

    // (b) A signalling NaN that goes through arithmetic must come out quieted,
    //     with sign and payload preserved.
    for &l in &snans {
        let out = run(c_lib(), 30.0, 1.0, l);
        let m = out[2];
        assert!(f32::from_bits(m).is_nan());
        assert_eq!(
            m,
            l.to_bits() | 0x0040_0000,
            "arithmetic must quiet the sNaN while keeping sign+payload"
        );
        assert_same("E18 sNaN through arithmetic", 30.0, 1.0, l);
    }

    // (c) Full differential sweep: every sNaN in every position, crossed with
    //     the whole special pool.
    let pool = specials_and_nans();
    for &n in &snans {
        for &a in &pool {
            for &b in &pool {
                assert_same("E18 sNaN as h", n, a, b);
                assert_same("E18 sNaN as s", a, n, b);
                assert_same("E18 sNaN as l", a, b, n);
            }
        }
        for &n2 in &snans {
            for &a in &pool {
                assert_same("E18 two sNaNs (h,s)", n, n2, a);
                assert_same("E18 two sNaNs (h,l)", n, a, n2);
                assert_same("E18 two sNaNs (s,l)", a, n, n2);
            }
            for &n3 in &snans {
                assert_same("E18 three sNaNs", n, n2, n3);
            }
        }
    }

    // (d) Random NaNs with arbitrary payloads in every position.
    let mut rng = Rng::new(0xE018);
    for _ in 0..50_000 {
        let mk_nan = |r: &mut Rng| {
            let sign = (r.next_u32() & 1) << 31;
            let payload = r.next_u32() & 0x007f_ffff;
            let payload = if payload == 0 { 1 } else { payload };
            f32::from_bits(sign | 0x7f80_0000 | payload)
        };
        let h = if rng.below(2) == 0 { mk_nan(&mut rng) } else { rng.bits_f32() };
        let s = if rng.below(2) == 0 { mk_nan(&mut rng) } else { rng.bits_f32() };
        let l = if rng.below(2) == 0 { mk_nan(&mut rng) } else { rng.bits_f32() };
        assert_same("E18 random NaN payloads", h, s, l);
    }
}

// ---------------------------------------------------------------------------
// E19 — subnormals and the extreme finite values; no flush-to-zero
// ---------------------------------------------------------------------------

#[test]
fn e19_subnormals() {
    let tiny: Vec<f32> = [
        0x0000_0001u32, // FLT_TRUE_MIN
        0x8000_0001,
        0x0000_0002,
        0x0040_0000, // mid subnormal
        0x007f_ffff, // largest subnormal
        0x807f_ffff,
        0x0080_0000, // FLT_MIN (smallest normal)
        0x8080_0000,
        0x7f7f_ffff, // FLT_MAX
        0xff7f_ffff,
    ]
    .iter()
    .map(|&b| f32::from_bits(b))
    .collect();

    // Pin: no flush-to-zero. With s = FLT_TRUE_MIN and l = 0.5, c = s exactly.
    let out = run(c_lib(), 30.0, f32::from_bits(1), 0.5);
    assert_eq!(
        f32::from_bits(out[0]).to_bits(),
        (0.5f32 + f32::from_bits(1) - 0.5 * f32::from_bits(1)).to_bits(),
        "subnormal saturation must not be flushed to zero"
    );

    for &a in &tiny {
        for &b in &tiny {
            for &c in &tiny {
                assert_same("E19 tiny^3", a, b, c);
            }
        }
        for &b in &specials_and_nans() {
            for &c in &specials_and_nans() {
                assert_same("E19 tiny as h", a, b, c);
                assert_same("E19 tiny as s", b, a, c);
                assert_same("E19 tiny as l", b, c, a);
            }
        }
        for &h in HUE_BOUNDARIES {
            assert_same("E19 boundary hue, tiny s", h, a, 0.5);
            assert_same("E19 boundary hue, tiny l", h, 0.5, a);
        }
    }

    let mut rng = Rng::new(0xE019);
    for _ in 0..50_000 {
        let pick = |r: &mut Rng| {
            if r.below(2) == 0 {
                tiny[r.below(tiny.len() as u32) as usize]
            } else {
                r.log_uniform()
            }
        };
        let h = pick(&mut rng);
        let s = pick(&mut rng);
        let l = pick(&mut rng);
        assert_same("E19 tiny fuzz", h, s, l);
    }
}

// ---------------------------------------------------------------------------
// E20 — catch-all: the whole 32-bit input space, independently per component
// ---------------------------------------------------------------------------

#[test]
fn e20_full_bit_pattern_fuzz() {
    let mut rng = Rng::new(0xE020);
    for _ in 0..300_000 {
        let h = rng.bits_f32();
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same("E20 whole-space fuzz", h, s, l);
    }
    // Second pass biased towards the exponent extremes, where the fully uniform
    // draw above almost never lands.
    for _ in 0..300_000 {
        let h = rng.log_uniform();
        let s = rng.log_uniform();
        let l = rng.log_uniform();
        assert_same("E20 log-uniform fuzz", h, s, l);
    }
    // Third pass: every layout, so the fuzz also covers aliasing.
    for _ in 0..50_000 {
        let lay = Layout::new(9, 3, rng.below(7) as usize);
        let h = rng.bits_f32();
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same_layout("E20 fuzz x layouts", lay, h, s, l);
    }
}
