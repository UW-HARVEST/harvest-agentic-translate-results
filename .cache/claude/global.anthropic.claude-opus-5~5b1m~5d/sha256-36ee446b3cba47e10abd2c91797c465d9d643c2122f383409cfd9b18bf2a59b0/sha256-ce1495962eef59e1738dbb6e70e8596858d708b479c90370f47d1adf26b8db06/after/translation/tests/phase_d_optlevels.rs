//! Phase D extra - robustness of the ground truth across C optimization levels.
//!
//! The C relies on signed-overflow and negative-right-shift behaviour that is
//! UB / implementation-defined, so an optimizing compiler could in principle
//! produce different results than the default (`-O0`) CMake build. This test
//! builds the *unmodified* `c_src/src/lib.c` at several optimization levels into
//! a temp directory (nothing in `c_src/` is touched) and diff-checks the Rust
//! `.so` against every one of them. If they all agree, the Rust matches the C
//! ground truth robustly rather than only matching one compiler configuration.
//!
//! Skipped gracefully if no C compiler is available.

mod common;

use common::*;

use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

use libloading::{Library, Symbol};

const OPT_FLAGS: [&str; 6] = ["-O0", "-O1", "-O2", "-O3", "-Os", "-Ofast"];

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("encode_quant_optlevels")
}

fn build_variant(cc: &str, flags: &[&str], tag: &str) -> Option<PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let src = root.join("c_src/src/lib.c");
    let inc = root.join("c_src/include");
    let dir = tmp_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let out = dir.join(format!("libcvar_{tag}.so"));

    let status = Command::new(cc)
        .arg("-shared")
        .arg("-fPIC")
        .args(flags)
        .arg("-I")
        .arg(&inc)
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status()
        .ok()?;
    if status.success() && out.is_file() {
        Some(out)
    } else {
        None
    }
}

fn have_cc(cc: &str) -> bool {
    Command::new(cc)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn rust_matches_c_across_optimization_levels() {
    let compilers: Vec<&str> = ["cc", "gcc", "clang"]
        .into_iter()
        .filter(|c| have_cc(c))
        .collect();
    if compilers.is_empty() {
        eprintln!("SKIP: no C compiler found");
        return;
    }

    let mut variants: Vec<(String, PathBuf)> = Vec::new();
    for cc in &compilers {
        for flags in OPT_FLAGS {
            let tag = format!("{cc}{}", flags.replace('-', "_"));
            if let Some(p) = build_variant(cc, &[flags], &tag) {
                variants.push((format!("{cc} {flags}"), p));
            }
        }
        // Also the explicitly-wrapping and strict-overflow variants, which is
        // where a UB-exploiting compiler would diverge if it were going to.
        for extra in [
            vec!["-O2", "-fwrapv"],
            vec!["-O2", "-fno-strict-overflow"],
            vec!["-O3", "-fstrict-overflow"],
            vec!["-O2", "-march=native"],
        ] {
            let tag = format!("{cc}{}", extra.join("").replace(['-', '='], "_"));
            if let Some(p) = build_variant(cc, &extra, &tag) {
                variants.push((format!("{cc} {}", extra.join(" ")), p));
            }
        }
    }

    assert!(
        !variants.is_empty(),
        "no C variant could be built even though a compiler exists"
    );
    eprintln!("built {} C variants", variants.len());

    // Load each variant and compare against the Rust .so.
    let mut rng = Rng::for_row("optlevels");
    let cases: Vec<Args> = {
        let mut v = Vec::new();
        // The full extremes cross-product (7^6) is too much per variant here;
        // use a broad randomized set across all axes plus all extreme tuples of
        // the three most overflow-sensitive parameters.
        for _ in 0..200_000 {
            let l = L_CLASSES[(rng.next_u64() % 13) as usize];
            let u = U_CLASSES[(rng.next_u64() % 12) as usize];
            let vc = V_CLASSES[(rng.next_u64() % 9) as usize];
            v.push(gen_args(l, u, vc, &mut rng));
        }
        for uni in EXTREMES {
            for step in EXTREMES {
                for pred in EXTREMES {
                    for lsbit in [0i32, 1, 2, 4, 8, -1, -4, i32::MIN, i32::MAX] {
                        v.push(Args::new(uni, step, pred, i32::MAX, i32::MIN, lsbit));
                        v.push(Args::new(uni, step, pred, i32::MIN, i32::MAX, lsbit));
                    }
                }
            }
        }
        for uni in -32..=32i32 {
            for lsbit in -8..=8i32 {
                for step in [0i32, 1, 8, -8, 255, i32::MAX, i32::MIN] {
                    v.push(Args::new(uni, step, 1000, -1000, 7, lsbit));
                }
            }
        }
        v
    };
    eprintln!("{} cases per variant", cases.len());

    for (name, path) in &variants {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let f: EncodeQuantFn = unsafe {
            let s: Symbol<EncodeQuantFn> = lib
                .get(b"encode_quant\0")
                .unwrap_or_else(|e| panic!("{name}: no encode_quant: {e}"));
            *s
        };

        let mut diverged = 0usize;
        let mut first: Option<(Args, c_int, c_int)> = None;
        for &a in &cases {
            let cv = unsafe { f(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
            let rv = call_rust(a);
            if cv != rv {
                diverged += 1;
                if first.is_none() {
                    first = Some((a, cv, rv));
                }
            }
        }
        assert_eq!(
            diverged,
            0,
            "Rust diverges from C built with `{name}` on {diverged} case(s); first: {:?}",
            first
        );
        eprintln!("  OK  {name}");
    }
}
