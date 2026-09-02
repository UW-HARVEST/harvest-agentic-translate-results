//! Phase A / Phase D — exported-symbol parity between the C `.so` and the Rust
//! `.so`, re-checked mechanically with `nm -D` on every `cargo test` run.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 && matches!(f[1], "T" | "D" | "B" | "R" | "W") {
            set.insert(f[2].to_string());
        }
    }
    set
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name. The diff must be empty in BOTH directions.
#[test]
fn symbol_parity_is_exact() {
    let cp = common::c_so_path();
    let rp = common::rust_so_path();
    let c = defined_symbols(&cp);
    let r = defined_symbols(&rp);

    let missing: Vec<&String> = c.difference(&r).collect();
    let extra: Vec<&String> = r.difference(&c).collect();

    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {missing:?}",
        missing.len()
    );
    assert!(
        extra.is_empty(),
        "{} symbol(s) exported only by the Rust .so: {extra:?}",
        extra.len()
    );
    assert_eq!(c.len(), 38, "unexpected C symbol count: {c:?}");
}

/// The Rust `.so` must not depend on anything outside libc / the unwinder.
#[test]
fn no_undefined_non_libc_symbols_in_rust_so() {
    let rp = common::rust_so_path();
    let undef = undefined_symbols(&rp);

    // Everything a Rust cdylib legitimately imports from the platform.
    let allowed_prefix = [
        "_Unwind_",
        "_ITM_",
        "__cxa_",
        "__gmon_start__",
        "__tls_get_addr",
        "__errno_location",
        "pthread_",
        "__libc_",
    ];
    let allowed_libc: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sqrtf", "sqrt", "fmaxf", "fminf", "cos", "sin", "cosf", "sinf",
        "__stack_chk_fail", "getrandom", "sysconf", "sigaltstack", "mprotect", "pipe2", "poll",
        "sigaction", "sigaddset", "sigemptyset", "raise", "exit",
    ]
    .into_iter()
    .collect();

    let mut bad = Vec::new();
    for s in &undef {
        let base = s.split('@').next().unwrap_or(s);
        if allowed_prefix.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_libc.contains(base) {
            continue;
        }
        bad.push(s.clone());
    }
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

/// Sanity: both libraries actually load and every symbol resolves as a
/// callable function pointer through `dlsym`.
#[test]
fn both_libraries_load_and_resolve_all_symbols() {
    let p = common::pair();
    assert_eq!(p.c.which, "C");
    assert_eq!(p.rs.which, "Rust");
    // Trivially exercise a symbol from each so the linker really bound them.
    unsafe {
        common::same("c2V(1,2)", (p.c.c2V)(1.0, 2.0), (p.rs.c2V)(1.0, 2.0));
        common::same("capsule", (p.c.capsule)(0.0, 0.0, 1.0, 1.0, 1.0), (p.rs.capsule)(0.0, 0.0, 1.0, 1.0, 1.0));
    }
}
