//! Phase D — symbol parity enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and must be resolvable via `dlsym`.

mod harness;

use harness::Pair;
use std::path::PathBuf;
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__"))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn find_c_so() -> PathBuf {
    let build_dir = crate_root().join("../c_src/build");
    let mut c: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("c_src/build missing - build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    c.sort();
    c.pop().expect("no .so in c_src/build")
}

fn find_rust_so() -> PathBuf {
    for profile in ["release", "debug"] {
        let p = crate_root()
            .join("target")
            .join(profile)
            .join("libfindrep_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libfindrep_lib.so not found");
}

#[test]
fn phase_d_rust_exports_every_c_symbol() {
    let c_syms = exported_symbols(&find_c_so());
    let r_syms = exported_symbols(&find_rust_so());

    assert!(!c_syms.is_empty(), "C .so exported nothing - bad build?");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {:?}\nC exports:    {:?}\nRust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        r_syms
    );

    // The eight documented entry points must all be present.
    for expected in [
        "add_to_accumulator",
        "divide_multiplier",
        "find_and_replace_char",
        "findrep",
        "multiply_with_multiplier",
        "process_octal_string",
        "subtract_from_accumulator",
        "validate_and_normalize",
    ] {
        assert!(c_syms.iter().any(|s| s == expected), "C missing {expected}");
        assert!(r_syms.iter().any(|s| s == expected), "Rust missing {expected}");
    }
}

#[test]
fn phase_d_rust_has_no_unresolved_non_libc_imports() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", find_rust_so().to_str().unwrap()])
        .output()
        .expect("nm failed");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && s != "w")
        .collect();
    // Everything Rust imports must come from libc / the unwinder / libgcc.
    let allowed_prefixes = [
        "_", "abort", "malloc", "free", "realloc", "calloc", "memcpy", "memmove", "memset",
        "memcmp", "strlen", "write", "writev", "close", "open", "read", "mmap", "munmap", "mremap",
        "sysconf", "getenv", "pthread", "dl_iterate_phdr", "gettid", "syscall", "poll", "readlink",
        "getcwd", "sigaction", "sigaltstack", "signal", "raise", "bcmp", "posix_memalign",
        "environ", "stat", "fstat", "lseek", "pipe", "dup", "exit", "unlink", "mkdir", "rmdir",
        "rename", "opendir", "readdir", "closedir", "chdir", "getpid", "getuid", "clock_gettime",
        "nanosleep", "sched_yield", "prctl", "madvise", "mprotect", "getrandom", "gnu_get_libc",
        "realpath", "statx", "fstat64", "stat64", "lseek64", "mmap64", "open64", "gettid",
    ];
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so imports unexpected non-libc symbols: {suspicious:?}"
    );
}

#[test]
fn phase_d_all_symbols_callable_via_dlsym() {
    // `Pair::fresh()` resolves all eight symbols in BOTH libraries via dlsym
    // and panics if any is missing, so construction alone proves callability.
    let p = Pair::fresh();
    assert_eq!(p.c.name(), "C");
    assert_eq!(p.rust.name(), "Rust");
    // Smoke-call each one through the FFI boundary.
    harness::same("dlsym: normalize", p.c.normalize(7), p.rust.normalize(7));
    harness::same("dlsym: add", p.c.add(1, 2), p.rust.add(1, 2));
    harness::same("dlsym: mul", p.c.mul(3, 4), p.rust.mul(3, 4));
    harness::same("dlsym: sub", p.c.sub(5, 6), p.rust.sub(5, 6));
    harness::same("dlsym: div", p.c.div(7, 8), p.rust.div(7, 8));
    harness::same_bytes("dlsym: octal", &p.c.octal(0o123), &p.rust.octal(0o123));
    let s = harness::buf_from(b"Octal: 0123", 32);
    harness::same_bytes(
        "dlsym: replace",
        &p.c.replace(&s, b'O' as i32),
        &p.rust.replace(&s, b'O' as i32),
    );
    harness::same("dlsym: findrep", p.c.findrep(1, 2, 3, 4), p.rust.findrep(1, 2, 3, 4));
}
