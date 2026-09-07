//! Phase D — symbol parity gate and a large randomized sweep.
//!
//! The symbol test shells out to `nm -D` on both `.so` files and asserts the
//! defined-symbol sets are equal, so a regression in the export surface fails
//! the test suite rather than a hand-run command.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm -D --defined-only");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Linker-synthesised section markers are not translation-unit
            // symbols and exist in every ELF shared object.
            const SECTION_MARKERS: &[&str] =
                &["_init", "_fini", "__bss_start", "_edata", "_end", "__TMC_END__"];
            if SECTION_MARKERS.contains(&name) {
                return None;
            }
            Some(format!("{kind} {name}"))
        })
        .collect()
}

fn nm_undefined_non_libc(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(path)
        .output()
        .expect("run nm -D -u");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    // Everything the Rust runtime shim imports comes from libc / libgcc_s and
    // is versioned (`sym@GLIBC_x.y`, `sym@GCC_x.y`) or is a weak toolchain hook.
    const WEAK_HOOKS: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let name = l.split_whitespace().last()?;
            if name.contains('@') || WEAK_HOOKS.contains(&name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// SYMBOLS.md gate: every symbol the C `.so` defines, the Rust `.so` must define
/// under the exact same name, and the Rust `.so` must not define extras.
#[test]
fn symbol_sets_are_identical() {
    let im = impls();
    let c = nm_defined(&im.c_path);
    let r = nm_defined(&im.rust_path);

    assert!(
        c.contains("T to_barycentric"),
        "sanity: C .so should export to_barycentric, got {c:?}"
    );

    let missing_in_rust: Vec<_> = c.difference(&r).cloned().collect();
    let extra_in_rust: Vec<_> = r.difference(&c).cloned().collect();

    assert!(
        missing_in_rust.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing_in_rust:?}\n\
         C:    {c:?}\nRust: {r:?}",
        missing_in_rust.len()
    );
    assert!(
        extra_in_rust.is_empty(),
        "Rust .so exports {} symbol(s) the C .so does not: {extra_in_rust:?}",
        extra_in_rust.len()
    );
    assert_eq!(c, r, "symbol sets must be identical");
}

/// No untranslated library symbol may be left dangling in the Rust `.so`.
#[test]
fn no_undefined_non_libc_symbols() {
    let im = impls();
    let leftovers = nm_undefined_non_libc(&im.rust_path);
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined non-libc symbols (untranslated code?): {leftovers:?}"
    );
}

/// The private helper the arithmetic core lives in must stay hidden, exactly as
/// the C's `static` helpers are, so that the export surface matches.
#[test]
fn asm_core_is_not_exported() {
    let im = impls();
    let r = nm_defined(&im.rust_path);
    assert!(
        !r.iter().any(|s| s.contains("to_barycentric_core")),
        "to_barycentric_core leaked into the dynamic symbol table: {r:?}"
    );
}

/// A large uniform-random-bits sweep over the whole 256-bit input space. This is
/// the catch-all that backstops the structured rows: every float class,
/// including NaN payload collisions, appears here.
#[test]
fn heavy_uniform_fuzz() {
    let n: usize = std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    let mut rng = Rng::new(SEED ^ 0xF0F0_F0F0_F0F0_F0F0);
    let mut failures = 0usize;
    let mut reports = Vec::new();
    for _ in 0..n {
        let vs = [
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
        ];
        if let Err(e) = diff("heavy-uniform", vs[0], vs[1], vs[2], vs[3]) {
            failures += 1;
            if reports.len() < 5 {
                reports.push(e);
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures}/{n} uniform-random cases diverged\n{}",
        reports.join("\n---\n")
    );
}

/// A large sweep biased toward the values that actually exercise the special
/// cases: each component is drawn either from `SPECIALS` or from a random
/// exponent/mantissa, so degenerate triangles, signed zeros, subnormals and NaN
/// collisions all occur with high probability.
#[test]
fn heavy_biased_fuzz() {
    let n: usize = std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    let mut rng = Rng::new(SEED ^ 0x0A0A_0A0A_0A0A_0A0A);
    let mut failures = 0usize;
    let mut reports = Vec::new();
    for _ in 0..n {
        let pick = |r: &mut Rng| {
            if r.below(2) == 0 {
                SPECIALS[r.below(SPECIALS.len() as u32) as usize]
            } else {
                r.any_bits()
            }
        };
        let mut vs = [
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
        ];
        // With some probability, make the triangle degenerate.
        match rng.below(6) {
            0 => vs[1] = vs[0],
            1 => vs[2] = vs[0],
            2 => vs[2] = vs[1],
            3 => {
                vs[1] = vs[0];
                vs[2] = vs[0];
            }
            4 => vs[3] = vs[0],
            _ => {}
        }
        if let Err(e) = diff("heavy-biased", vs[0], vs[1], vs[2], vs[3]) {
            failures += 1;
            if reports.len() < 5 {
                reports.push(e);
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures}/{n} biased-random cases diverged\n{}",
        reports.join("\n---\n")
    );
}

/// Exhaustive over a dense structured grid: all four vectors drawn from a
/// 12-value pool in every combination would be 12^8, so instead sweep every
/// pair of slots exhaustively over the pool while the rest cycles.
#[test]
fn structured_grid_sweep() {
    const POOL: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        f32::MIN_POSITIVE,
        SUBNORMAL_MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        QNAN,
        SNAN,
    ];
    let mut rng = Rng::for_row("grid");
    let mut failures = 0usize;
    let mut reports = Vec::new();
    let mut total = 0usize;
    for s1 in 0..8usize {
        for s2 in 0..8usize {
            for &a in POOL {
                for &b in POOL {
                    let mut vs = [
                        rng.vec_with(|r| r.scaled(4.0)),
                        rng.vec_with(|r| r.scaled(4.0)),
                        rng.vec_with(|r| r.scaled(4.0)),
                        rng.vec_with(|r| r.scaled(4.0)),
                    ];
                    set_slot(&mut vs, s1, a);
                    set_slot(&mut vs, s2, b);
                    total += 1;
                    if let Err(e) = diff("grid", vs[0], vs[1], vs[2], vs[3]) {
                        failures += 1;
                        if reports.len() < 5 {
                            reports.push(e);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures}/{total} structured-grid cases diverged\n{}",
        reports.join("\n---\n")
    );
}
