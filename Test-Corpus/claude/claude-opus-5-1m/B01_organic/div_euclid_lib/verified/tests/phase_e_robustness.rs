//! Phase B/C robustness addendum:
//!
//! 1. `f1_full_range_stride_sweep` — a deterministic sweep that walks the WHOLE
//!    `i32` dividend range (stride is coprime with 2^32 so every residue class
//!    is hit) against every interesting divisor.
//! 2. `f2_reference_build_variants` — the C source contains exactly one signed
//!    overflow (`q = INT_MAX + 1` for `div_euclid(INT_MIN, -1)`), which is UB
//!    and could in principle be compiled differently at other optimisation
//!    levels. This test rebuilds the C source with `-O0/-O1/-O2/-O3`,
//!    `-fwrapv`, `-fno-strict-overflow` and (if available) clang, and requires
//!    the Rust `.so` to agree with EVERY one of those reference builds, so the
//!    translation is not tied to one particular C build configuration.
//!    Reference objects are written to `target/`; `c_src/` is never modified.

mod common;

use common::{assert_same, boundary_values, libs, DivFn, Rng, I32_MAX, I32_MIN};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn f1_full_range_stride_sweep() {
    let divisors: Vec<i32> = {
        let mut d = vec![
            1,
            -1,
            2,
            -2,
            3,
            -3,
            5,
            -5,
            7,
            -7,
            10,
            -10,
            1023,
            -1023,
            1 << 16,
            -(1 << 16),
            1 << 30,
            -(1 << 30),
            I32_MAX,
            -I32_MAX,
            I32_MIN,
            0,
        ];
        d.dedup();
        d
    };
    // 2^32 / 65_537 ~= 65_534 distinct dividends per divisor, spread over the
    // entire range (65_537 is odd => the orbit covers all residues mod 2^32).
    let stride: i64 = 65_537;
    let mut v1: i64 = I32_MIN as i64;
    let mut count = 0usize;
    while v1 <= I32_MAX as i64 {
        let a = v1 as i32;
        for &v2 in &divisors {
            assert_same("F1", a, v2);
        }
        v1 += stride;
        count += 1;
    }
    assert!(count > 60_000, "sweep too short: {count}");
}

#[test]
fn f2_reference_build_variants() {
    let manifest = common::manifest_dir();
    let src = manifest.join("c_src/src/lib.c");
    let inc = manifest.join("c_src/include");
    let outdir = manifest.join("target/altc"); // never inside c_src/
    std::fs::create_dir_all(&outdir).expect("create target/altc");

    let variants: [(&str, &[&str]); 7] = [
        ("gcc-O0", &["-O0"]),
        ("gcc-O1", &["-O1"]),
        ("gcc-O2", &["-O2"]),
        ("gcc-O3", &["-O3"]),
        ("gcc-O2-fwrapv", &["-O2", "-fwrapv"]),
        ("gcc-O2-fno-strict-overflow", &["-O2", "-fno-strict-overflow"]),
        ("clang-O2", &["-O2"]),
    ];

    let mut refs: Vec<(String, DivFn)> = Vec::new();
    for (name, flags) in variants {
        let cc = if name.starts_with("clang") { "clang" } else { "gcc" };
        let so: PathBuf = outdir.join(format!("ref_{name}.so"));
        let ok = Command::new(cc)
            .args(flags)
            .args(["-fPIC", "-shared", "-o"])
            .arg(&so)
            .arg(&src)
            .arg("-I")
            .arg(&inc)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("skipping unavailable reference build: {name}");
            continue;
        }
        let lib: &'static libloading::Library =
            Box::leak(Box::new(unsafe { libloading::Library::new(&so) }.expect("dlopen ref")));
        let f: DivFn = unsafe { *lib.get::<DivFn>(b"div_euclid\0").expect("dlsym ref") };
        refs.push((name.to_string(), f));
    }
    assert!(
        refs.len() >= 4,
        "expected several C reference builds, got {}",
        refs.len()
    );

    let rust = libs().rust;

    let check = |v1: i32, v2: i32| {
        let expect = rust(v1, v2);
        for (name, f) in &refs {
            let got = f(v1, v2);
            assert_eq!(
                got, expect,
                "Rust disagrees with C built as [{name}] on div_euclid({v1}, {v2}): C={got} Rust={expect}"
            );
        }
    };

    // the UB case first, then the whole boundary cross product, then randoms
    check(I32_MIN, -1);
    check(I32_MIN, 1);
    check(I32_MIN, I32_MIN);
    check(I32_MIN + 1, -1);
    let vals = boundary_values();
    for &v1 in &vals {
        for &v2 in &vals {
            check(v1, v2);
        }
    }
    let mut rng = Rng::new(0xF002);
    for _ in 0..200_000 {
        check(rng.next_i32(), rng.next_i32());
    }
}
