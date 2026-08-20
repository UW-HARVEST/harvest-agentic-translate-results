//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported `hsl_to_rgb`
//! symbol and compares the three output components as raw `u32` bit patterns,
//! so `+0.0` vs `-0.0` and NaN sign/payload differences are caught.

mod common;

use common::*;

/// How many randomized inputs each ordinary row uses.
const N: usize = 4000;

// ---------------------------------------------------------------------------
// Phase D — symbol parity (kept next to the tests so it runs on every `cargo test`)
// ---------------------------------------------------------------------------

mod symbols {
    use super::*;
    use std::process::Command;

    fn nm(args: &[&str], path: &std::path::Path) -> String {
        let out = Command::new("nm")
            .args(args)
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Global symbols defined in the dynamic symbol table.
    fn exported(path: &std::path::Path) -> Vec<String> {
        let mut v: Vec<String> = nm(&["-D", "--defined-only"], path)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (a, b) = (it.next()?, it.next()?);
                // "<addr> T name" for globals, "<addr> t name" / "w name" otherwise
                let (kind, name) = match it.next() {
                    Some(n) => (b, n),
                    None => (a, b),
                };
                // Uppercase type letters are global/exported symbols.
                if kind.len() == 1 && kind.chars().next().unwrap().is_ascii_uppercase() {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn symbol_parity_c_exports_are_all_exported_by_rust() {
        let h = harness();
        let c = exported(&h.c_so);
        let r = exported(&h.rust_so);
        assert!(
            c.contains(&"hsl_to_rgb".to_string()),
            "the C .so must export hsl_to_rgb, got {c:?}"
        );
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
             C: {c:?}\nRust: {r:?}"
        );
        let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
        assert!(
            extra.is_empty(),
            "the Rust .so exports symbols the C .so does not: {extra:?}"
        );
    }

    /// `fmodf` must be *imported* from libm (as the C `.so` does), not satisfied
    /// by `compiler_builtins`' statically linked copy, so that the exceptional
    /// paths (`fmodf(±inf, 2.0f)`) cannot diverge.
    #[test]
    fn rust_so_imports_fmodf_from_libm() {
        let h = harness();
        let undef = nm(&["-D", "--undefined-only"], &h.rust_so);
        assert!(
            undef.lines().any(|l| l.split_whitespace().last() == Some("fmodf")
                || l.contains("fmodf@")),
            "the Rust .so must import fmodf dynamically, undefined syms:\n{undef}"
        );
        let all = nm(&[], &h.rust_so);
        let local_def = all.lines().any(|l| {
            let mut it = l.split_whitespace();
            match (it.next(), it.next(), it.next()) {
                (Some(_), Some(k), Some(n)) => n == "fmodf" && k != "U",
                _ => false,
            }
        });
        assert!(
            !local_def,
            "a local definition of fmodf is shadowing libm's in the Rust .so:\n{all}"
        );
    }

    #[test]
    fn both_libraries_expose_the_symbol_via_dlsym() {
        let h = harness();
        // `harness()` already resolved `hsl_to_rgb` in both objects; make the
        // dependency explicit and verify the pointers really are distinct code.
        assert!(!h.rust.is_empty());
        for (label, f) in &h.rust {
            assert!(
                *f as usize != h.c as usize,
                "{label} resolved to the same address as the C implementation"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 1-6 — the six hue sectors of the if/else chain
// ---------------------------------------------------------------------------

fn sector_row(idx: usize, seed: u64) {
    let (label, lo, hi) = SECTORS[idx];
    let mut rng = Rng::new(seed);
    let inputs = (0..N).map(|_| {
        [
            rng.range(lo, hi).to_bits(),
            rng.range(f32::from_bits(1), 1.0).to_bits(), // s in (0, 1]
            rng.range(f32::from_bits(1), 1.0).to_bits(), // l in (0, 1)
        ]
    });
    assert_same_all(label, inputs.collect::<Vec<_>>());
}

#[test]
fn row01_sector_0_60() {
    sector_row(0, 0x0101);
}

#[test]
fn row02_sector_60_120() {
    sector_row(1, 0x0202);
}

#[test]
fn row03_sector_120_180_falls_through_to_else() {
    sector_row(2, 0x0303);
    // The C guard is `h < 120.0f && h < 180.0f`, so 120..180 matches no branch
    // and the final `else` writes (m, m, m). Pin that behaviour down explicitly.
    let h = harness();
    let mut rng = Rng::new(0x3333);
    for _ in 0..500 {
        let src = [
            rng.range(120.0, 180.0).to_bits(),
            rng.range(0.01, 1.0).to_bits(),
            rng.range(0.01, 0.99).to_bits(),
        ];
        let c = call(h.c, src);
        assert_eq!(
            c.rgb[0], c.rgb[1],
            "h in [120,180) must reach the final else (r == g == b == m), got {:#x?}",
            c.rgb
        );
        assert_eq!(c.rgb[1], c.rgb[2]);
        assert_same("h[120,180) == else", src);
    }
}

#[test]
fn row04_sector_180_240() {
    sector_row(3, 0x0404);
}

#[test]
fn row05_sector_240_300() {
    sector_row(4, 0x0505);
}

#[test]
fn row06_sector_300_360() {
    sector_row(5, 0x0606);
}

// ---------------------------------------------------------------------------
// Row 7 — h >= 360 (no wrap-around)
// ---------------------------------------------------------------------------

#[test]
fn row07_hue_at_or_above_360() {
    let mut rng = Rng::new(0x0707);
    let mut inputs = Vec::new();
    for _ in 0..N {
        inputs.push([
            rng.range(360.0, 1e9).to_bits(),
            rng.range(f32::from_bits(1), 1.0).to_bits(),
            rng.range(0.0, 1.0).to_bits(),
        ]);
    }
    for _ in 0..N {
        inputs.push([
            rng.range(1e9, f32::MAX).to_bits(),
            rng.range(f32::from_bits(1), 1.0).to_bits(),
            rng.range(0.0, 1.0).to_bits(),
        ]);
    }
    assert_same_all("h >= 360", inputs.clone());

    // Must land in the final else => all three components equal `m`.
    let h = harness();
    for src in inputs.iter().take(200) {
        let c = call(h.c, *src);
        assert_eq!(c.rgb[0], c.rgb[1], "h >= 360 must reach the final else");
        assert_eq!(c.rgb[1], c.rgb[2]);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — strictly negative hue takes the third branch
// ---------------------------------------------------------------------------

#[test]
fn row08_negative_hue_takes_third_branch() {
    let mut rng = Rng::new(0x0808);
    let mut inputs = Vec::new();
    for _ in 0..N {
        inputs.push([
            (-rng.range(f32::from_bits(1), 1e9)).to_bits(),
            rng.range(f32::from_bits(1), 1.0).to_bits(),
            rng.range(0.0, 1.0).to_bits(),
        ]);
    }
    for _ in 0..N {
        inputs.push([
            (-rng.range(1e9, f32::MAX)).to_bits(),
            rng.range(f32::from_bits(1), 1.0).to_bits(),
            rng.range(0.0, 1.0).to_bits(),
        ]);
    }
    assert_same_all("h < 0", inputs);

    // `h < 120 && h < 180` is true for every negative h, so the third branch
    // body runs: dest[0] = m, dest[1] = c + m, dest[2] = x + m. With s = 1 and
    // l = 0.5 we get c = 1, m = 0, so dest[1] must be exactly 1.0 - not `m`.
    let h = harness();
    let src = [(-30.0f32).to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
    let c = call(h.c, src);
    assert_eq!(
        c.rgb[1],
        1.0f32.to_bits(),
        "negative hue must take the third branch, not the final else; got {:#x?}",
        c.rgb
    );
    assert_same("h = -30", src);
}

// ---------------------------------------------------------------------------
// Rows 9 & 10 — exact boundaries and 1-ULP neighbours
// ---------------------------------------------------------------------------

#[test]
fn row09_exact_boundary_hues() {
    let mut rng = Rng::new(0x0909);
    let mut inputs = Vec::new();
    for b in BOUNDARIES {
        for _ in 0..600 {
            inputs.push([
                b.to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    assert_same_all("exact boundary hues", inputs);

    // Sector selection at the boundaries, with s = 1 and l = 0.5 (c = 1, m = 0,
    // x = 0 at every multiple of 60): 120 and 360 must fall into the else.
    let h = harness();
    let expect_else = [(120.0f32, true), (360.0f32, true), (0.0, false), (60.0, false),
                       (180.0, false), (240.0, false), (300.0, false)];
    for (b, is_else) in expect_else {
        let src = [b.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
        let c = call(h.c, src);
        let all_equal = c.rgb[0] == c.rgb[1] && c.rgb[1] == c.rgb[2];
        assert_eq!(
            all_equal, is_else,
            "h = {b}: expected final-else = {is_else}, got rgb = {:#x?}",
            c.rgb
        );
        assert_same("boundary sector selection", src);
    }
}

#[test]
fn row10_one_ulp_around_every_boundary() {
    let mut rng = Rng::new(0x0A0A);
    let mut inputs = Vec::new();
    for b in BOUNDARIES {
        for hv in [
            next_after(b, f32::NEG_INFINITY),
            b,
            next_after(b, f32::INFINITY),
        ] {
            for _ in 0..300 {
                inputs.push([
                    hv.to_bits(),
                    rng.range(f32::from_bits(1), 1.0).to_bits(),
                    rng.range(0.0, 1.0).to_bits(),
                ]);
            }
        }
    }
    assert_same_all("boundary +- 1 ULP", inputs);
}

// ---------------------------------------------------------------------------
// Rows 11-13 — the `s == 0` early-out and its immediate neighbourhood
// ---------------------------------------------------------------------------

fn early_out_row(ctx: &str, s_bits: u32, seed: u64) {
    let h = harness();
    let mut rng = Rng::new(seed);
    let mut inputs = Vec::new();
    for _ in 0..N {
        inputs.push([rng.raw(), s_bits, rng.raw()]);
    }
    for hv in interesting_floats() {
        for lv in interesting_floats() {
            inputs.push([hv, s_bits, lv]);
        }
    }
    assert_same_all(ctx, inputs.clone());

    // The early-out copies `l` verbatim into all three components.
    for src in inputs {
        let c = call(h.c, src);
        assert_eq!(
            c.rgb,
            [src[2], src[2], src[2]],
            "{ctx}: s == 0 must copy l bit-for-bit into r, g and b"
        );
    }
}

#[test]
fn row11_s_is_positive_zero() {
    early_out_row("s = +0.0", 0x0000_0000, 0x0B0B);
}

#[test]
fn row12_s_is_negative_zero() {
    early_out_row("s = -0.0", 0x8000_0000, 0x0C0C);
}

#[test]
fn row13_s_is_subnormal_so_no_early_out() {
    let h = harness();
    let mut rng = Rng::new(0x0D0D);
    let s_values = [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE / 2.0,
        -f32::MIN_POSITIVE / 2.0,
        f32::from_bits(0x007F_FFFF),
    ];
    let mut inputs = Vec::new();
    for s in s_values {
        for (_, lo, hi) in SECTORS {
            for _ in 0..200 {
                inputs.push([
                    rng.range(lo, hi).to_bits(),
                    s.to_bits(),
                    rng.range(f32::from_bits(1), 1.0).to_bits(),
                ]);
            }
        }
    }
    assert_same_all("s subnormal", inputs);

    // A subnormal s is != 0, so the early-out must NOT be taken. For a finite l
    // that is unobservable (`c` stays subnormal, so `l - 0.5c == l` and
    // `c + m == l` after rounding, which makes the arithmetic path produce
    // exactly `(l, l, l)` anyway). `l = +inf` makes the difference visible:
    // c = -inf, m = +inf, so `c + m` is NaN instead of `l`.
    let src = [30.0f32.to_bits(), 1u32, f32::INFINITY.to_bits()];
    let c = call(h.c, src);
    assert_ne!(
        c.rgb,
        [src[2], src[2], src[2]],
        "a subnormal s must not trigger the s == 0 early-out"
    );
    assert!(
        f32::from_bits(c.rgb[0]).is_nan(),
        "s = 1e-45, l = inf must give c + m = -inf + inf = NaN, got {:#x?}",
        c.rgb
    );
    assert_same("s = 1e-45, l = inf", src);
    // ... while `s = +0.0` with the same `l` really does broadcast `l`.
    let early = [30.0f32.to_bits(), 0u32, f32::INFINITY.to_bits()];
    assert_eq!(
        call(h.c, early).rgb,
        [src[2], src[2], src[2]],
        "s = +0.0 must broadcast l even when l = inf"
    );
    assert_same("s = +0.0, l = inf", early);
}

// ---------------------------------------------------------------------------
// Rows 14-19 — the `s` / `l` value classes
// ---------------------------------------------------------------------------

#[test]
fn row14_s_one_and_l_at_the_corners() {
    let mut rng = Rng::new(0x0E0E);
    let mut inputs = Vec::new();
    for l in [0.0f32, -0.0, 0.5, 1.0] {
        for (_, lo, hi) in SECTORS {
            for _ in 0..200 {
                inputs.push([rng.range(lo, hi).to_bits(), 1.0f32.to_bits(), l.to_bits()]);
            }
        }
        for b in BOUNDARIES {
            inputs.push([b.to_bits(), 1.0f32.to_bits(), l.to_bits()]);
        }
    }
    assert_same_all("s = 1, l at corners", inputs);
}

#[test]
fn row15_s_greater_than_one() {
    let mut rng = Rng::new(0x0F0F);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..400 {
            inputs.push([
                rng.range(lo, hi).to_bits(),
                rng.range(1.0, 1e6).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
            inputs.push([
                rng.range(lo, hi).to_bits(),
                rng.range(1e30, f32::MAX).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    assert_same_all("s > 1", inputs);
}

#[test]
fn row16_s_negative() {
    let mut rng = Rng::new(0x1010);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..400 {
            inputs.push([
                rng.range(lo, hi).to_bits(),
                (-rng.range(f32::from_bits(1), 1.0)).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
            inputs.push([
                rng.range(lo, hi).to_bits(),
                (-rng.range(1.0, f32::MAX)).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    assert_same_all("s < 0", inputs);
}

#[test]
fn row17_l_outside_zero_one() {
    let mut rng = Rng::new(0x1111);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..400 {
            let s = rng.range(f32::from_bits(1), 1.0).to_bits();
            inputs.push([rng.range(lo, hi).to_bits(), s, (-rng.range(0.0, 1e6)).to_bits()]);
            inputs.push([rng.range(lo, hi).to_bits(), s, rng.range(1.0, 1e6).to_bits()]);
            inputs.push([
                rng.range(lo, hi).to_bits(),
                s,
                rng.range(1e30, f32::MAX).to_bits(),
            ]);
            inputs.push([
                rng.range(lo, hi).to_bits(),
                s,
                (-rng.range(1e30, f32::MAX)).to_bits(),
            ]);
        }
    }
    assert_same_all("l outside [0,1]", inputs);
}

#[test]
fn row18_l_exactly_one_half() {
    let mut rng = Rng::new(0x1212);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..500 {
            inputs.push([
                rng.range(lo, hi).to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                0.5f32.to_bits(),
            ]);
        }
    }
    for b in BOUNDARIES {
        for _ in 0..100 {
            inputs.push([
                b.to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                0.5f32.to_bits(),
            ]);
        }
    }
    assert_same_all("l = 0.5", inputs);
}

#[test]
fn row19_l_subnormal_and_tiny() {
    let mut rng = Rng::new(0x1313);
    let l_values = [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MIN_POSITIVE / 2.0,
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
    ];
    let mut inputs = Vec::new();
    for l in l_values {
        for (_, lo, hi) in SECTORS {
            for _ in 0..150 {
                inputs.push([
                    rng.range(lo, hi).to_bits(),
                    rng.range(f32::from_bits(1), 1.0).to_bits(),
                    l.to_bits(),
                ]);
            }
        }
    }
    assert_same_all("l subnormal", inputs);
}

// ---------------------------------------------------------------------------
// Rows 20-22 — infinities
// ---------------------------------------------------------------------------

#[test]
fn row20_hue_infinite() {
    let mut rng = Rng::new(0x1414);
    let s_pool = [
        0x0000_0000u32,
        0x8000_0000,
        1.0f32.to_bits(),
        0x7FC0_0000,
        0x7F80_0000,
        0xFF80_0000,
    ];
    let l_pool = [
        0.0f32.to_bits(),
        (-0.0f32).to_bits(),
        0.5f32.to_bits(),
        1.0f32.to_bits(),
        0x7FC0_0000,
        0x7F80_0000,
        0xFF80_0000,
    ];
    let mut inputs = Vec::new();
    for hv in [f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()] {
        for s in s_pool {
            for l in l_pool {
                inputs.push([hv, s, l]);
            }
        }
        for _ in 0..N {
            inputs.push([
                hv,
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    assert_same_all("h = +-inf", inputs);

    // -inf takes the third branch and therefore *uses* x, whose value comes out
    // of `fmodf(-inf, 2.0f)`. That is the one place the libm special case is
    // observable, so nail the exact bit pattern down for both libraries.
    let h = harness();
    let src = [f32::NEG_INFINITY.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
    let c = call(h.c, src);
    assert!(
        f32::from_bits(c.rgb[2]).is_nan(),
        "h = -inf must produce a NaN blue component via fmodf(-inf, 2), got {:#x?}",
        c.rgb
    );
    assert_same("h = -inf, fmodf domain error", src);
}

#[test]
fn row21_saturation_infinite() {
    let mut rng = Rng::new(0x1515);
    let mut inputs = Vec::new();
    for s in [f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()] {
        for l in interesting_floats() {
            for b in BOUNDARIES {
                inputs.push([b.to_bits(), s, l]);
            }
            inputs.push([30.0f32.to_bits(), s, l]);
            inputs.push([200.0f32.to_bits(), s, l]);
            inputs.push([(-5.0f32).to_bits(), s, l]);
        }
        for (_, lo, hi) in SECTORS {
            for _ in 0..300 {
                inputs.push([rng.range(lo, hi).to_bits(), s, rng.range(0.0, 1.0).to_bits()]);
            }
        }
    }
    assert_same_all("s = +-inf", inputs);
}

#[test]
fn row22_lightness_infinite() {
    let mut rng = Rng::new(0x1616);
    let mut inputs = Vec::new();
    for l in [f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()] {
        for s in interesting_floats() {
            for b in BOUNDARIES {
                inputs.push([b.to_bits(), s, l]);
            }
            inputs.push([30.0f32.to_bits(), s, l]);
            inputs.push([150.0f32.to_bits(), s, l]);
            inputs.push([(-150.0f32).to_bits(), s, l]);
        }
        for (_, lo, hi) in SECTORS {
            for _ in 0..300 {
                inputs.push([
                    rng.range(lo, hi).to_bits(),
                    rng.range(f32::from_bits(1), 1.0).to_bits(),
                    l,
                ]);
            }
        }
    }
    assert_same_all("l = +-inf", inputs);
}

// ---------------------------------------------------------------------------
// Rows 23-26 — NaN propagation and operand order
// ---------------------------------------------------------------------------

#[test]
fn row23_hue_nan() {
    let h = harness();
    let mut rng = Rng::new(0x1717);
    let mut inputs = Vec::new();
    for _ in 0..N {
        inputs.push([rng.nan(), rng.raw(), rng.raw()]);
    }
    for _ in 0..N {
        inputs.push([
            rng.nan(),
            rng.range(f32::from_bits(1), 1.0).to_bits(),
            rng.range(0.0, 1.0).to_bits(),
        ]);
    }
    assert_same_all("h = NaN", inputs.clone());

    // Every comparison against a NaN h is unordered, so the final else runs.
    for src in inputs.iter().filter(|s| f32::from_bits(s[1]) != 0.0).take(500) {
        let c = call(h.c, *src);
        assert_eq!(
            c.rgb[0], c.rgb[1],
            "h = NaN must reach the final else (r == g == b == m)"
        );
        assert_eq!(c.rgb[1], c.rgb[2]);
    }
}

#[test]
fn row24_saturation_nan() {
    let h = harness();
    let mut rng = Rng::new(0x1818);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..500 {
            inputs.push([
                rng.range(lo, hi).to_bits(),
                rng.nan(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    for b in BOUNDARIES {
        for _ in 0..100 {
            inputs.push([b.to_bits(), rng.nan(), rng.range(0.0, 1.0).to_bits()]);
        }
    }
    assert_same_all("s = NaN", inputs);

    // `s == 0` is false for NaN, so the early-out is skipped and NaN propagates.
    let src = [30.0f32.to_bits(), 0x7FC0_0000, 0.25f32.to_bits()];
    let c = call(h.c, src);
    assert!(
        f32::from_bits(c.rgb[0]).is_nan(),
        "s = NaN must not take the early-out; got {:#x?}",
        c.rgb
    );
    assert_same("s = NaN", src);
}

#[test]
fn row25_lightness_nan() {
    let mut rng = Rng::new(0x1919);
    let mut inputs = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..500 {
            inputs.push([
                rng.range(lo, hi).to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.nan(),
            ]);
        }
    }
    for b in BOUNDARIES {
        for _ in 0..100 {
            inputs.push([
                b.to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.nan(),
            ]);
        }
    }
    assert_same_all("l = NaN", inputs);
}

#[test]
fn row26_multiple_distinct_nan_payloads() {
    let mut rng = Rng::new(0x1A1A);
    let mut inputs = Vec::new();
    // Two of three NaN, and all three NaN, with distinct payloads so that the
    // surviving NaN identifies which operand each SSE op keeps.
    for _ in 0..N {
        let (a, b, c) = (rng.nan(), rng.nan(), rng.nan());
        inputs.push([a, b, rng.range(0.0, 1.0).to_bits()]);
        inputs.push([a, rng.range(f32::from_bits(1), 1.0).to_bits(), c]);
        inputs.push([rng.range(0.0, 360.0).to_bits(), b, c]);
        inputs.push([a, b, c]);
    }
    // Deterministic, hand-picked payload triples across all sectors.
    let nans = [
        0x7FC0_0000u32, 0xFFC0_0000, 0x7FC0_0001, 0xFFC0_1234, 0x7F80_0001, 0xFF80_0001,
        0x7FBF_FFFF, 0xFFFF_FFFF,
    ];
    for &a in &nans {
        for &b in &nans {
            for hv in [30.0f32, 90.0, 150.0, 200.0, 260.0, 320.0, -30.0, 400.0] {
                inputs.push([hv.to_bits(), a, b]);
                inputs.push([a, b, 0.5f32.to_bits()]);
                inputs.push([a, 1.0f32.to_bits(), b]);
            }
        }
    }
    assert_same_all("multiple NaN payloads", inputs);
}

// ---------------------------------------------------------------------------
// Rows 27-28 — unbiased sweeps
// ---------------------------------------------------------------------------

#[test]
fn row27_fully_random_bit_patterns() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let inputs: Vec<[u32; 3]> = (0..100_000)
        .map(|_| [rng.raw(), rng.raw(), rng.raw()])
        .collect();
    assert_same_all("random raw u32 triples", inputs);

    // Same again, but with log-uniform magnitudes so that small/huge finite
    // values (which a uniform u32 draw makes rare) are well represented.
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let inputs: Vec<[u32; 3]> = (0..50_000)
        .map(|_| {
            [
                rng.log_uniform(true).to_bits(),
                rng.log_uniform(true).to_bits(),
                rng.log_uniform(true).to_bits(),
            ]
        })
        .collect();
    assert_same_all("log-uniform triples", inputs);
}

#[test]
fn row28_cross_product_of_interesting_values() {
    let pool = interesting_floats();
    // The full 3-way cross product would be |pool|^3; sample the h axis over the
    // whole pool and take the (s, l) cross product, which is the pair that the
    // arithmetic actually mixes, then rotate the roles.
    let mut inputs = Vec::new();
    for &s in &pool {
        for &l in &pool {
            inputs.push([30.0f32.to_bits(), s, l]);
            inputs.push([150.0f32.to_bits(), s, l]);
            inputs.push([(-45.0f32).to_bits(), s, l]);
        }
    }
    for &hv in &pool {
        for &s in &pool {
            inputs.push([hv, s, 0.5f32.to_bits()]);
        }
        for &l in &pool {
            inputs.push([hv, 1.0f32.to_bits(), l]);
        }
        for b in BOUNDARIES {
            inputs.push([hv, b.to_bits(), 0.25f32.to_bits()]);
        }
    }
    assert_same_all("interesting-value cross product", inputs);
}

// ---------------------------------------------------------------------------
// Rows 29-32 — pointer relationships and buffer hygiene
// ---------------------------------------------------------------------------

#[test]
fn row29_in_place_conversion() {
    let h = harness();
    let mut rng = Rng::new(0x1D1D);
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..300 {
            cases.push([
                rng.range(lo, hi).to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    for _ in 0..1000 {
        cases.push([rng.raw(), rng.raw(), rng.raw()]);
        cases.push([rng.raw(), 0, rng.raw()]); // s == 0 early-out, in place
        cases.push([rng.raw(), 0x8000_0000, rng.raw()]);
    }

    for src in cases {
        let c = call_overlapping(h.c, src, 0);
        // In place must agree with the disjoint result.
        let disjoint = call(h.c, src);
        assert_eq!(
            [c[4], c[5], c[6]],
            disjoint.rgb,
            "C: in-place result differs from the disjoint result for {src:#x?}"
        );
        for (label, f) in &h.rust {
            let r = call_overlapping(*f, src, 0);
            assert_eq!(
                c, r,
                "{label}: in-place (dest == src) mismatch for src = {src:#x?}"
            );
        }
    }
}

#[test]
fn row30_partially_overlapping_buffers() {
    let h = harness();
    let mut rng = Rng::new(0x1E1E);
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..150 {
            cases.push([
                rng.range(lo, hi).to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    for _ in 0..500 {
        cases.push([rng.raw(), rng.raw(), rng.raw()]);
        cases.push([rng.raw(), 0, rng.raw()]);
    }

    for off in [-3isize, -2, -1, 1, 2, 3] {
        for src in &cases {
            let c = call_overlapping(h.c, *src, off);
            for (label, f) in &h.rust {
                let r = call_overlapping(*f, *src, off);
                assert_eq!(
                    c, r,
                    "{label}: overlapping dest = src{off:+} mismatch for src = {src:#x?}"
                );
            }
        }
    }
}

#[test]
fn row31_misaligned_buffers() {
    let h = harness();
    let mut rng = Rng::new(0x1F1F);
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for (_, lo, hi) in SECTORS {
        for _ in 0..150 {
            cases.push([
                rng.range(lo, hi).to_bits(),
                rng.range(f32::from_bits(1), 1.0).to_bits(),
                rng.range(0.0, 1.0).to_bits(),
            ]);
        }
    }
    for _ in 0..500 {
        cases.push([rng.raw(), rng.raw(), rng.raw()]);
        cases.push([rng.raw(), 0, rng.raw()]);
    }

    for off in [1usize, 2, 3] {
        for src in &cases {
            let c = call_misaligned(h.c, *src, off);
            for (label, f) in &h.rust {
                let r = call_misaligned(*f, *src, off);
                assert_eq!(
                    c, r,
                    "{label}: mis-aligned (+{off} bytes) mismatch for src = {src:#x?}"
                );
            }
        }
    }
}

#[test]
fn row32_no_out_of_bounds_access() {
    // `assert_same` already checks the guard words and the immutability of the
    // source buffer on every single call, so this row re-runs a broad sweep and
    // additionally verifies that a 4th destination slot is never touched.
    let h = harness();
    let mut rng = Rng::new(0x2020);
    for _ in 0..20_000 {
        let src = [rng.raw(), rng.raw(), rng.raw()];
        let c = call(h.c, src);
        assert_eq!(c.dest_guards, [GUARD_LO, GUARD_HI]);
        assert_ne!(
            c.rgb,
            [UNWRITTEN, UNWRITTEN, UNWRITTEN],
            "the C library must always write all three components"
        );
        for (label, f) in &h.rust {
            let r = call(*f, src);
            assert_eq!(
                r.dest_guards,
                [GUARD_LO, GUARD_HI],
                "{label} wrote past dest[0..3] or before dest[0]"
            );
            assert_eq!(r.src_after, c.src_after, "{label} mutated src");
            assert_eq!(r.rgb, c.rgb, "{label} output mismatch for {src:#x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 33-34 — statefulness and a dense deterministic sweep
// ---------------------------------------------------------------------------

#[test]
fn row33_no_hidden_state_or_fpu_mode_leakage() {
    let h = harness();
    let mut rng = Rng::new(0x2121);
    // Interleave C and Rust calls on the same buffers. If either library left
    // MXCSR (rounding mode / FTZ / DAZ) or any global state behind, the results
    // would start to drift after the first few iterations.
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for _ in 0..10_000 {
        cases.push([rng.raw(), rng.raw(), rng.raw()]);
    }
    // Include subnormal-producing inputs, which are the ones that would expose
    // a stray FTZ/DAZ setting.
    for _ in 0..2000 {
        cases.push([
            rng.range(0.0, 360.0).to_bits(),
            f32::from_bits(1).to_bits(),
            f32::from_bits(rng.next_u32() & 0x007F_FFFF).to_bits(),
        ]);
    }

    let mut first_pass = Vec::with_capacity(cases.len());
    for src in &cases {
        let c = call(h.c, *src);
        for (label, f) in &h.rust {
            let r = call(*f, *src);
            assert_eq!(c.rgb, r.rgb, "{label} mismatch for {src:#x?}");
        }
        first_pass.push(c.rgb);
    }
    // Replay in reverse: same inputs must still give the same outputs.
    for (i, src) in cases.iter().enumerate().rev() {
        for (label, f) in &h.rust {
            let r = call(*f, *src);
            assert_eq!(
                first_pass[i], r.rgb,
                "{label}: result changed on replay for {src:#x?} (hidden state?)"
            );
        }
        let c = call(h.c, *src);
        assert_eq!(first_pass[i], c.rgb, "C: result changed on replay");
    }
}

#[test]
fn row34_dense_hue_sweep() {
    let mut inputs = Vec::new();
    // -720 .. 1080 in 0.25 degree steps, for a few (s, l) pairs.
    let sl = [
        (1.0f32, 0.5f32),
        (0.5, 0.25),
        (0.25, 0.75),
        (1.0, 0.0),
        (1.0, 1.0),
        (f32::from_bits(1), 0.5),
    ];
    let mut i = -2880i32;
    while i <= 4320 {
        let hv = i as f32 * 0.25;
        for (s, l) in sl {
            inputs.push([hv.to_bits(), s.to_bits(), l.to_bits()]);
        }
        i += 1;
    }
    assert_same_all("dense hue sweep", inputs);

    // Also sweep every ULP in a tight window around each boundary.
    let mut inputs = Vec::new();
    for b in BOUNDARIES {
        let mut v = b;
        for _ in 0..64 {
            v = next_after(v, f32::NEG_INFINITY);
        }
        for _ in 0..128 {
            inputs.push([v.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()]);
            inputs.push([v.to_bits(), 0.3f32.to_bits(), 0.7f32.to_bits()]);
            v = next_after(v, f32::INFINITY);
        }
    }
    assert_same_all("per-ULP boundary sweep", inputs);
}
