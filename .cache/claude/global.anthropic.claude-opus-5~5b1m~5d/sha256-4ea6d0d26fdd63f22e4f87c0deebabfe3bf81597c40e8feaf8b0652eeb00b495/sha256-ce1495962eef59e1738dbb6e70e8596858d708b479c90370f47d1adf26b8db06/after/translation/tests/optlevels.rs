//! Robustness check: is the Rust translation faithful to the C *independently of
//! how the C was compiled*?
//!
//! The authoritative ground truth is the `.so` that `c_src/CMakeLists.txt`
//! produces — no `CMAKE_BUILD_TYPE` is set, so that is an unoptimised build. But
//! a translation that only matched `-O0` would be fragile: it could be matching
//! an artifact of one particular instruction selection rather than the language
//! semantics. So this test compiles `c_src/src/lib.c` at `-O0`, `-O1`, `-O2`,
//! `-O3` and `-Os` (leaving `c_src/` itself untouched — the objects go into
//! `target/`) and compares all of them.
//!
//! **Result of running it:** the Rust matches the CMake-built C on every input,
//! but the C's *own* optimisation levels do not all agree with each other. Every
//! such disagreement is confined to the **sign and payload bits of a NaN** — a
//! value IEEE-754 leaves unspecified and which gcc's instruction selection
//! therefore changes freely (at `-O0` `fabsf` is a real `andps 0x7fffffff` that
//! clears the sign; at `-O1`+ gcc folds it away and the original NaN sign
//! survives). So this test asserts the two things that are actually true and
//! meaningful:
//!
//!   1. the Rust reproduces the CMake-built C **bit for bit, always** (that build
//!      is the ground truth named by the task), and
//!   2. wherever the C's behaviour is *optimisation-independent*, the Rust agrees
//!      with it too — i.e. every remaining disagreement is NaN-vs-NaN, never a
//!      difference in a numeric value, a zero sign, or which branch was taken.
//!
//! `-ffast-math` is deliberately NOT tested: it licenses the compiler to break
//! IEEE-754 semantics, so it would be a different program.

mod common;

use common::*;

use std::path::PathBuf;
use std::process::Command;

const LEVELS: &[&str] = &["-O0", "-O1", "-O2", "-O3", "-Os"];

fn build_variants() -> Vec<Lib> {
    let out_dir = manifest_dir().join("target").join("c-optlevels");
    std::fs::create_dir_all(&out_dir).expect("create target/c-optlevels");
    let src = workspace_root().join("c_src").join("src").join("lib.c");
    let inc = workspace_root().join("c_src").join("include");

    let mut libs = Vec::new();
    for level in LEVELS {
        let so: PathBuf = out_dir.join(format!("libc{}.so", level.replace('-', "_")));
        let status = Command::new("cc")
            .arg(level)
            .args(["-fPIC", "-shared", "-o"])
            .arg(&so)
            .arg(&src)
            .arg("-I")
            .arg(&inc)
            .arg("-lm")
            .status()
            .expect("run cc");
        assert!(status.success(), "cc {level} failed");
        libs.push(Lib::open_public(&format!("c{level}"), &so));
    }
    libs
}

/// Compare every optimisation level of the C against the CMake-built C `.so` and
/// against every Rust variant.
#[test]
fn all_c_optimisation_levels_agree_with_the_rust() {
    let variants = build_variants();
    let reference = c_lib();

    let pool = specials_and_nans();
    let mut cases: Vec<(f32, f32, f32)> = Vec::new();

    // Deterministic randomized cases across all the axes of CONFIGS.md.
    let mut rng = Rng::new(0x0FEE);
    for _ in 0..60_000 {
        let h = match rng.below(4) {
            0 => random_hue_any_sector(&mut rng),
            1 => rng.bits_f32(),
            2 => rng.log_uniform(),
            _ => rng.pick(&pool),
        };
        let s = if rng.below(3) == 0 { rng.pick(&pool) } else { rng.bits_f32() };
        let l = if rng.below(3) == 0 { rng.pick(&pool) } else { rng.bits_f32() };
        cases.push((h, s, l));
    }
    // Plus the whole specials^3 cross-product and every exact sector boundary.
    for &h in &pool {
        for &s in &pool {
            for &l in &pool {
                cases.push((h, s, l));
            }
        }
    }
    for &h in HUE_BOUNDARIES {
        for &s in &pool {
            for &l in &pool {
                cases.push((h, s, l));
            }
        }
    }
    for h in hue_boundary_neighbours() {
        for &s in &pool {
            for &l in &pool {
                cases.push((h, s, l));
            }
        }
    }

    // Failures that must never happen.
    let mut hard: Vec<String> = Vec::new();
    // Disagreements between the C's own optimisation levels, which are allowed
    // only when both sides are NaN (unspecified bits).
    let mut nan_only = 0usize;

    let fmt = |h: f32, s: f32, l: f32, got: [u32; 3], want: [u32; 3], who: &str| {
        format!(
            "{who}: h={:#010x} s={:#010x} l={:#010x} -> {:#010x},{:#010x},{:#010x} (cmake C: {:#010x},{:#010x},{:#010x})",
            h.to_bits(), s.to_bits(), l.to_bits(),
            got[0], got[1], got[2], want[0], want[1], want[2],
        )
    };

    for &(h, s, l) in &cases {
        let want = run(reference, h, s, l);

        // (1) The Rust must match the ground-truth build exactly.
        for r in rust_libs() {
            let got = run(r, h, s, l);
            if got != want && hard.len() < 20 {
                hard.push(fmt(h, s, l, got, want, &r.name));
            }
        }

        // (2) A C optimisation level may differ from -O0 only in NaN bits.
        for v in &variants {
            let got = run(v, h, s, l);
            if got == want {
                continue;
            }
            let all_differences_are_nan_vs_nan = got.iter().zip(want.iter()).all(|(&g, &w)| {
                g == w || (f32::from_bits(g).is_nan() && f32::from_bits(w).is_nan())
            });
            if all_differences_are_nan_vs_nan {
                nan_only += 1;
            } else if hard.len() < 20 {
                hard.push(fmt(h, s, l, got, want, &v.name));
            }
        }
    }

    assert!(
        hard.is_empty(),
        "{} case(s) with a NON-NaN disagreement (a real semantic difference):\n{}",
        hard.len(),
        hard.join("\n")
    );
    eprintln!(
        "{} inputs x ({} C optimisation levels + {} Rust builds):\n  \
         Rust == CMake-built C on every input;\n  \
         {} case(s) where an optimised C build chose different NaN sign/payload bits \
         (IEEE-754 leaves those unspecified) - no numeric, zero-sign or branch difference anywhere.",
        cases.len(),
        variants.len(),
        rust_libs().len(),
        nan_only,
    );
}
