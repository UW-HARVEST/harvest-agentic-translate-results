//! Meta-tests about the harness itself and about export parity.
//!
//! Two things are guarded here:
//!   1. The harness really loads two distinct shared objects and really would
//!      report a difference (otherwise every other test could pass vacuously).
//!   2. Every symbol the C `.so` exports is exported by the Rust `.so` under the
//!      exact same name.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Harness self-check
// ---------------------------------------------------------------------------

/// The two libraries must be separate objects: identical symbol addresses would
/// mean we are comparing one implementation with itself.
#[test]
fn harness_loads_two_distinct_libraries() {
    let l = libs();
    for name in ["capsule", "c2GJK", "c2Dot", "c2Collided"] {
        let (c, r) = l.pair::<unsafe extern "C" fn()>(name);
        let ca = *c as usize;
        let ra = *r as usize;
        assert_ne!(ca, ra, "`{name}` resolved to the same address in both libraries");
    }
}

/// Prove the comparison helpers are not vacuous: feeding two genuinely
/// different C functions through them must trip the assertion.
#[test]
fn harness_detects_a_real_mismatch() {
    let l = libs();
    // c2Skew and c2CCW90 are negatives of each other.
    let (skew, _) = l.pair::<unsafe extern "C" fn(c2v) -> c2v>("c2Skew");
    let (ccw, _) = l.pair::<unsafe extern "C" fn(c2v) -> c2v>("c2CCW90");
    let a = c2v { x: 3.0, y: 7.0 };
    let (x, y) = unsafe { (skew(a), ccw(a)) };
    assert_ne!(raw(&x), raw(&y), "sanity inputs were not actually different");

    let caught = std::panic::catch_unwind(|| {
        assert_bytes_eq(&x, &y, "intentional mismatch");
    });
    assert!(caught.is_err(), "assert_bytes_eq failed to flag a real difference");

    let caught = std::panic::catch_unwind(|| {
        assert_f32_eq(1.0, 1.0000001, "intentional mismatch");
    });
    assert!(caught.is_err(), "assert_f32_eq failed to flag a real difference");

    // -0.0 vs 0.0 and NaN vs NaN are handled bitwise, not by `==`.
    assert!(!f32_bits_eq(0.0, -0.0), "0.0 and -0.0 must not compare equal");
    assert!(f32_bits_eq(f32::NAN, f32::NAN), "identical NaN bits must compare equal");
}

// ---------------------------------------------------------------------------
// Export parity (`nm -D`)
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn so_in(dir: &Path, want_capsule_lib: bool) -> PathBuf {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n.ends_with(".so") && (n.contains("capsule_lib") == want_capsule_lib)
        })
        .collect();
    hits.sort();
    hits.pop().unwrap_or_else(|| panic!("no matching .so in {}", dir.display()))
}

/// Dynamic symbols defined by a shared object, excluding the toolchain's own
/// bookkeeping entries (`_init`, `_fini`, `__*`, ...).
fn exported_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm -D {}`: {e}", so.display()));
    assert!(out.status.success(), "`nm -D {}` failed", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .filter(|s| !s.starts_with('_'))
        .map(|s| s.to_string())
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_so = so_in(&repo_root().join("c_src/build"), false);

    // Use the same Rust object the rest of the suite loads.
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let profile = deps.parent().unwrap();
    let rust_so = [
        std::env::var("CAPSULE_RUST_SO").map(PathBuf::from).unwrap_or_else(|_| profile.join("libcapsule_lib.so")),
        deps.join("libcapsule_lib.so"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ffi-so/debug/libcapsule_lib.so"),
    ]
    .into_iter()
    .find(|p| p.exists())
    .unwrap_or_else(|| {
        // Force the harness to build it, then look again.
        let _ = libs();
        so_in(&Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ffi-so/debug"), true)
    });

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(!c_syms.is_empty(), "no symbols found in {}", c_so.display());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {missing:?}",
        rust_so.display(),
        missing.len(),
        c_so.display(),
    );

    // Every C symbol must also be resolvable via dlsym in both objects.
    let l = libs();
    for s in &c_syms {
        let _ = l.pair::<unsafe extern "C" fn()>(s);
    }

    eprintln!("export parity OK: {} symbols", c_syms.len());
}

// ---------------------------------------------------------------------------
// Coverage probe for the one indeterminate path in the C source
// ---------------------------------------------------------------------------

/// `c2GJK`'s loop appends a simplex vertex without setting `u`; if the loop ever
/// exits by exhausting `iter < 20`, C would read that indeterminate `u` in
/// `c2Witness`. This probe records the highest iteration count reached so the
/// suite documents whether that path is actually reachable.
#[test]
fn c2GJK_never_exhausts_its_iteration_budget() {
    type GjkFn = unsafe extern "C" fn(
        *const std::ffi::c_void,
        i32,
        *const c2x,
        *const std::ffi::c_void,
        i32,
        *const c2x,
        *mut c2v,
        *mut c2v,
        i32,
        *mut i32,
        *mut c2GJKCache,
    ) -> f32;

    let l = libs();
    let (c, _r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0xC0FFEE);
    let mut max_iter = 0i32;

    for _ in 0..scale(40000) {
        let circle = rng.circle();
        let aabb = rng.aabb();
        let cap = rng.capsule();
        let pick = |k: u32| -> (*const std::ffi::c_void, i32) {
            match k {
                0 => (&circle as *const _ as *const _, C2_TYPE_CIRCLE),
                1 => (&aabb as *const _ as *const _, C2_TYPE_AABB),
                _ => (&cap as *const _ as *const _, C2_TYPE_CAPSULE),
            }
        };
        let (pa, ta) = pick(rng.below(3));
        let (pb, tb) = pick(rng.below(3));
        let ax = rng.xform();
        let bx = rng.xform();
        let mut it = -1i32;
        unsafe {
            c(
                pa,
                ta,
                &ax,
                pb,
                tb,
                &bx,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                rng.below(2) as i32,
                &mut it,
                std::ptr::null_mut(),
            );
        }
        max_iter = max_iter.max(it);
    }

    eprintln!("max c2GJK iterations observed: {max_iter}");
    assert!(
        max_iter < 20,
        "c2GJK reached the iteration cap ({max_iter}); the C code then reads an \
         indeterminate `u`, so such inputs cannot be compared deterministically"
    );
}
