//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces `SYMBOLS.md`: every dynamic symbol the C library exports must also
//! be exported, under the exact same name, by the Rust library, and the Rust
//! library must have no undefined non-libc symbols.

mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn c_so() -> PathBuf {
    let dir = root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    v.sort();
    assert!(!v.is_empty(), "no C .so in {}", dir.display());
    v.remove(0)
}

fn rust_so() -> PathBuf {
    for profile in ["release", "debug"] {
        let p = root().join("translation/target").join(profile).join("libupdate_frame_header_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so; run cargo build --release");
}

/// Defined dynamic symbol names, via `nm -D --defined-only`.
fn defined_syms(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, ty, name) = (it.next()?, it.next()?, it.next()?);
            // Only global/weak code and data, not local ('t','d',...) entries.
            if ty.chars().next()?.is_uppercase() { Some(name.to_string()) } else { None }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Undefined dynamic symbol names, via `nm -D --undefined-only`.
fn undefined_syms(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn sym_01_rust_exports_every_c_symbol() {
    let c = defined_syms(&c_so());
    let r = defined_syms(&rust_so());
    assert!(!c.is_empty(), "C .so exported nothing — bad build?");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C: {c:?}\n  Rust: {r:?}"
    );
}

#[test]
fn sym_02_expected_symbol_present_in_both() {
    for so in [c_so(), rust_so()] {
        let syms = defined_syms(&so);
        assert!(
            syms.iter().any(|s| s == "update_frame_header"),
            "{} does not export update_frame_header (has {syms:?})",
            so.display()
        );
    }
}

/// Every undefined symbol in the Rust `.so` must be satisfied by the platform
/// (libc / libgcc / ld.so). `ldd -r` performs the real relocation check, so it
/// is authoritative: it prints `undefined symbol:` for anything that cannot be
/// resolved at load time.
#[test]
fn sym_03_no_undefined_non_libc_symbols_in_rust() {
    let so = rust_so();

    // 1. Authoritative: nothing is left unresolved after full relocation.
    let out = Command::new("ldd").args(["-r", so.to_str().unwrap()]).output().expect("ldd");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let unresolved: Vec<&str> =
        text.lines().filter(|l| l.contains("undefined symbol")).collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved symbols after `ldd -r`:\n{}",
        unresolved.join("\n")
    );

    // 2. Every undefined symbol is either version-tagged against a platform
    //    library (`name@GLIBC_x`, `@GCC_x`, ...) or one of the unversioned
    //    compiler/runtime hooks. Anything else would mean the Rust library
    //    depends on code that was never translated.
    let platform_versions = ["@GLIBC", "@GCC_", "@GLIBCXX", "@CXXABI", "@LIBC"];
    let unversioned_ok = [
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
        "_Unwind_Resume",
        "rust_eh_personality",
    ];
    let undef = undefined_syms(&so);
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !platform_versions.iter().any(|v| s.contains(v))
                && !unversioned_ok.contains(&s.as_str())
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined symbols that are not platform-provided: {bad:?}\n\
         all undefined: {undef:?}"
    );

    // 3. And it must not import `update_frame_header` from anywhere — it has to
    //    define it itself, not forward to the C library.
    assert!(
        !undef.iter().any(|s| s.starts_with("update_frame_header")),
        "Rust .so imports update_frame_header instead of defining it: {undef:?}"
    );
}

#[test]
fn sym_04_struct_abi_matches_c_layout() {
    // Verified against a compiled C probe: size=24 align=4 off=0 4 8 12 16 20.
    assert_eq!(std::mem::size_of::<Tflac>(), 24, "sizeof(tflac)");
    assert_eq!(std::mem::align_of::<Tflac>(), 4, "alignof(tflac)");
    let t = Tflac::default();
    let base = &t as *const Tflac as usize;
    let off = |p: *const u8| p as usize - base;
    assert_eq!(off(&t.samplerate as *const u32 as *const u8), 0);
    assert_eq!(off(&t.channels as *const u32 as *const u8), 4);
    assert_eq!(off(&t.bitdepth as *const u32 as *const u8), 8);
    assert_eq!(off(&t.channel_mode as *const u8), 12);
    assert_eq!(off(&t.frame_header as *const u32 as *const u8), 16);
    assert_eq!(off(&t.cur_blocksize as *const u32 as *const u8), 20);
}

/// There is no binary target in this crate (`crate-type = ["cdylib"]` only) and
/// the C CMake project builds only a SHARED library, so there is no driver
/// executable whose stdout could be compared. This test pins that fact so the
/// requirement cannot be silently skipped if a binary is added later.
#[test]
fn sym_05_no_driver_binary_to_compare() {
    let cargo_toml = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(
        !cargo_toml.contains("[[bin]]"),
        "a [[bin]] target was added — stdout must now be compared against a C driver"
    );
    assert!(!root().join("translation/src/main.rs").exists(), "src/main.rs appeared");
    let cmake = std::fs::read_to_string(root().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "the C project now builds an executable — stdout must be compared"
    );
}
