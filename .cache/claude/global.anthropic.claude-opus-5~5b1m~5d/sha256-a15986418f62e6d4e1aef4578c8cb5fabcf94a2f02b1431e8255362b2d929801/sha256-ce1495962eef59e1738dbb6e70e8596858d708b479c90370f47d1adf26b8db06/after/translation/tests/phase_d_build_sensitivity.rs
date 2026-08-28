//! Phase D — provenance of the NaN-payload behaviour.
//!
//! `to_barycentric` is pure straight-line `float` arithmetic, so its *numeric*
//! result is fixed by the C source. Its **NaN payload**, however, is not: on
//! x86-64 the scalar SSE ops are two-operand (`mulss dst, src` computes
//! `dst = dst op src`) and, when more than one operand is NaN, the result keeps
//! the *destination* operand's payload. Which value the compiler parks in the
//! destination register is a register-allocation decision, i.e. a property of
//! the compiled binary rather than of the C source.
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no optimisation flags,
//! so the reference `.so` is `-O0`, and that is the ground truth the Rust
//! translation reproduces.
//!
//! These tests make that reasoning falsifiable:
//!
//! * `d7` — two C builds of the *same* source at different `-O` levels agree
//!   perfectly on NaN-free inputs but disagree on most NaN-carrying inputs.
//!   So the payload sensitivity is inherent to the C, not introduced by Rust.
//! * `d8` — where they disagree, Rust always sides with the `-O0` build, which
//!   is the one `CMakeLists.txt` produces.
//! * `d9` — guards that the reference build really is unoptimised, so that a
//!   future change to `CMakeLists.txt` fails loudly here with an explanation
//!   instead of producing mysterious payload mismatches everywhere.
//!
//! All of these skip cleanly if `gcc` is unavailable.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

type Fn4 = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

fn variants_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".verify/cvar")
}

/// Compile `c_src/src/lib.c` at the given optimisation level into
/// `translation/.verify/cvar/` (never touching `c_src/`). Returns `None` if
/// `gcc` is missing.
///
/// Writes to a PID-unique temporary name and then renames, so that concurrent
/// callers can never observe a half-written `.so`.
fn build_variant(opt: &str) -> Option<PathBuf> {
    let dir = variants_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let name = format!("lib{}.so", opt.trim_start_matches('-'));
    let out = dir.join(&name);
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));

    let c_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.join("c_src");
    let status = Command::new("gcc")
        .arg("-shared")
        .arg("-fPIC")
        .arg(opt)
        .arg("-I")
        .arg(c_src.join("include"))
        .arg(c_src.join("src/lib.c"))
        .arg("-o")
        .arg(&tmp)
        .status()
        .ok()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        return None;
    }
    std::fs::rename(&tmp, &out).ok()?;
    out.is_file().then_some(out)
}

/// The `-O0` and `-O2` variants, compiled exactly ONCE per test binary.
///
/// Tests run on parallel threads, so without this every test that wanted a
/// variant would race the others to `gcc -o` the same path — which is exactly
/// the flake this function exists to remove.
fn variants() -> Option<&'static (PathBuf, PathBuf)> {
    static V: OnceLock<Option<(PathBuf, PathBuf)>> = OnceLock::new();
    V.get_or_init(|| Some((build_variant("-O0")?, build_variant("-O2")?)))
        .as_ref()
}

struct Loaded {
    _lib: libloading::Library,
    f: Fn4,
}

fn load(path: &std::path::Path) -> Loaded {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let sym: libloading::Symbol<Fn4> = lib
            .get(b"to_barycentric\0")
            .expect("variant exports to_barycentric");
        let f = *sym;
        Loaded { _lib: lib, f }
    }
}

/// The NaN-payload sensitivity belongs to the C build, not to the translation.
#[test]
fn d7_c_builds_differ_from_each_other_only_on_nan_payloads() {
    let Some((p0, p2)) = variants() else {
        eprintln!("SKIP d7: gcc unavailable");
        return;
    };
    let a = load(p0);
    let b = load(p2);

    // (1) NaN-free inputs: the two builds must agree perfectly. This is the
    //     part of the behaviour the C source actually pins down.
    let mut rng = Rng::new(0xD007_0000_0000_0001);
    for _ in 0..200_000 {
        let p1 = rng.vec2(|r| r.normal_in(-20, 20));
        let p2v = rng.vec2(|r| r.normal_in(-20, 20));
        let p3 = rng.vec2(|r| r.normal_in(-20, 20));
        let p = rng.vec2(|r| r.normal_in(-20, 20));
        let r0 = unsafe { (a.f)(p1, p2v, p3, p) };
        let r2 = unsafe { (b.f)(p1, p2v, p3, p) };
        assert_eq!(
            r0.bits(),
            r2.bits(),
            "two builds of the SAME C source disagree on a NaN-free input \
             p1={p1:?} p2={p2v:?} p3={p3:?} p={p:?}: -O0={r0:?} -O2={r2:?}"
        );
    }

    // (2) NaN-carrying inputs: they disagree, a lot. If this ever stops being
    //     true the whole `*_dst_*` apparatus in src/lib.rs is unnecessary.
    let mut disagreements = 0usize;
    for _ in 0..200_000 {
        let g = |r: &mut Rng| {
            if r.chance(50) {
                r.any_nan()
            } else {
                r.normal_in(-8, 8)
            }
        };
        let p1 = rng.vec2(g);
        let p2v = rng.vec2(g);
        let p3 = rng.vec2(g);
        let p = rng.vec2(g);
        if unsafe { (a.f)(p1, p2v, p3, p) }.bits() != unsafe { (b.f)(p1, p2v, p3, p) }.bits() {
            disagreements += 1;
        }
    }
    println!("C@-O0 vs C@-O2 disagree on {disagreements}/200000 NaN-carrying inputs");
    assert!(
        disagreements > 1_000,
        "expected the two C builds to disagree on NaN payloads; got {disagreements}. \
         The payload behaviour may no longer be codegen-dependent."
    );
}

/// Rust must side with the `-O0` build — the one `c_src/CMakeLists.txt` makes —
/// on every input, including the ones where the C builds disagree.
#[test]
fn d8_rust_matches_the_o0_reference_where_c_builds_disagree() {
    let Some((p0, p2)) = variants() else {
        eprintln!("SKIP d8: gcc unavailable");
        return;
    };
    let a = load(p0);
    let b = load(p2);
    let l = libs();

    let mut rng = Rng::new(0xD008_0000_0000_0002);
    let mut checked = 0usize;
    for _ in 0..300_000 {
        let g = |r: &mut Rng| {
            if r.chance(60) {
                r.any_nan()
            } else {
                r.normal_in(-8, 8)
            }
        };
        let p1 = rng.vec2(g);
        let p2v = rng.vec2(g);
        let p3 = rng.vec2(g);
        let p = rng.vec2(g);

        let r0 = unsafe { (a.f)(p1, p2v, p3, p) };
        let r2 = unsafe { (b.f)(p1, p2v, p3, p) };
        let rr = unsafe { (l.rust)(p1, p2v, p3, p) };

        // Always: Rust == the -O0 reference.
        assert_eq!(
            rr.bits(),
            r0.bits(),
            "Rust diverges from the -O0 reference: p1={p1:?} p2={p2v:?} p3={p3:?} p={p:?} \
             -O0={r0:?} Rust={rr:?}"
        );
        if r0.bits() != r2.bits() {
            checked += 1;
        }
    }
    println!("Rust agreed with -O0 on all inputs, incl. {checked} where -O2 differs");
    assert!(checked > 1_000, "only {checked} disagreement cases sampled");
}

/// The reference build must be the unoptimised one, because that is what the
/// NaN-payload translation is calibrated against.
#[test]
fn d9_reference_cmake_build_is_unoptimised() {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/CMakeCache.txt");
    if !cache.is_file() {
        eprintln!("SKIP d9: {} not present", cache.display());
        return;
    }
    let txt = std::fs::read_to_string(&cache).expect("read CMakeCache.txt");

    let value = |key: &str| -> String {
        txt.lines()
            .find(|l| l.starts_with(&format!("{key}:")))
            .and_then(|l| l.split_once('=').map(|(_, v)| v.trim().to_string()))
            .unwrap_or_default()
    };

    let build_type = value("CMAKE_BUILD_TYPE");
    let flags = value("CMAKE_C_FLAGS");
    println!("CMAKE_BUILD_TYPE={build_type:?} CMAKE_C_FLAGS={flags:?}");

    assert!(
        build_type.is_empty(),
        "the reference build now uses CMAKE_BUILD_TYPE={build_type:?}. \
         c_src/CMakeLists.txt sets none, so the reference is expected to be -O0. \
         Optimised C builds pick different SSE destination operands and therefore \
         different NaN payloads (see test d7); src/lib.rs's *_dst_* helpers would \
         have to be recalibrated against the new build."
    );
    for bad in ["-O1", "-O2", "-O3", "-Ofast", "-ffast-math"] {
        assert!(
            !flags.contains(bad),
            "CMAKE_C_FLAGS now contains {bad}: see the message in this test"
        );
    }
}
