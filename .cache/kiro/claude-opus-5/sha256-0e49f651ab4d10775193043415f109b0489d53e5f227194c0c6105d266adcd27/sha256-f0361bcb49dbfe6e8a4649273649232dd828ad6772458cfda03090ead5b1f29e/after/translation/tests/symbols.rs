//! Phase D — symbol parity enforced as a test, so the gate cannot silently rot.
//!
//! Reads `nm -D` on both shared objects and asserts every symbol the C `.so`
//! exports is exported by the Rust `.so` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep global text/data/bss/weak symbols; drop local ones.
            match kind {
                "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i" | "u" => {
                    Some(name.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let libs = common::Libs::load();
    let c = defined_dynamic_symbols(&libs.c_path);
    let r = defined_dynamic_symbols(&libs.rust_path);

    assert!(
        c.contains("driver"),
        "the C .so should export `driver`; got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );
}

#[test]
fn rust_so_has_no_unresolvable_symbols() {
    let libs = common::Libs::load();
    let undef = undefined_dynamic_symbols(&libs.rust_path);

    // Everything the Rust cdylib imports must come from libc / libgcc / the
    // dynamic loader. Anything else would mean a missing translation unit.
    let allowed_prefixes = [
        "_", "__", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat",
        "getcwd", "getenv", "gettid", "lseek", "malloc", "mem", "mmap", "munmap", "open",
        "posix_memalign", "printf", "pthread_", "read", "realloc", "realpath", "stat", "statx",
        "strlen", "syscall", "write",
    ];
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports symbols that are neither libc nor unwind: {unexpected:?}"
    );

    // `ldd -r` is the authoritative check that nothing is unresolvable.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&libs.rust_path)
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reported unresolved symbols for the Rust .so:\n{text}"
    );
}
