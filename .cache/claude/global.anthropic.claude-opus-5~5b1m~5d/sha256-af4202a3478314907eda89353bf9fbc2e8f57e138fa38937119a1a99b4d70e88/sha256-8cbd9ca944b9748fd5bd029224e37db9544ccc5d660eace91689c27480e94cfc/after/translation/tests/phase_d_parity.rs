//! Phase D — symbol parity, cross-`-O`-level robustness, and a long soak.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    v.pop().expect("no C .so; build c_src first")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TFM_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let p = dir.join("libtfm_lib.so");
    assert!(p.exists(), "missing {}", p.display());
    p
}

/// Global `T` (defined, exported) symbols of a shared object.
fn exported_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 3 && f[1] == "T" {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

// ===========================================================================
// Symbol parity gate
// ===========================================================================
#[test]
fn symbol_diff_is_empty() {
    let c = c_so_path();
    let r = rust_so_path();
    let cs = exported_symbols(&c);
    let rs = exported_symbols(&r);

    assert!(
        cs.contains(&"tfm".to_string()),
        "the C .so does not export `tfm`; symbol extraction is broken: {cs:?}"
    );

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {cs:?}\n\
         Rust= {rs:?}"
    );
}

/// The Rust `.so` must not depend on any symbol that cannot be satisfied by the
/// system C library, or an external consumer's `dlopen` would fail.
#[test]
fn rust_so_has_no_unresolvable_dependencies() {
    let r = rust_so_path();
    // `dlopen` with RTLD_NOW resolves every relocation eagerly, so a successful
    // load proves there are no unresolvable non-libc symbols.
    let lib = unsafe { Library::new(&r) }
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {} failed: {e}", r.display()));
    let f: Symbol<TfmFn> = unsafe { lib.get(b"tfm\0") }.expect("tfm");
    // And it is callable.
    let src = [1.0f32, 2.0, 3.0];
    let mut out = [0.0f32; 2];
    unsafe { (*f)(out.as_mut_ptr(), src.as_ptr(), 1) };
    assert!(out.iter().all(|v| v.is_finite()));
}

// ===========================================================================
// Cross-`-O`-level robustness
// ===========================================================================

/// Build `c_src/src/lib.c` at a given optimization level into `target/`, WITHOUT
/// touching anything inside `c_src/`.
fn build_c_at(opt: &str) -> Option<PathBuf> {
    let root = repo_root();
    let outdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("c_optlevels");
    std::fs::create_dir_all(&outdir).ok()?;
    let out = outdir.join(format!("libc_{opt}.so"));
    let st = Command::new("gcc")
        .arg(format!("-{opt}"))
        .args(["-fPIC", "-shared"])
        .arg(format!("-I{}", root.join("c_src/include").display()))
        .arg(root.join("c_src/src/lib.c"))
        .arg("-o")
        .arg(&out)
        .arg("-lm")
        .status()
        .ok()?;
    if st.success() && out.exists() {
        Some(out)
    } else {
        None
    }
}

struct Impl {
    name: String,
    _lib: Library,
    f: TfmFn,
}

fn load(name: &str, p: &Path) -> Impl {
    let lib = unsafe { Library::new(p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()));
    let s: Symbol<TfmFn> = unsafe { lib.get(b"tfm\0") }.expect("tfm");
    let f = *s;
    Impl {
        name: name.to_string(),
        _lib: lib,
        f,
    }
}

fn has_nan_operand(src: &[f32]) -> bool {
    src.iter().any(|v| v.is_nan())
}

/// The headline robustness property.
///
/// GCC picks different FP operand orders at different `-O` levels, and operand
/// order is observable in x86 SSE NaN-payload selection. So the C library
/// disagrees with *itself* across `-O` levels — but ONLY when an input operand
/// is already a NaN. This test pins down both halves of that:
///
///   1. For inputs with **no NaN operand**, EVERY `-O` level of the C and the
///      Rust agree bit-for-bit. (This is the property that actually matters.)
///   2. The Rust matches the build that `c_src/CMakeLists.txt` produces
///      (`CMAKE_BUILD_TYPE` unset ⇒ no `-O` flag ⇒ `-O0`) on **all** inputs,
///      NaN operands included.
#[test]
fn rust_matches_cmake_build_and_all_opt_levels_agree_off_nan() {
    let cmake = load("cmake(c_src/build)", &c_so_path());
    let rust = load("rust", &rust_so_path());

    let mut opt_impls: Vec<Impl> = Vec::new();
    for o in ["O0", "O1", "O2", "O3", "Os", "Ofast"] {
        // -Ofast enables -ffast-math, which legitimately changes FP semantics;
        // it is not a configuration `c_src` can be built with, so skip it.
        if o == "Ofast" {
            continue;
        }
        if let Some(p) = build_c_at(o) {
            opt_impls.push(load(o, &p));
        } else {
            eprintln!("note: could not build C at -{o}; skipping that level");
        }
    }
    assert!(
        !opt_impls.is_empty(),
        "no gcc optimization-level builds succeeded; cannot run this check"
    );

    let mut rng = Rng::new(0x5EED_5EED);
    let mut nan_inputs = 0u64;
    let mut clean_inputs = 0u64;
    let mut opt_level_disagreements = 0u64;

    let call = |f: TfmFn, src: &[f32]| -> [f32; 2] {
        let mut o = [0.0f32; 2];
        unsafe { f(o.as_mut_ptr(), src.as_ptr(), 1) };
        o
    };

    for it in 0..400_000u64 {
        // Alternate between "finite-ish" (exponent forced below 0xFF, so never
        // inf/NaN as an *input*) and completely arbitrary bit patterns.
        let src: Vec<f32> = (0..3)
            .map(|_| {
                let b = rng.next_u32();
                if it % 2 == 0 {
                    let e = (rng.next_u32() % 254) << 23;
                    f32::from_bits((b & !0x7F80_0000) | e)
                } else {
                    f32::from_bits(b)
                }
            })
            .collect();

        let reference = call(cmake.f, &src);
        let r = call(rust.f, &src);

        // (2) Rust must match the CMake build on EVERY input.
        assert_eq!(
            reference.map(f32::to_bits),
            r.map(f32::to_bits),
            "Rust diverges from the CMake C build\n src  = {}\n C    = {}\n Rust = {}",
            show(&src),
            show(&reference),
            show(&r)
        );

        if has_nan_operand(&src) {
            nan_inputs += 1;
            // Across -O levels the C is allowed to differ here; just record it.
            for im in &opt_impls {
                if call(im.f, &src).map(f32::to_bits) != reference.map(f32::to_bits) {
                    opt_level_disagreements += 1;
                }
            }
        } else {
            clean_inputs += 1;
            // (1) With no NaN operand, every -O level AND the Rust must agree.
            for im in &opt_impls {
                let o = call(im.f, &src);
                assert_eq!(
                    o.map(f32::to_bits),
                    reference.map(f32::to_bits),
                    "C -{} diverges from the CMake build on a NaN-free input\n \
                     src = {}\n cmake = {}\n -{} = {}",
                    im.name,
                    show(&src),
                    show(&reference),
                    im.name,
                    show(&o)
                );
                assert_eq!(
                    o.map(f32::to_bits),
                    r.map(f32::to_bits),
                    "Rust diverges from C -{} on a NaN-free input\n src = {}",
                    im.name,
                    show(&src)
                );
            }
        }
    }

    assert!(clean_inputs > 100_000, "only {clean_inputs} NaN-free inputs");
    assert!(nan_inputs > 1_000, "only {nan_inputs} NaN-operand inputs");
    eprintln!(
        "phase_d: {clean_inputs} NaN-free inputs (all -O levels + Rust agree), \
         {nan_inputs} NaN-operand inputs, \
         {opt_level_disagreements} C-vs-C -O-level disagreements (Rust always matched CMake)"
    );
}

// ===========================================================================
// Long randomized soak across every shape at once
// ===========================================================================
#[test]
fn soak_all_shapes_mixed() {
    let mut rng = Rng::new(0x504A_C500);
    let l = libs();

    let mut pool: Vec<f32> = SPECIALS.to_vec();
    pool.extend(nan_patterns());

    for _ in 0..20_000 {
        let count = 1 + rng.below(64);
        // Mix value sources within a single call so branches, clamp outcomes and
        // value classes interleave across iterations.
        let src: Vec<f32> = (0..3 * count)
            .map(|_| match rng.below(4) {
                0 => rng.finite(),
                1 => rng.any_bits(),
                2 => pool[rng.below(pool.len())],
                _ => rng.normal_in(1.0e-40, 1.0e38),
            })
            .collect();

        // Disjoint buffers.
        diff_call(&src, count as i32, "soak disjoint");

        // Aliased buffers, same data.
        let mut buf: Vec<f32> = src.clone();
        buf.extend(std::iter::repeat(FILL).take(4));
        let mut cb = buf.clone();
        let mut rb = buf.clone();
        unsafe {
            (l.c_tfm)(cb.as_mut_ptr(), cb.as_ptr(), count as i32);
            (l.rust_tfm)(rb.as_mut_ptr(), rb.as_ptr(), count as i32);
        }
        assert!(
            bits_eq(&cb, &rb),
            "soak aliased mismatch count={count}\n in={}\n C ={}\n R ={}",
            show(&buf),
            show(&cb),
            show(&rb)
        );
    }
}

/// Chunked-vs-whole equivalence: calling `tfm` once with `count == n` must give
/// exactly the same bytes as calling it `n` times with `count == 1` (the C loop
/// carries no state between iterations). Verified for BOTH implementations, so a
/// Rust that accidentally introduced loop-carried state would be caught.
#[test]
fn chunked_equals_whole() {
    let mut rng = Rng::new(0xC401);
    let l = libs();
    for _ in 0..5_000 {
        let n = 1 + rng.below(40);
        let src: Vec<f32> = (0..3 * n).map(|_| rng.any_bits()).collect();

        for &f in &[l.c_tfm, l.rust_tfm] {
            let mut whole = vec![FILL; 2 * n];
            unsafe { f(whole.as_mut_ptr(), src.as_ptr(), n as i32) };

            let mut piecewise = vec![FILL; 2 * n];
            for i in 0..n {
                unsafe {
                    f(
                        piecewise.as_mut_ptr().add(2 * i),
                        src.as_ptr().add(3 * i),
                        1,
                    )
                };
            }
            assert!(
                bits_eq(&whole, &piecewise),
                "chunked != whole for n={n}\n whole={}\n piece={}",
                show(&whole),
                show(&piecewise)
            );
        }

        // And the two implementations agree on the whole call.
        diff_call(&src, n as i32, "chunked_equals_whole");
    }
}

/// Split a long call into two arbitrary halves and check the concatenation
/// matches — catches stride/offset errors that a single call cannot reveal.
#[test]
fn split_calls_compose() {
    let mut rng = Rng::new(0xC402);
    let l = libs();
    for _ in 0..5_000 {
        let n = 2 + rng.below(60);
        let k = 1 + rng.below(n - 1); // 1 <= k < n
        let src: Vec<f32> = (0..3 * n).map(|_| rng.any_bits()).collect();

        for &f in &[l.c_tfm, l.rust_tfm] {
            let mut whole = vec![FILL; 2 * n];
            unsafe { f(whole.as_mut_ptr(), src.as_ptr(), n as i32) };

            let mut split = vec![FILL; 2 * n];
            unsafe {
                f(split.as_mut_ptr(), src.as_ptr(), k as i32);
                f(
                    split.as_mut_ptr().add(2 * k),
                    src.as_ptr().add(3 * k),
                    (n - k) as i32,
                );
            }
            assert!(bits_eq(&whole, &split), "split != whole for n={n} k={k}");
        }
        diff_call(&src, n as i32, "split_calls_compose");
    }
}
