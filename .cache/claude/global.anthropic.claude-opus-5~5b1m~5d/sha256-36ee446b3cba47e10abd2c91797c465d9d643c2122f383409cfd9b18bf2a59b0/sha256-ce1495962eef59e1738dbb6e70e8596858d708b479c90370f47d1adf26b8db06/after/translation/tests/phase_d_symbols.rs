//! Phase D - symbol parity, feature enumeration and completion gate.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Exported (defined) dynamic symbols of a shared object, per `nm -D`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` - is binutils installed?");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>" for defined symbols.
        if cols.len() >= 3 && cols[1].len() == 1 {
            let ty = cols[1].chars().next().unwrap();
            if matches!(ty, 'T' | 'W' | 'D' | 'B' | 'R' | 'i') {
                set.insert(cols[2].to_string());
            }
        }
    }
    set
}

/// Undefined (imported) dynamic symbols of a shared object.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if let Some(name) = line.split_whitespace().last() {
            set.insert(name.to_string());
        }
    }
    set
}

/// Phase D gate: every symbol exported by the C `.so` must also be exported by
/// the Rust `.so`, with the exact same name.
#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let l = libs();
    let c_syms = exported_symbols(&l.c_path);
    let r_syms = exported_symbols(&l.rust_path);

    eprintln!("C    .so ({}) exports {} symbol(s):", l.c_path.display(), c_syms.len());
    for s in &c_syms {
        eprintln!("    {s}");
    }
    eprintln!("Rust .so ({}) exports {} symbol(s)", l.rust_path.display(), r_syms.len());

    assert!(
        !c_syms.is_empty(),
        "nm found no exported symbols in the C .so - build problem?"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or translate \
         the missing C source.",
        missing.len()
    );

    // The one documented public symbol must actually be there (guards against a
    // vacuous pass if `nm` output parsing ever breaks).
    assert!(c_syms.contains("encode_quant"), "C .so lost encode_quant");
    assert!(r_syms.contains("encode_quant"), "Rust .so lost encode_quant");
}

/// The Rust `.so` must have no unresolved non-libc/non-runtime imports.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let l = libs();
    let undef = undefined_symbols(&l.rust_path);

    // Everything std legitimately imports from libc / libgcc / the ELF runtime.
    let allowed_exact: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];

    let mut unexpected = Vec::new();
    for s in &undef {
        let base = s.split('@').next().unwrap_or(s);
        let ok = allowed_exact.contains(&base)
            || base.starts_with("_Unwind_")
            || base.starts_with("__")   // __errno_location, __tls_get_addr, __cxa_*
            || base.starts_with("pthread_")
            || matches!(
                base,
                "abort" | "bcmp" | "calloc" | "close" | "dl_iterate_phdr" | "free"
                    | "fstat" | "fstat64" | "getcwd" | "getenv" | "gettid" | "lseek"
                    | "lseek64" | "malloc" | "memcmp" | "memcpy" | "memmove" | "memset"
                    | "mmap" | "mmap64" | "munmap" | "open" | "open64" | "posix_memalign"
                    | "read" | "readlink" | "realloc" | "realpath" | "stat" | "stat64"
                    | "statx" | "strlen" | "syscall" | "write" | "writev" | "sysconf"
                    | "signal" | "sigaction" | "sigaltstack" | "poll" | "memrchr"
                    | "getrandom" | "qsort_r" | "environ"
            );
        if !ok {
            unexpected.push(s.clone());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined (non-libc) symbols: {unexpected:?}"
    );

    // Proof the imports really do resolve: dlopen already succeeded in `libs()`.
    let a = Args::new(9, 77, -12, 34, 56, 4);
    assert_eq!(call_c(a), call_rust(a));
}

/// Phase D also requires covering every feature combination. Assert mechanically
/// that the crate declares no features, so the default build is the only one.
#[test]
fn features_declared_in_cargo_toml() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml now declares [features]; Phases B and C must be re-run for \
         every combination and CONFIGS.md/ERRORS.md updated:\n{manifest}"
    );
    // `libloading` is a dev-dependency, so it adds no feature to the cdylib.
    assert!(
        manifest.contains("[dev-dependencies]") && manifest.contains("libloading"),
        "libloading must be a dev-dependency"
    );
}

/// The Rust `.so` must be a `cdylib` exporting a C ABI, i.e. the symbol must be
/// reachable by its plain name with no Rust mangling.
#[test]
fn rust_export_is_unmangled_c_abi() {
    let l = libs();
    let r_syms = exported_symbols(&l.rust_path);
    assert!(
        r_syms.contains("encode_quant"),
        "encode_quant not exported unmangled"
    );
    // No Rust-mangled variant of the function should be the only export.
    let mangled: Vec<&String> = r_syms
        .iter()
        .filter(|s| s.contains("encode_quant") && s.as_str() != "encode_quant")
        .collect();
    eprintln!("additional encode_quant-related exports: {mangled:?}");
}

/// Cross-phase regression sweep: a compact randomized run over every axis so
/// this file alone fails if the translation regresses.
#[test]
fn phase_d_regression_sweep() {
    let mut rng = Rng::for_row("phase_d");
    for _ in 0..200_000 {
        let l = L_CLASSES[(rng.next_u64() % 13) as usize];
        let u = U_CLASSES[(rng.next_u64() % 12) as usize];
        let v = V_CLASSES[(rng.next_u64() % 9) as usize];
        let a = gen_args(l, u, v, &mut rng);
        check("phase_d", a);
    }
}
