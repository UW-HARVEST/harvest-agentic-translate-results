//! Phase D — symbol parity between the two shared objects, enforced from
//! inside the test suite (not only from the driver script).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|s| s == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.into_iter().next().expect("no C .so; build c_src first")
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    let p = dir.join("libpoly_ray_lib.so");
    assert!(p.exists(), "missing {}", p.display());
    p
}

/// Exported (defined) dynamic text/weak symbols.
fn exported(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() >= 3 && (f[1] == "T" || f[1] == "W" || f[1] == "i") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Names that legitimately appear as imports in any Rust cdylib: libc, the
/// unwinder, and the glibc/ELF housekeeping stubs.
fn is_runtime_import(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    if base.starts_with("_Unwind_")
        || base.starts_with("__")
        || base.starts_with("_ITM_")
        || base.starts_with("pthread_")
    {
        return true;
    }
    const LIBC: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "stat64", "statx", "strlen", "syscall", "write", "writev", "sqrtf", "sqrt", "memcmp",
        "qsort", "exit", "sysconf", "getauxval", "poll", "readv", "fcntl", "sigaltstack",
        "sigaction", "mprotect", "pipe2", "signal", "raise", "environ",
    ];
    LIBC.contains(&base)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = find_c_so();
    let r = find_rust_so();
    let cs = exported(&c);
    let rs = exported(&r);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    eprintln!(
        "C .so exports {} symbols; Rust .so exports {}",
        cs.len(),
        rs.len()
    );
    assert!(
        !cs.is_empty(),
        "the C .so exported no symbols — is {} really built?",
        c.display()
    );
    assert!(
        missing.is_empty(),
        "\nRust .so is MISSING {} symbol(s) exported by the C .so:\n  {}\n\
         Add the #[no_mangle] extern \"C\" wrapper, or translate the missing C \
         source if a whole module was skipped.",
        missing.len(),
        missing
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    let r = find_rust_so();
    let bad: Vec<String> = undefined(&r)
        .into_iter()
        .filter(|n| !is_runtime_import(n))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc imports: {bad:?}"
    );
}

/// The two `static inline` helpers must NOT be exported, because the C `.so`
/// does not export them either — exporting extra symbols with these names
/// would be a surface mismatch in the other direction.
#[test]
fn static_inline_helpers_stay_private() {
    let c = find_c_so();
    let r = find_rust_so();
    let cs = exported(&c);
    let rs = exported(&r);
    for name in [
        "c2SignedDistPointToPlane_OneDimensional",
        "c2RayToPlane_OneDimensional",
    ] {
        assert!(!cs.contains(name), "C unexpectedly exports {name}");
        assert!(
            !rs.contains(name),
            "Rust exports {name} but the C .so keeps it static inline"
        );
    }
}

/// Records the full symbol list so `SYMBOLS.md` can be checked against reality.
#[test]
fn dump_symbol_tables() {
    let c = find_c_so();
    let r = find_rust_so();
    eprintln!("--- C exports ---");
    for s in exported(&c) {
        eprintln!("{s}");
    }
    eprintln!("--- Rust exports (C-named subset) ---");
    let cs = exported(&c);
    for s in exported(&r) {
        if cs.contains(&s) {
            eprintln!("{s}");
        }
    }
}
