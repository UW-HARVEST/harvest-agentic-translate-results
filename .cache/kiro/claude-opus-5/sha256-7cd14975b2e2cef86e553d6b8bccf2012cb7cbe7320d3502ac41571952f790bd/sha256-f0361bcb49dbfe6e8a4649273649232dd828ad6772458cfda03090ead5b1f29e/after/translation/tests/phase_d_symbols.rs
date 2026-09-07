//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Automates the `nm -D` diff so it is re-checked on every test run rather
//! than being a one-off manual observation recorded in `SYMBOLS.md`.

mod harness;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn c_so() -> std::path::PathBuf {
    let build = harness::repo_root().join("c_src").join("build");
    let mut v: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    v.sort();
    v.pop().expect("no C .so; build c_src first")
}

fn rust_so() -> std::path::PathBuf {
    // Same release artifact the differential tests load.
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("cargo build --release --lib");
    assert!(status.success());
    let exe = std::env::current_exe().unwrap();
    let target = exe.parent().unwrap().parent().unwrap().parent().unwrap();
    let p = target.join("release").join(format!(
        "{}bitwriter_add_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(p.exists(), "missing {}", p.display());
    p
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );

    // Sanity: the surface is not empty (guards against nm silently returning
    // nothing and the diff trivially "passing").
    assert!(c.contains("bitwriter_add"), "C .so must export bitwriter_add");
    assert!(
        r.contains("bitwriter_add"),
        "Rust .so must export bitwriter_add"
    );
    assert_eq!(c.len(), 1, "C surface changed; update SYMBOLS.md: {c:?}");
}

#[test]
fn rust_has_no_unresolved_non_libc_symbols() {
    let u = nm_undefined(&rust_so());
    // Everything legitimately undefined in a cdylib comes from libc / libgcc's
    // unwinder / the dynamic loader.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location", "statx",
        "gettid", "syscall",
    ];
    let libc_syms: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap", "open64",
        "posix_memalign", "pthread_key_create", "pthread_key_delete", "pthread_setspecific",
        "read", "readlink", "realloc", "realpath", "stat64", "strlen", "write", "writev",
        "pthread_getattr_np", "pthread_self", "pthread_attr_destroy", "pthread_attr_getstack",
        "sigaction", "sigaltstack", "sysconf", "mprotect", "raise", "signal", "getauxval",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_destroy", "memrchr",
        "__libc_start_main",
    ]
    .into_iter()
    .collect();

    let bad: Vec<&String> = u
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !allowed_prefixes.iter().any(|p| base.starts_with(p)) && !libc_syms.contains(base)
        })
        .collect();

    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols (a partially translated module \
         would show up here): {bad:?}"
    );
}

#[test]
fn cargo_toml_declares_no_features() {
    // The completion gate requires Phases B-C to hold under EVERY feature
    // combination. Assert mechanically that the set of combinations is exactly
    // one (the default), so `check_features.sh` cannot silently go stale if a
    // `[features]` table is added later.
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !manifest.lines().any(|l| l.trim() == "[features]"),
        "Cargo.toml now has a [features] table -- CONFIGS.md/SYMBOLS.md and \
         check_features.sh must be updated to cross every combination"
    );
}
