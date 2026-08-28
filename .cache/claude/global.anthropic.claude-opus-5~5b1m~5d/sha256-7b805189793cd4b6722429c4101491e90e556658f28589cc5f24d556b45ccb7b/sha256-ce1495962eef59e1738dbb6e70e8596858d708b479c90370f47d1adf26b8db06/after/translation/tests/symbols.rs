//! Phase D — symbol parity between the two shared objects.
//!
//! Re-derives the `SYMBOLS.md` claim with `nm -D` at test time so the document
//! cannot silently go stale.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("running `nm` failed — is binutils installed?");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only code/data the library actually publishes.
            (kind == "T" || kind == "t").then(|| name.to_string())
        })
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("running `nm` failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c = nm_defined(&common::c_so_path());
    let r = nm_defined(&common::rust_so_path());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    let extra: Vec<_> = r.difference(&c).cloned().collect();

    println!("C exports {} symbols, Rust exports {}", c.len(), r.len());
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
         translate the missing C source.",
        missing.len(),
        missing
    );
    assert!(
        extra.is_empty(),
        "Rust .so exports {} symbol(s) the C .so does not: {:?}",
        extra.len(),
        extra
    );
    assert_eq!(c.len(), 31, "expected the 31 symbols enumerated in SYMBOLS.md");
}

/// The Rust `.so` must not import anything beyond libc / libgcc-unwind, which
/// would indicate an un-translated dependency.
#[test]
fn no_undefined_non_libc_symbols() {
    let undef = nm_undefined(&common::rust_so_path());
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__"];
    let allowed_libc: BTreeSet<&str> = [
        "malloc", "calloc", "realloc", "free", "posix_memalign", "memcpy", "memmove", "memset",
        "bcmp", "memcmp", "strlen", "abort", "getenv", "getcwd", "readlink", "realpath", "open64",
        "open", "close", "read", "write", "writev", "lseek64", "lseek", "stat64", "fstat64",
        "statx", "mmap64", "mmap", "munmap", "syscall", "gettid", "dl_iterate_phdr", "sqrtf",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
        "sysconf", "poll", "sigaction", "sigaltstack", "mprotect", "pthread_self",
        "pthread_attr_init", "pthread_attr_destroy", "pthread_getattr_np",
        "pthread_attr_getstack",
    ]
    .into_iter()
    .collect();

    let mut bad = Vec::new();
    for s in &undef {
        let base = s.split('@').next().unwrap_or(s);
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_libc.contains(base) {
            continue;
        }
        bad.push(s.clone());
    }
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbol(s): {bad:?}"
    );
}

/// Sanity: both libraries can actually be dlopened and every one of the 31
/// symbols resolves through `libloading` (this is what the rest of the suite
/// relies on).
#[test]
fn both_libraries_load_all_symbols() {
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // Touch one symbol from each so the lazy binding is forced.
    unsafe {
        let _ = (c.c2RotIdentity)();
        let _ = (r.c2RotIdentity)();
    }
}
