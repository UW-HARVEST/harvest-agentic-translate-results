//! Phase D — symbol parity between the C `.so` and the Rust `.so`, plus a heavy
//! (ignored-by-default) fuzz run for extra confidence.

mod common;

use common::{Libs, Rng, SEED};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn dynsyms(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn first_so(dir: &Path) -> PathBuf {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    assert!(!v.is_empty(), "no .so in {}", dir.display());
    v.remove(0)
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name.
#[test]
fn symbol_parity_is_exact() {
    let c = match std::env::var("HSL_C_SO") {
        Ok(p) => PathBuf::from(p),
        Err(_) => first_so(&repo_root().join("c_src/build")),
    };
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let r = match std::env::var("HSL_RUST_SO") {
        Ok(p) => PathBuf::from(p),
        Err(_) => profile_dir.join("libhsl_to_rgb_lib.so"),
    };
    assert!(r.is_file(), "build the cdylib first: {}", r.display());

    let cs = dynsyms(&c);
    let rs = dynsyms(&r);
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {cs:?}\nRust ({}) = {rs:?}",
        c.display(),
        r.display()
    );
    assert!(
        cs.contains(&"hsl_to_rgb".to_string()),
        "sanity: the C .so must export hsl_to_rgb, got {cs:?}"
    );
    eprintln!("C exports {} symbol(s); 0 missing from Rust", cs.len());
}

/// The loaded Rust symbol must be a genuinely different code address from the C
/// one — i.e. the test really did go through two distinct shared objects.
#[test]
fn both_libraries_are_distinct_objects() {
    let libs = Libs::load();
    assert_ne!(libs.c as usize, libs.rust as usize);
    // And they agree on a trivial input, proving both are live.
    libs.assert_same([120.0, 0.0, 0.375], "phase_d smoke");
}

/// Heavy fuzz — 5,000,000 fully random bit patterns. Not run by default because
/// of its runtime; enable with `cargo test --release -- --ignored`.
#[test]
#[ignore = "long-running; run explicitly with --ignored"]
fn heavy_bitpattern_fuzz() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xD0D0);
    for i in 0..5_000_000u64 {
        let src = [r.any_f32(), r.any_f32(), r.any_f32()];
        libs.assert_same(src, &format!("heavy i={i}"));
    }
}

/// Heavy NaN-only fuzz — 5,000,000 NaN triples, the case where operand order
/// (and hence the surviving payload) matters.
#[test]
#[ignore = "long-running; run explicitly with --ignored"]
fn heavy_nan_fuzz() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0x00A0_0BAD);
    for i in 0..5_000_000u64 {
        let src = [r.any_nan(), r.any_nan(), r.any_nan()];
        libs.assert_same(src, &format!("heavy nan i={i}"));
    }
}
