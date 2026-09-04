//! Phase D — exported-symbol parity, asserted as a test so it cannot drift.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dynamic_symbols(path: &PathBuf, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    std::env::var("DRIVER_C_SO").map(PathBuf::from).unwrap_or_else(|_| {
        manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so")
    })
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().and_then(|d| d.parent()).unwrap();
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["release", "debug"] {
        let c = manifest_dir().join("target").join(p).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("could not locate the Rust libdriver.so");
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = dynamic_symbols(&c_so(), &["-D", "--defined-only"]);
    let r = dynamic_symbols(&rust_so(), &["-D", "--defined-only"]);
    assert_eq!(
        c,
        vec![
            "allocate_matrix",
            "driver",
            "free_matrix",
            "initialize_matrix_from_string",
            "matrix_to_string",
            "multiply_matrices",
            "write_to_file",
        ],
        "the C .so's exported surface changed; SYMBOLS.md needs updating"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    assert_eq!(c, r, "exported-symbol sets differ (extra in Rust: {:?})",
        r.iter().filter(|s| !c.contains(s)).collect::<Vec<_>>());
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = dynamic_symbols(&rust_so(), &["-D", "--undefined-only"]);
    // Everything the Rust .so imports must come from glibc / libgcc_s, i.e. be
    // resolvable in the current process. Anything else would be a missing
    // translation unit.
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__", "pthread_"];
    let known_libc = [
        "abort", "atoi", "bcmp", "calloc", "close", "dl_iterate_phdr", "fclose", "fopen",
        "fprintf", "free", "fstat", "fstat64", "fwrite", "getcwd", "getenv", "gettid", "lseek64",
        "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap", "open",
        "open64", "perror", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "snprintf", "stat", "stat64", "statx", "stderr", "strcat", "strdup", "strerror", "strlen",
        "strtok_r", "syscall", "write", "writev", "sigaltstack", "sigaction", "sysconf",
        "pthread_self", "mprotect", "getpid", "raise", "signal", "abs",
    ];
    let mut unexpected = Vec::new();
    for s in &undef {
        let name = s.split('@').next().unwrap_or(s);
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        if known_libc.contains(&name) {
            continue;
        }
        unexpected.push(s.clone());
    }
    assert!(
        unexpected.is_empty(),
        "the Rust .so imports symbols that are neither libc nor Rust runtime: {unexpected:?}"
    );
}
