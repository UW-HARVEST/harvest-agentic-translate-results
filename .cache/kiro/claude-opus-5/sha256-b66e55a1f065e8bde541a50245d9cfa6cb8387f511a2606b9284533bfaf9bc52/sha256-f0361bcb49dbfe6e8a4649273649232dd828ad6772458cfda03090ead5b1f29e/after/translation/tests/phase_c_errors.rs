//! Phase C — error-path differential tests, gated on `ERRORS.md`.
//!
//! `hsl_to_rgb` is `void` and performs no validation, so there is no error code
//! to compare. The only observables are (a) the exact bit pattern written to
//! `dest[0..3]`, (b) whether `dest` was written at all, and (c) for the
//! null-pointer rows, the signal that kills the process. Every row asserts on
//! whichever of those the C actually exposes.
//!
//! Checklist at the bottom of this file mirrors `ERRORS.md`.

mod common;

use common::*;

const N: usize = 20_000;

/// The poison value the harness pre-fills `dest` with.
const POISON: u32 = 0xDEAD_BEEF;

// --- Row 1: s == +0.0 -----------------------------------------------------

#[test]
fn err01_saturation_positive_zero_rejects_via_early_return() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let l = rng.any_f32();
        let h = rng.any_f32();
        let src = [h, 0.0f32, l];
        assert_same(&p, "err01", src);
        // The rejection is observable: all three lanes are exactly `l`,
        // bit-for-bit, and `c`/`m`/`x` were never involved.
        let c = p.call_c(src);
        assert_eq!(
            c,
            [l.to_bits(); 3],
            "err01: C did not take the early return for {}",
            show_in(src)
        );
        assert_eq!(p.call_rust(src), [l.to_bits(); 3], "err01 Rust");
    }
}

// --- Row 2: s == -0.0 -----------------------------------------------------

#[test]
fn err02_saturation_negative_zero_also_rejects() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let l = rng.any_f32();
        let src = [rng.any_f32(), -0.0f32, l];
        assert_same(&p, "err02", src);
        assert_eq!(
            p.call_c(src),
            [l.to_bits(); 3],
            "err02: -0.0 must compare equal to 0 in C for {}",
            show_in(src)
        );
        assert_eq!(p.call_rust(src), [l.to_bits(); 3], "err02 Rust");
    }
}

// --- Row 3: h in [120,180) falls through every predicate ------------------

#[test]
fn err03_hue_120_to_180_is_rejected_by_all_arms() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let h = rng.range(120.0, 180.0);
        let mut s = rng.range(0.0, 1.0);
        if s == 0.0 {
            s = 1.0;
        }
        let l = rng.range(0.0, 1.0);
        let src = [h, s, l];
        assert_same(&p, "err03", src);
        // Sentinel for the "flat grey" fallback: all three lanes equal.
        let c = p.call_c(src);
        assert!(
            c[0] == c[1] && c[1] == c[2],
            "err03: expected flat grey (m,m,m) from the final else, got {} for {}",
            show3(c),
            show_in(src)
        );
    }
    // exact lower edge and the last f32 below 180
    for &h in [120.0f32, 179.999_99, 150.0].iter() {
        let src = [h, 0.75, 0.4];
        assert_same(&p, "err03 edge", src);
        let c = p.call_c(src);
        assert!(c[0] == c[1] && c[1] == c[2], "err03 edge not grey at h={h}");
    }
}

// --- Row 4: h >= 360 ------------------------------------------------------

#[test]
fn err04_hue_at_or_past_360_is_rejected() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let h = rng.range(360.0, 1.0e9);
        let mut s = rng.range(0.0, 1.0);
        if s == 0.0 {
            s = 1.0;
        }
        let src = [h, s, rng.range(0.0, 1.0)];
        assert_same(&p, "err04", src);
        let c = p.call_c(src);
        assert!(
            c[0] == c[1] && c[1] == c[2],
            "err04: expected the else fallback for {}",
            show_in(src)
        );
    }
    assert_same(&p, "err04 exact 360", [360.0, 0.8, 0.3]);
    assert_same(&p, "err04 huge", [f32::MAX, 0.8, 0.3]);
}

// --- Row 5: h == +inf -----------------------------------------------------

#[test]
fn err05_hue_positive_infinity_is_rejected() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let mut s = rng.any_f32();
        if s == 0.0 {
            s = 1.0;
        }
        let src = [f32::INFINITY, s, rng.any_f32()];
        assert_same(&p, "err05", src);
    }
    // With finite s,l the fallback is the flat grey `m`.
    let src = [f32::INFINITY, 0.6, 0.4];
    let c = p.call_c(src);
    assert_eq!(c[0], c[1]);
    assert_eq!(c[1], c[2]);
    assert_same(&p, "err05 canonical", src);
}

// --- Row 6: h == NaN ------------------------------------------------------

#[test]
fn err06_hue_nan_is_rejected_by_unordered_compare() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for neg in [false, true] {
        for _ in 0..N / 2 {
            let h = rng.nan(neg);
            let s = rng.range(0.001, 2.0);
            let l = rng.range(0.0, 1.0);
            let src = [h, s, l];
            assert_same(&p, "err06", src);
            let c = p.call_c(src);
            assert!(
                c[0] == c[1] && c[1] == c[2],
                "err06: NaN hue must reach the final else, got {} for {}",
                show3(c),
                show_in(src)
            );
        }
    }
    // Canonical quiet NaN, both signs.
    for h in [f32::NAN, -f32::NAN] {
        assert_same(&p, "err06 canonical NaN", [h, 0.7, 0.35]);
    }
}

// --- Row 7: h < 0 is captured by the buggy arm 3 --------------------------

#[test]
fn err07_negative_hue_lands_in_buggy_arm3_not_the_else() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let h = -rng.range(1.0e-3, 1.0e9);
        let s = rng.range(0.001, 1.0);
        let l = rng.range(0.05, 0.95);
        let src = [h, s, l];
        assert_same(&p, "err07", src);
    }
    // Prove it is arm 3, not the else: with s,l chosen so c != 0 the three
    // lanes are NOT all equal (the else would make them equal).
    let src = [-30.0f32, 1.0, 0.5];
    let c = p.call_c(src);
    assert!(
        !(c[0] == c[1] && c[1] == c[2]),
        "err07: negative hue must hit arm 3 (m, c+m, x+m), got flat {}",
        show3(c)
    );
    assert_eq!(p.call_rust(src), c, "err07 Rust must agree");
    // And the smallest negative f32 magnitudes.
    for h in [-0.0f32, -1.0e-45, -f32::MIN_POSITIVE, -f32::EPSILON] {
        assert_same(&p, "err07 tiny negative", [h, 1.0, 0.5]);
    }
}

// --- Row 8: h == -inf -----------------------------------------------------

#[test]
fn err08_hue_negative_infinity() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let mut s = rng.any_f32();
        if s == 0.0 {
            s = 1.0;
        }
        assert_same(&p, "err08", [f32::NEG_INFINITY, s, rng.any_f32()]);
    }
    // fmodf(-inf, 2) is NaN, so x is NaN and lane 1/2 must be NaN in both.
    let src = [f32::NEG_INFINITY, 1.0f32, 0.5f32];
    let c = p.call_c(src);
    assert!(
        f32::from_bits(c[2]).is_nan(),
        "err08: expected NaN in dest[2] from fmodf(-inf,2), got {}",
        show(c[2])
    );
    assert_eq!(p.call_rust(src), c, "err08 Rust must agree");
}

// --- Row 9: h exactly 120 -------------------------------------------------

#[test]
fn err09_hue_exactly_120() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let s = rng.range(0.001, 2.0);
        let l = rng.range(-2.0, 2.0);
        assert_same(&p, "err09", [120.0, s, l]);
    }
    let src = [120.0f32, 1.0, 0.5];
    let c = p.call_c(src);
    assert!(
        c[0] == c[1] && c[1] == c[2],
        "err09: h == 120 must fall to the else, got {}",
        show3(c)
    );
}

// --- Row 10: lower-inclusive edges 180 / 240 / 300 are ACCEPTED -----------

#[test]
fn err10_lower_inclusive_edges_are_accepted() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for &h in [180.0f32, 240.0, 300.0].iter() {
        for _ in 0..N / 3 {
            let s = rng.range(0.001, 2.0);
            let l = rng.range(-2.0, 2.0);
            assert_same(&p, "err10", [h, s, l]);
        }
        // c != 0 => not the flat-grey else, i.e. the edge was accepted.
        let src = [h, 1.0f32, 0.5f32];
        let c = p.call_c(src);
        assert!(
            !(c[0] == c[1] && c[1] == c[2]),
            "err10: h == {h} must be accepted by its arm, got flat {}",
            show3(c)
        );
        assert_eq!(p.call_rust(src), c, "err10 Rust must agree at h={h}");
    }
    // 60 is also lower-inclusive (arm 2) and 0 for arm 1.
    for &h in [0.0f32, 60.0].iter() {
        assert_same(&p, "err10 low edges", [h, 1.0, 0.5]);
    }
}

// --- Row 11: h exactly 360 is REJECTED -----------------------------------

#[test]
fn err11_hue_exactly_360_is_rejected() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let s = rng.range(0.001, 2.0);
        let l = rng.range(-2.0, 2.0);
        assert_same(&p, "err11", [360.0, s, l]);
    }
    let src = [360.0f32, 1.0, 0.5];
    let c = p.call_c(src);
    assert!(
        c[0] == c[1] && c[1] == c[2],
        "err11: h == 360 is past the last sector and must hit the else, got {}",
        show3(c)
    );
    // one step below 360 IS accepted
    let below = f32::from_bits(360.0f32.to_bits() - 1);
    let cb = p.call_c([below, 1.0, 0.5]);
    assert!(
        !(cb[0] == cb[1] && cb[1] == cb[2]),
        "err11: nextafter(360,-inf) must still be in arm 6"
    );
    assert_same(&p, "err11 just below 360", [below, 1.0, 0.5]);
}

// --- Row 12: s == NaN is NOT rejected ------------------------------------

#[test]
fn err12_saturation_nan_is_not_rejected() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for neg in [false, true] {
        for _ in 0..N / 2 {
            let s = rng.nan(neg);
            let h = rng.range(-400.0, 400.0);
            let l = rng.range(-2.0, 2.0);
            let src = [h, s, l];
            assert_same(&p, "err12", src);
            // `s == 0` is false for NaN, so the early return was NOT taken:
            // the output must be NaN, not a copy of `l`.
            let c = p.call_c(src);
            assert!(
                f32::from_bits(c[0]).is_nan(),
                "err12: NaN saturation must flow into the arithmetic, got {} for {}",
                show3(c),
                show_in(src)
            );
        }
    }
}

// --- Row 13: unclamped / out-of-domain l and s ---------------------------

#[test]
fn err13_no_clamping_of_lightness_or_saturation() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let classes: [fn(&mut Rng) -> f32; 7] = [
        |r| r.range(-1.0e6, -1.0e-6),
        |r| r.range(1.0 + 1.0e-6, 1.0e6),
        |_| f32::INFINITY,
        |_| f32::NEG_INFINITY,
        |_| 1.0e-45,
        |_| -1.0e-45,
        |r| {
            let neg = r.next_u64() & 1 == 1;
            r.nan(neg)
        },
    ];
    for _ in 0..N {
        let h = rng.range(-400.0, 400.0);
        let si = (rng.next_u32() as usize) % classes.len();
        let li = (rng.next_u32() as usize) % classes.len();
        let s = classes[si](&mut rng);
        let l = classes[li](&mut rng);
        assert_same(&p, "err13", [h, s, l]);
    }
    // Demonstrate the absence of clamping: l > 1 with s = 1 gives c < 0 and at
    // least one output lane outside [0,1] in BOTH implementations.
    // (h=30, s=1, l=5  ->  c = -8, m = 9, x = -4  ->  arm 1 = (1, 5, 9))
    let src = [30.0f32, 1.0, 5.0];
    let c = p.call_c(src);
    assert!(
        c.iter()
            .any(|&b| f32::from_bits(b) < 0.0 || f32::from_bits(b) > 1.0),
        "err13: expected an unclamped result, got {}",
        show3(c)
    );
    assert_eq!(p.call_rust(src), c);
}

// --- Row 14: null pointers (undefined behaviour -> same signal) ----------

/// Child process body for the null-pointer rows. Selected by `HSL_NULL_CASE`.
#[test]
#[ignore = "child process helper for err14"]
fn err14_null_child() {
    let case = std::env::var("HSL_NULL_CASE").expect("HSL_NULL_CASE unset");
    let p = Pair::load();
    let f = if case.starts_with("c-") { p.c } else { p.rust };
    let src = [30.0f32, 0.7, 0.4];
    let mut dest = [0.0f32; 3];
    // Flush so the parent can tell "reached the call" from "failed to load".
    println!("about to call: {case}");
    use std::io::Write;
    std::io::stdout().flush().ok();
    unsafe {
        match case.as_str() {
            "c-dest" | "rust-dest" => f(std::ptr::null_mut(), src.as_ptr()),
            "c-src" | "rust-src" => f(dest.as_mut_ptr(), std::ptr::null()),
            "c-both" | "rust-both" => f(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("unknown case {other}"),
        }
    }
    // If we get here the pointer was somehow tolerated; report it.
    println!("survived: {case} -> {:?}", dest);
}

#[test]
fn err14_null_pointers_fault_identically() {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let run = |case: &str| -> (Option<i32>, Option<i32>) {
        let out = std::process::Command::new(&exe)
            .args(["err14_null_child", "--ignored", "--exact", "--nocapture"])
            .env("HSL_NULL_CASE", case)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        (out.status.code(), out.status.signal())
    };

    for kind in ["dest", "src", "both"] {
        let c = run(&format!("c-{kind}"));
        let r = run(&format!("rust-{kind}"));
        assert_eq!(
            c, r,
            "err14 ({kind}): C exited {c:?} but Rust exited {r:?}; \
             they must fail the same way"
        );
        // And it really is a fault, not a graceful exit.
        assert_eq!(
            c.1,
            Some(11),
            "err14 ({kind}): expected SIGSEGV from the C, got {c:?}"
        );
    }
}

// --- Row 15: exactly three floats read / written -------------------------

#[test]
fn err15_reads_and_writes_exactly_three_floats() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let src3 = [rng.any_f32(), rng.any_f32(), rng.any_f32()];

        // Vary everything past src[2]; output must be unchanged.
        let mk = |tail: u32| {
            let mut a = [0f32; 8];
            a[..3].copy_from_slice(&src3);
            for k in 3..8 {
                a[k] = f32::from_bits(tail.wrapping_mul(k as u32 + 1));
            }
            a
        };
        let a = mk(0x0000_0000);
        let b = mk(0x9E37_79B9);

        let call = |f: HslToRgb, input: &[f32; 8]| -> ([u32; 3], [u32; 5]) {
            let mut dest = [f32::from_bits(POISON); 8];
            unsafe { f(dest.as_mut_ptr(), input.as_ptr()) };
            (
                [dest[0].to_bits(), dest[1].to_bits(), dest[2].to_bits()],
                [
                    dest[3].to_bits(),
                    dest[4].to_bits(),
                    dest[5].to_bits(),
                    dest[6].to_bits(),
                    dest[7].to_bits(),
                ],
            )
        };

        let (ca, cga) = call(p.c, &a);
        let (cb, cgb) = call(p.c, &b);
        let (ra, rga) = call(p.rust, &a);
        let (rb, rgb) = call(p.rust, &b);

        assert_eq!(ca, cb, "err15: C read past src[2] for {}", show_in(src3));
        assert_eq!(ra, rb, "err15: Rust read past src[2] for {}", show_in(src3));
        assert_eq!(ca, ra, "err15 divergence for {}", show_in(src3));
        assert_eq!(cb, rb, "err15 divergence (b) for {}", show_in(src3));
        for g in [cga, cgb, rga, rgb] {
            assert_eq!(
                g,
                [POISON; 5],
                "err15: wrote past dest[2] for {}",
                show_in(src3)
            );
        }
    }
}

// --- Row 16: dest == src -------------------------------------------------

#[test]
fn err16_full_aliasing() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let src = if rng.bool() {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        } else {
            [rng.range(-400.0, 400.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)]
        };
        let alias = |f: HslToRgb| -> [u32; 3] {
            let mut buf = src;
            unsafe { f(buf.as_mut_ptr(), buf.as_ptr()) };
            [buf[0].to_bits(), buf[1].to_bits(), buf[2].to_bits()]
        };
        let c = alias(p.c);
        let r = alias(p.rust);
        assert_eq!(c, r, "err16 aliased divergence for {}", show_in(src));
        // Aliasing must not change the answer versus disjoint buffers.
        assert_eq!(
            c,
            p.call_c(src),
            "err16: C's aliased result differs from its disjoint result for {}",
            show_in(src)
        );
    }
}

// --- Row 17: shifted overlap --------------------------------------------

#[test]
fn err17_shifted_overlap() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let src = if rng.bool() {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        } else {
            [rng.range(-400.0, 400.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)]
        };
        for delta in [-2i32, -1, 1, 2] {
            let run = |f: HslToRgb| -> Vec<u32> {
                let mut buf = [f32::from_bits(POISON); 10];
                let s_off = 4usize;
                buf[s_off] = src[0];
                buf[s_off + 1] = src[1];
                buf[s_off + 2] = src[2];
                let d_off = (s_off as i32 + delta) as usize;
                unsafe { f(buf.as_mut_ptr().add(d_off), buf.as_ptr().add(s_off)) };
                buf.iter().map(|v| v.to_bits()).collect()
            };
            assert_eq!(
                run(p.c),
                run(p.rust),
                "err17 (delta={delta}) divergence for {}",
                show_in(src)
            );
        }
    }
}

// --- Generic FFI boundaries the task requires regardless of the table ----

#[test]
fn err18_no_enums_exist_so_no_invalid_enum_value_is_possible() {
    // Documented negative result: the entire public surface is
    //   void hsl_to_rgb(float *dest, const float *src);
    // There is no enum, no int flag, and no mode parameter, so there is no
    // out-of-range enum value to smuggle across the FFI boundary. The nearest
    // equivalent — an f32 bit pattern with no meaningful interpretation — is
    // covered exhaustively by row 27 of CONFIGS.md and by the class sweep here.
    let p = Pair::load();
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read lib.h");
    assert!(
        !header.contains("enum"),
        "lib.h grew an enum; ERRORS.md must be regenerated"
    );

    // Every IEEE-754 class in every argument position.
    let reps = [
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-45,
        -1.0e-45,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0001),
    ];
    for &h in reps.iter() {
        for &s in reps.iter() {
            for &l in reps.iter() {
                assert_same(&p, "err18 class cross-product", [h, s, l]);
            }
        }
    }
}

#[test]
fn err19_zero_and_oversized_lengths_have_no_parameter_to_abuse() {
    // There is no length/count argument anywhere in the API, so "zero length"
    // and "oversized length" reduce to buffer geometry, which err15/16/17 cover.
    // What remains testable is that a *minimally sized* allocation (exactly
    // 3 floats, nothing after it) works in both — i.e. neither over-reads.
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let src: Box<[f32; 3]> = Box::new([rng.any_f32(), rng.any_f32(), rng.any_f32()]);
        let mut cd: Box<[f32; 3]> = Box::new([f32::from_bits(POISON); 3]);
        let mut rd: Box<[f32; 3]> = Box::new([f32::from_bits(POISON); 3]);
        unsafe {
            (p.c)(cd.as_mut_ptr(), src.as_ptr());
            (p.rust)(rd.as_mut_ptr(), src.as_ptr());
        }
        let cb: Vec<u32> = cd.iter().map(|v| v.to_bits()).collect();
        let rb: Vec<u32> = rd.iter().map(|v| v.to_bits()).collect();
        assert_eq!(cb, rb, "err19 divergence for {}", show_in(*src));
    }
}

#[test]
fn err20_one_step_past_every_documented_boundary() {
    // Every literal the C compares against, plus one f32 step either side.
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let step_up = |x: f32| {
        if x == 0.0 {
            f32::from_bits(1)
        } else if x.to_bits() >> 31 == 0 {
            f32::from_bits(x.to_bits() + 1)
        } else {
            f32::from_bits(x.to_bits() - 1)
        }
    };
    let step_down = |x: f32| -step_up(-x);
    let mut hs: Vec<f32> = Vec::new();
    for &e in SECTOR_EDGES.iter() {
        hs.extend_from_slice(&[step_down(e), e, step_up(e)]);
    }
    // s and l are compared only against 0 (and used in 0.5/1.0/2.0 arithmetic).
    let sl: Vec<f32> = [0.0f32, 1.0, 0.5]
        .iter()
        .flat_map(|&v| vec![step_down(v), v, step_up(v)])
        .collect();
    for &h in &hs {
        for &s in &sl {
            for &l in &sl {
                assert_same(&p, "err20 boundary+-1ulp", [h, s, l]);
            }
        }
        for _ in 0..50 {
            assert_same(
                &p,
                "err20 boundary x random",
                [h, rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md checklist — every row has a test above, all passing.
//
//  [x]  1  s == +0.0 early return            -> err01
//  [x]  2  s == -0.0 early return            -> err02
//  [x]  3  h in [120,180) dead range         -> err03
//  [x]  4  h >= 360                          -> err04
//  [x]  5  h == +inf                         -> err05
//  [x]  6  h == NaN                          -> err06
//  [x]  7  h < 0 captured by buggy arm 3     -> err07
//  [x]  8  h == -inf                         -> err08
//  [x]  9  h == 120 exactly                  -> err09
//  [x] 10  h == 180 / 240 / 300 accepted     -> err10
//  [x] 11  h == 360 exactly rejected         -> err11
//  [x] 12  s == NaN not rejected             -> err12
//  [x] 13  unclamped l / s                   -> err13
//  [x] 14  NULL dest / src -> same SIGSEGV   -> err14 (+ err14_null_child)
//  [x] 15  exactly src[0..3] / dest[0..3]    -> err15
//  [x] 16  dest == src                       -> err16
//  [x] 17  dest == src +- 1/2                -> err17
//       plus generic FFI boundaries:
//  [x]     no enum exists / all f32 classes  -> err18
//  [x]     no length parameter exists        -> err19
//  [x]     +-1 ulp past every boundary       -> err20
// ---------------------------------------------------------------------------
