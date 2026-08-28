//! Symbol-table parity: every symbol the C `.so` exports must also be exported
//! by the Rust `.so` under exactly the same name, and must be resolvable
//! through `dlsym`.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every function with external linkage in `c_src/src/lib.c`.  The two
/// `static inline` helpers are deliberately absent.
const EXPECTED: &[&str] = &[
    "c2AABBtoAABB",
    "c2AABBtoPoint",
    "c2Absv",
    "c2Add",
    "c2CCW90",
    "c2CastRay",
    "c2CircleToPoint",
    "c2Div",
    "c2Dot",
    "c2Len",
    "c2Maxv",
    "c2Minv",
    "c2MulmvT",
    "c2Mulvs",
    "c2Norm",
    "c2RaytoAABB",
    "c2RaytoCapsule",
    "c2RaytoCircle",
    "c2Skew",
    "c2Sub",
    "c2V",
    "gen_ray",
];

fn dynamic_text_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("running `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            (kind == "T").then(|| name.to_string())
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn c_so() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build; build the C library first")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.pop().expect("no .so in c_src/build")
}

fn rust_so() -> PathBuf {
    // Reuse the harness's resolution (which builds the cdylib on demand) by
    // loading the libraries first, then locating the artifact.
    let _ = libs();
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let direct = profile_dir.join("libgen_ray_lib.so");
    if direct.exists() {
        return direct;
    }
    let release = profile_dir
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/difftest")
        .join(if release { "release" } else { "debug" })
        .join("libgen_ray_lib.so")
}

#[test]
fn c_so_exports_exactly_the_expected_set() {
    let got = dynamic_text_symbols(&c_so());
    let mut want: Vec<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "the C .so symbol table changed; update EXPECTED and the Rust exports"
    );
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dynamic_text_symbols(&c_so());
    let rs = dynamic_text_symbols(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}

/// The Rust `.so` must not export extra public functions beyond the C set;
/// Rust's own runtime symbols (`rust_eh_personality` and friends) are ignored.
#[test]
fn rust_so_exports_no_unexpected_c_style_symbols() {
    let c = dynamic_text_symbols(&c_so());
    let rs = dynamic_text_symbols(&rust_so());
    let extra: Vec<&String> = rs
        .iter()
        .filter(|s| !c.contains(s))
        .filter(|s| !s.starts_with("_") && !s.starts_with("rust_"))
        .collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

/// `nm` only proves the names are in the table; this confirms each one is
/// actually resolvable via `dlsym` in both libraries.
#[test]
fn every_symbol_resolves_in_both_libraries() {
    let l = libs();
    for name in EXPECTED {
        let mut key = name.as_bytes().to_vec();
        key.push(0);
        unsafe {
            l.c.get::<*const ()>(&key)
                .unwrap_or_else(|e| panic!("dlsym {name} in the C .so: {e}"));
            l.rs.get::<*const ()>(&key)
                .unwrap_or_else(|e| panic!("dlsym {name} in the Rust .so: {e}"));
        }
    }
}
