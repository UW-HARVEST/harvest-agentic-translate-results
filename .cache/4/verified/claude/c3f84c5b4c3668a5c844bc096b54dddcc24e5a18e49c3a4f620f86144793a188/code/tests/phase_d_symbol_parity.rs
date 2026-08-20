//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces mechanically (not by hand-maintained list) that every dynamic
//! symbol the C shared object exports is also exported by the Rust shared
//! object under the exact same name, and that each one is actually resolvable
//! via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("c_src/build/libtranslated_rust.so"))
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().unwrap();
    let profile = deps.parent().unwrap();
    for c in [deps.join("libcleanup_lib.so"), profile.join("libcleanup_lib.so")] {
        if c.exists() {
            return c;
        }
    }
    panic!("libcleanup_lib.so not found");
}

/// Defined (exported) dynamic symbols, via `nm -D --defined-only`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let _addr = f.next()?;
            let kind = f.next()?;
            let name = f.next()?;
            // Exported code/data only.
            if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

/// Every symbol exported by the C `.so` must also be exported by the Rust `.so`.
#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_syms = defined_symbols(&c_so());
    let r_syms = defined_symbols(&rust_so());

    assert!(!c_syms.is_empty(), "nm found no exported symbols in the C .so");

    let missing: Vec<_> = c_syms.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "[D1] {} C symbol(s) MISSING from the Rust .so: {:?}\n  C exports:    {:?}\n  Rust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        r_syms
    );

    // The three known ABI entry points must be present on both sides.
    for want in ["cleanup", "print_result", "cleanup_resources"] {
        assert!(c_syms.contains(want), "[D1] C .so lost `{want}`");
        assert!(r_syms.contains(want), "[D1] Rust .so does not export `{want}`");
    }
}

/// Every exported C symbol must be resolvable via `dlsym` in BOTH libraries —
/// catches a symbol that `nm` shows but that cannot actually be looked up.
#[test]
fn d2_every_c_symbol_is_dlsym_resolvable_in_rust() {
    let c_syms = defined_symbols(&c_so());
    for name in &c_syms {
        let mut key = name.clone().into_bytes();
        key.push(0);
        for lib in [c_lib(), rust_lib()] {
            let ok = unsafe { lib.raw_symbol(&key) };
            assert!(ok, "[D2] {}: dlsym(\"{name}\") failed", lib.name);
        }
    }
}

/// The Rust `.so` must not depend on any non-libc / non-runtime symbol, which
/// would mean part of the translation lives outside the shared object.
#[test]
fn d3_rust_so_has_no_unexpected_undefined_symbols() {
    let undef = undefined_symbols(&rust_so());

    // Strip glibc/ld version suffixes: `printf@GLIBC_2.2.5` -> `printf`.
    let bare: BTreeSet<String> =
        undef.iter().map(|s| s.split('@').next().unwrap_or(s).to_string()).collect();

    let allowed_prefixes = ["_Unwind_", "_ITM_", "__cxa_", "__libc_", "_dl_", "__tls_"];
    let allowed_exact: BTreeSet<&str> = [
        "__errno_location",
        "__gmon_start__",
        "__register_atfork",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "mmap64",
        "munmap",
        "open",
        "open64",
        "posix_memalign",
        "printf",
        // Optimized builds let LLVM lower printf/snprintf to cheaper libc calls
        // (e.g. `printf("...\n")` with no conversions becomes `puts`). These are
        // semantics-preserving libc lowerings, not extra dependencies.
        "puts",
        "putchar",
        "putc",
        "fputs",
        "fputc",
        "fwrite",
        "fwrite_unlocked",
        "fputs_unlocked",
        "putchar_unlocked",
        "strchrnul",
        "__printf_chk",
        "__snprintf_chk",
        "__sprintf_chk",
        "pthread_getspecific",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_mutex_lock",
        "pthread_mutex_unlock",
        "pthread_self",
        "pthread_setspecific",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sigaction",
        "sigaltstack",
        "snprintf",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "strncmp",
        "syscall",
        "sysconf",
        "write",
        "writev",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<_> = bare
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .cloned()
        .collect();

    assert!(
        unexpected.is_empty(),
        "[D3] Rust .so has unexpected (non-libc) undefined symbols: {unexpected:?}"
    );
}

/// Documents the build-configuration surface: there is exactly ONE valid feature
/// combination, so Phases B/C under the default config cover every combination.
#[test]
fn d4_exactly_one_feature_combination() {
    let toml = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");

    // No `[features]` table => the only combination is the empty one.
    let has_features = toml
        .lines()
        .map(str::trim)
        .any(|l| l == "[features]" || l.starts_with("[features."));
    assert!(
        !has_features,
        "[D4] Cargo.toml grew a [features] table — Phases B and C must now be \
         re-run for every feature combination, and CONFIGS.md/SYMBOLS.md updated:\n{toml}"
    );

    // Nothing in src/ may branch on a feature either.
    for entry in walk(&manifest_dir().join("src")) {
        let src = std::fs::read_to_string(&entry).unwrap_or_default();
        assert!(
            !src.contains("feature = \""),
            "[D4] {} branches on a cargo feature but Cargo.toml declares none",
            entry.display()
        );
    }

    // And the C build has no configuration knobs.
    let cml = std::fs::read_to_string(manifest_dir().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cml.contains("option("),
        "[D4] CMakeLists.txt grew an option() — enumerate the new C configs"
    );
    for f in ["c_src/src/lib.c", "c_src/include/lib.h"] {
        let src = std::fs::read_to_string(manifest_dir().join(f)).expect(f);
        for tok in ["#if", "#ifdef", "#ifndef"] {
            assert!(!src.contains(tok), "[D4] {f} contains `{tok}` — enumerate the C variants");
        }
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}
