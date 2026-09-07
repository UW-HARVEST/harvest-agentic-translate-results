//! Phase D — symbol parity, enforced from inside the test suite so it cannot
//! silently rot.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        return PathBuf::from(p);
    }
    let dir = root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected one .so in {}, got {found:?}", dir.display());
    found.pop().unwrap()
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["release", "debug"] {
        let c = base.join(p).join("libenvy_lib.so");
        if c.is_file() {
            return c;
        }
    }
    panic!("libenvy_lib.so not built");
}

#[test]
fn symbol_parity_is_exact() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C symbols:    {c:?}\n\
         Rust symbols: {r:?}",
        missing.len()
    );

    // The C library's full exported surface, as an explicit guard against the
    // C growing a symbol the Rust never learns about.
    let expected = [
        "apply_bit_operations",
        "envy",
        "init_config_from_env",
        "parse_env_numeric",
        "perform_operation",
    ];
    for e in expected {
        assert!(c.contains(&e.to_string()), "C .so no longer exports {e}: {c:?}");
        assert!(r.contains(&e.to_string()), "Rust .so does not export {e}: {r:?}");
    }
    assert_eq!(
        c.len(),
        expected.len(),
        "the C .so exports symbols not listed in SYMBOLS.md: {c:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_project_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so())
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let undefined: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    // Anything that is neither a libc/glibc-versioned import nor a known
    // toolchain runtime stub would be an unresolved piece of the translation.
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__"];
    let allowed_exact = [
        "getenv", "atoi", "strchr", "printf", "fprintf", "snprintf", "stderr", "malloc", "calloc",
        "realloc", "free", "posix_memalign", "memcpy", "memmove", "memset", "bcmp", "strlen",
        "abort", "puts", "write", "writev", "read", "open64", "close", "lseek64", "fstat64",
        "stat64", "statx", "mmap64", "munmap", "getcwd", "readlink", "realpath", "syscall",
        "gettid", "dl_iterate_phdr", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "pthread_getspecific", "sysconf", "getauxval", "memrchr",
        "strnlen", "sigaction", "sigaltstack", "mprotect", "pipe2", "poll", "fwrite", "fputs",
        "environ", "qsort", "raise", "signal", "backtrace",
    ];
    let mut suspicious = Vec::new();
    for u in undefined {
        let base = u.split('@').next().unwrap();
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_exact.contains(&base) {
            continue;
        }
        suspicious.push(u.to_string());
    }
    assert!(
        suspicious.is_empty(),
        "the Rust .so has undefined symbols that are not libc/runtime imports: {suspicious:?}"
    );
}
