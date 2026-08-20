//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Automates the `nm -D` diff documented in `SYMBOLS.md` so it can never drift.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_so_path, libs, rust_so_path};

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

fn dynamic_defined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

fn dynamic_undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "-u"], so).into_iter().collect()
}

/// Symbols that are resolved by the platform (glibc / libgcc / CRT stubs) and
/// therefore legitimately undefined in a shared object.
fn is_system_symbol(s: &str) -> bool {
    const ALLOWED_UNVERSIONED: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__tls_get_addr",
        "_edata",
        "_end",
        "__bss_start",
    ];
    s.contains("@GLIBC") || s.contains("@GCC") || ALLOWED_UNVERSIONED.contains(&s)
}

#[test]
fn both_shared_objects_exist_and_load() {
    let l = libs();
    assert!(l.c.path.exists(), "C .so missing: {}", l.c.path.display());
    assert!(
        l.rust.path.exists(),
        "Rust .so missing: {}",
        l.rust.path.display()
    );
    // Sanity: the two `wcscat` entry points are distinct code objects (this is
    // also asserted inside `libs()`).
    assert_ne!(l.c.wcscat as usize, l.rust.wcscat as usize);
}

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c = dynamic_defined(&c_so_path());
    let r = dynamic_defined(&rust_so_path());

    assert!(
        c.contains("wcscat"),
        "the C .so does not export `wcscat` — is it built? symbols: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );
}

#[test]
fn rust_so_has_no_undefined_non_system_symbols() {
    let u = dynamic_undefined(&rust_so_path());
    let bad: Vec<&String> = u.iter().filter(|s| !is_system_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-system symbols (missing implementations?): {bad:?}"
    );
}

#[test]
fn c_so_has_no_undefined_non_system_symbols() {
    // Establishes the baseline the Rust side is compared against.
    let u = dynamic_undefined(&c_so_path());
    let bad: Vec<&String> = u.iter().filter(|s| !is_system_symbol(s)).collect();
    assert!(bad.is_empty(), "C .so has unexpected undefined symbols: {bad:?}");
}

/// The Rust `.so` must not accidentally export Rust-mangled internals in place
/// of, or in addition to, the C ABI surface: every non-system symbol it
/// exports has to be one the C `.so` exports too.
#[test]
fn rust_so_exports_no_unexpected_extra_symbols() {
    let c = dynamic_defined(&c_so_path());
    let r = dynamic_defined(&rust_so_path());
    let extra: Vec<&String> = r
        .difference(&c)
        .filter(|s| {
            // Linker-provided / CRT symbols are not part of the API surface.
            !matches!(
                s.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

/// Guard against a subtle harness failure mode: glibc *also* exports a symbol
/// named `wcscat` (with a completely different, 2-argument signature). If
/// `dlsym` had resolved the "C" side to glibc's implementation instead of the
/// library's own, every differential assertion would be measuring the wrong
/// thing. Both entry points must differ from glibc's.
#[test]
fn neither_entry_point_is_glibc_wcscat() {
    let libc = match unsafe { libloading::Library::new("libc.so.6") } {
        Ok(l) => l,
        Err(e) => {
            eprintln!("skipping: cannot dlopen libc.so.6: {e}");
            return;
        }
    };
    let glibc_wcscat: usize = unsafe {
        match libc.get::<unsafe extern "C" fn()>(b"wcscat\0") {
            Ok(s) => *s as usize,
            Err(e) => {
                eprintln!("skipping: libc has no `wcscat`: {e}");
                return;
            }
        }
    };
    let l = libs();
    assert_ne!(
        l.c.wcscat as usize, glibc_wcscat,
        "the `C` side resolved to glibc's wcscat, not to {}",
        l.c.path.display()
    );
    assert_ne!(
        l.rust.wcscat as usize, glibc_wcscat,
        "the `Rust` side resolved to glibc's wcscat, not to {}",
        l.rust.path.display()
    );
}
