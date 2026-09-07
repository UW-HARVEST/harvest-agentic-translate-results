//! Phase D — symbol parity and build-freshness gates, enforced as tests so they
//! cannot silently rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn dynamic_defined_symbols(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so])
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm -D {so} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_owned))
        .collect()
}

fn undefined_symbols(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so])
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .map(|s| s.split('@').next().unwrap().to_owned())
        .collect()
}

/// The symbol diff must reach **empty**: every symbol the C `.so` exports must be
/// exported by the Rust `.so` under the exact same name.
#[test]
fn symbol_parity_is_exact() {
    let c = dynamic_defined_symbols(c_so());
    let r = dynamic_defined_symbols(rust_so());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // The C surface is exactly these two functions (see SYMBOLS.md).
    assert!(c.contains("static_alias"), "C .so lost `static_alias`: {c:?}");
    assert!(c.contains("driver"), "C .so lost `driver`: {c:?}");
    assert_eq!(
        c,
        ["driver", "static_alias"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the C public surface changed; SYMBOLS.md/CONFIGS.md/ERRORS.md need updating"
    );

    // The library-internal `inner` must stay internal in both.
    assert!(!c.contains("inner"));
    assert!(!r.iter().any(|s| s == "inner" || s == "INNER"));
}

/// No missing/undefined **non-libc** symbols in the Rust `.so`.
#[test]
fn rust_so_has_no_unresolved_nonlibc_symbols() {
    // `ldd -r` reports any symbol the dynamic loader cannot resolve at all.
    let out = Command::new("ldd").args(["-r", rust_so()]).output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.to_lowercase().contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{text}"
    );

    // Cross-check: every imported symbol is a libc / language-runtime symbol.
    let undef = undefined_symbols(rust_so());
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_", "__errno", "pthread_", "stat", "fstat",
        "lseek", "mmap", "munmap", "open", "read", "write", "close", "getcwd", "getenv", "readlink",
        "realpath", "syscall", "gettid", "malloc", "calloc", "realloc", "free", "posix_memalign",
        "mem", "bcmp", "strlen", "abort", "printf", "dl_iterate_phdr", "statx", "writev",
    ];
    for s in &undef {
        assert!(
            allowed_prefixes.iter().any(|p| s.starts_with(p)),
            "Rust .so imports an unexpected non-libc symbol: {s}"
        );
    }
    // The translation must go through libc `printf`, like the C does, so stdout
    // buffering/ordering matches.
    assert!(
        undef.contains("printf"),
        "Rust .so no longer imports libc `printf`; stdout semantics may diverge"
    );
    assert!(undefined_symbols(c_so()).contains("printf"));
}

/// Guard against testing a stale artifact: both `.so`s must be at least as new as
/// their sources, otherwise every differential result above is meaningless.
#[test]
fn built_artifacts_are_not_stale() {
    fn mtime(p: &str) -> std::time::SystemTime {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("{p}: {e}"))
            .modified()
            .unwrap()
    }
    let root = env!("CARGO_MANIFEST_DIR");

    let rust_src = format!("{root}/src/lib.rs");
    assert!(
        mtime(rust_so()) >= mtime(&rust_src),
        "{} is older than src/lib.rs — run `cargo build --release` first",
        rust_so()
    );

    let c_src = format!("{root}/../c_src/src/staticalias.c");
    let c_hdr = format!("{root}/../c_src/include/staticalias.h");
    assert!(
        mtime(c_so()) >= mtime(&c_src) && mtime(c_so()) >= mtime(&c_hdr),
        "{} is older than the C sources — rebuild the C library first",
        c_so()
    );
    assert!(Path::new(c_so()).exists() && Path::new(rust_so()).exists());
}

/// There is exactly one feature configuration to verify: `Cargo.toml` declares no
/// `[features]` and the C source has no `#ifdef`. If either changes, Phases B–C
/// must be re-run per combination, so fail loudly here.
#[test]
fn feature_surface_is_singular() {
    let root = env!("CARGO_MANIFEST_DIR");
    let cargo = std::fs::read_to_string(format!("{root}/Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml gained a [features] section; re-run Phases B–C per combination"
    );
    let c = std::fs::read_to_string(format!("{root}/../c_src/src/staticalias.c")).unwrap();
    for bad in ["#ifdef", "#ifndef", "#if ", "#elif"] {
        assert!(
            !c.contains(bad),
            "c_src/src/staticalias.c gained `{bad}`; it now has conditional code paths"
        );
    }
}
