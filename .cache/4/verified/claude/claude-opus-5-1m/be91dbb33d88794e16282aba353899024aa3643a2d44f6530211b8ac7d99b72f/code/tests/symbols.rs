// Phase D — symbol parity between the C `.so` and the Rust `.so`, asserted from
// inside the test suite (not only by an external script), plus guards against
// testing a stale artifact.

mod common;

use common::*;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("`nm` must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(t), Some(n)) => Some(format!("{t} {n}")),
                // "<type> <name>" (no address)
                (Some(t), Some(n), None) => Some(format!("{t} {n}")),
                _ => None,
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn names(syms: &[String]) -> Vec<String> {
    let mut v: Vec<String> = syms
        .iter()
        .map(|s| s.split_whitespace().nth(1).unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every exported symbol of the C `.so` must also be exported by the Rust `.so`,
/// with the exact same name. The diff must be empty.
#[test]
fn d1_exported_symbol_parity() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    let (cn, rn) = (names(&c), names(&r));

    assert!(
        !cn.is_empty(),
        "no exported symbols found in the C .so — build it first"
    );
    let missing: Vec<&String> = cn.iter().filter(|s| !rn.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {cn:?}\n\
         Rust: {rn:?}"
    );
    // the one public entry point, with the same binding/type letter
    assert!(c.contains(&"T sieve".to_string()), "C symbols: {c:?}");
    assert!(r.contains(&"T sieve".to_string()), "Rust symbols: {r:?}");
    // the C library exports exactly one public symbol (single translation unit)
    assert_eq!(cn, vec!["sieve".to_string()], "C export set changed: {cn:?}");
}

/// The Rust `.so` must have no unresolved (non-libc/non-runtime) imports.
#[test]
fn d2_no_unresolved_imports() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("`nm` must be available");
    let text = String::from_utf8_lossy(&out.stdout);
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_"];
    let libc_like = [
        "printf", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcpy", "memmove", "memset",
        "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific", "read", "readlink",
        "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sysconf", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_mutex_destroy", "pthread_mutex_init", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
        "sigaction", "sigaltstack", "sysfs", "poll", "environ", "signal", "raise", "getrandom",
        "clock_gettime", "nanosleep", "sched_yield", "memrchr", "memchr", "strerror_r", "dlsym",
    ];
    let mut unexpected = Vec::new();
    for line in text.lines() {
        let sym = line.split_whitespace().last().unwrap_or("");
        let base = sym.split('@').next().unwrap_or(sym);
        if base.is_empty() {
            continue;
        }
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) || libc_like.contains(&base) {
            continue;
        }
        unexpected.push(base.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbols: {unexpected:?}"
    );

    // and the loader can actually resolve everything
    let ldd = Command::new("ldd").arg("-r").arg(rust_so_path()).output();
    if let Ok(o) = ldd {
        let s = String::from_utf8_lossy(&o.stdout).to_string()
            + &String::from_utf8_lossy(&o.stderr);
        assert!(
            !s.contains("undefined symbol"),
            "ldd -r reports unresolved symbols:\n{s}"
        );
    }
}

/// Guard: the artifact under test must correspond to the current `src/lib.rs`.
/// (`cargo test` does not rebuild cdylib targets, which would otherwise let a
/// stale `.so` pass every differential test.)
#[test]
fn d3_artifact_under_test_is_fresh() {
    let so = rust_so_path();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let so_m = std::fs::metadata(&so).unwrap().modified().unwrap();
    let src_m = std::fs::metadata(&src).unwrap().modified().unwrap();
    assert!(
        so_m >= src_m,
        "the Rust .so under test ({}) is older than src/lib.rs — differential \
         results would be meaningless",
        so.display()
    );
}

/// The cargo-produced cdylib (when up to date) must behave identically to the
/// artifact used by the rest of the suite, i.e. the packaging is equivalent.
#[test]
fn d4_cargo_artifact_matches() {
    let Some(cargo_so) = cargo_so_path() else {
        eprintln!("note: no cargo cdylib present; skipping");
        return;
    };
    if !cargo_so_is_fresh(&cargo_so) {
        eprintln!(
            "note: {} is stale (run `cargo build` first); skipping",
            cargo_so.display()
        );
        return;
    }
    let lib = unsafe { libloading::Library::new(&cargo_so) }.expect("dlopen cargo cdylib");
    let f: libloading::Symbol<SieveFn> =
        unsafe { lib.get(b"sieve\0") }.expect("cargo cdylib must export `sieve`");
    let g: SieveFn = *f;
    let (c, _) = funcs();
    for val in [9, 0, -1, -37, 7, 2_147_483_639] {
        let expect = run(c, val);
        let got = run(g, val);
        assert_eq!(expect, got, "cargo artifact diverged for sieve({val})");
    }
}
