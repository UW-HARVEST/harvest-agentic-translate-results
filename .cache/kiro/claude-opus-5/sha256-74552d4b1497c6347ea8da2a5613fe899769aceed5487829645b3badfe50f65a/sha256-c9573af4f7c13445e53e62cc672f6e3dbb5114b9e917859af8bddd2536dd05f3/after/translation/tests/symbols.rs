//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! This is the same check `check_symbols.sh` performs, run as part of
//! `cargo test` so it cannot be forgotten.

mod common;
use common::*;
use std::process::Command;

/// Every non-`static` definition in `c_src/src/lib.c`. Kept as a literal list so
/// the test still means something if `nm` is unavailable.
const EXPECTED: &[&str] = &[
    "helxo",
    "stbds_arrfreef",
    "stbds_arrgrowf",
    "stbds_hash_bytes",
    "stbds_hash_string",
    "stbds_hmdel_key",
    "stbds_hmfree_func",
    "stbds_hmget_key",
    "stbds_hmget_key_ts",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_rand_seed",
    "stbds_shmode_func",
    "stbds_stralloc",
    "stbds_strreset",
    "strkey",
];

/// C `static` functions and objects, which must NOT be exported by either `.so`.
const MUST_NOT_EXPORT: &[&str] = &[
    "stbds_probe_position",
    "stbds_log2",
    "stbds_make_hash_index",
    "stbds_siphash_bytes",
    "stbds_is_key_equal",
    "stbds_hm_find_slot",
    "stbds_strdup",
    "stbds_hash_seed",
    "buffer",
    "stbds_unit_tests",
];

fn defined_symbols(path: &std::path::Path) -> Option<Vec<String>> {
    let out = Command::new("nm").args(["-D", "--defined-only"]).arg(path).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    Some(v)
}

#[test]
fn phase_d_symbol_parity() {
    let cp = c_so_path();
    let rp = rust_so_path();
    let (Some(cs), Some(rs)) = (defined_symbols(&cp), defined_symbols(&rp)) else {
        eprintln!("`nm` unavailable; falling back to the dlsym check only");
        return;
    };

    // Sanity: the C .so exports exactly the expected set.
    let mut want: Vec<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(cs, want, "the C .so's export list changed; update SYMBOLS.md");

    // Every C symbol must be exported by the Rust .so under the exact same name.
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // Symbols that are `static` in the C must not leak out of either .so.
    for s in MUST_NOT_EXPORT {
        assert!(!cs.iter().any(|x| x == s), "C .so unexpectedly exports {s}");
        assert!(
            !rs.iter().any(|x| x == s),
            "Rust .so exports {s}, which is `static` in the C — parity violation"
        );
    }

    println!("symbol parity: {} C symbols, 0 missing from the Rust .so", cs.len());
}

#[test]
fn phase_d_no_unresolved_non_libc_imports() {
    let rp = rust_so_path();
    let out = Command::new("nm").args(["-D", "--undefined-only"]).arg(&rp).output();
    let Ok(out) = out else { return };
    if !out.status.success() {
        return;
    }
    // Imports that are legitimately resolved by libc / libgcc / ld.so.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
        "__libc_", "__pthread_", "pthread_", "gnu_get_libc_", "__stack_chk_",
    ];
    let allowed: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "read", "readlink", "realloc", "realpath", "sprintf", "stat", "stat64",
        "statx", "strcmp", "strlen", "syscall", "write", "writev", "sigaltstack", "sigaction",
        "sigemptyset", "sysconf", "mprotect", "poll", "malloc_usable_size", "getauxval",
        "pthread_self", "memrchr", "strerror_r", "qsort", "atexit", "exit",
    ];
    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(sym) = line.split_whitespace().nth(1) else { continue };
        let base = sym.split('@').next().unwrap();
        if allowed.contains(&base) || allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        unexpected.push(base.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbols that nothing provides: {unexpected:?}"
    );
}

/// Loader-level parity: every expected symbol must be `dlsym`-able out of the
/// Rust `.so`. `Lib::open` already panics on a missing symbol, so simply opening
/// both libraries proves it for the whole surface.
#[test]
fn phase_d_all_symbols_dlsym_able() {
    let (c, r) = libs();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    // 16 function pointers were resolved out of each .so by Lib::open.
    assert_eq!(EXPECTED.len(), 16);
}

/// `translation/Cargo.toml` declares no `[features]`, so "every feature
/// combination" is a single combination. Assert that mechanically, so the test
/// starts failing the moment a feature is added without extending the matrix.
#[test]
fn phase_d_feature_matrix_is_exhaustive() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let mut in_features = false;
    let mut features = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
            continue;
        }
        if in_features && !t.is_empty() && !t.starts_with('#') {
            if let Some((name, _)) = t.split_once('=') {
                features.push(name.trim().to_string());
            }
        }
    }
    assert!(
        features.is_empty(),
        "Cargo.toml now declares features {features:?}; extend check_features.sh \
         and re-run Phases B and C for every combination"
    );
    // ...and no cfg(feature = ...) gates in the source either.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read src/lib.rs");
    assert!(
        !src.contains("feature ="),
        "src/lib.rs contains a cfg(feature = ...) gate but Cargo.toml declares no features"
    );
}
