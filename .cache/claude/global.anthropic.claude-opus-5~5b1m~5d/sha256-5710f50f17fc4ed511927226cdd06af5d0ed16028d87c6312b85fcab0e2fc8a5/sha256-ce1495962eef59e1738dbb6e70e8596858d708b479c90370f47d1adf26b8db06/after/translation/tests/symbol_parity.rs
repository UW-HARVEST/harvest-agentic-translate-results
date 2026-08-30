//! Phase D — symbol parity, enforced as a test so it cannot silently regress.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Runs `nm -D <so>` and returns the set of *defined, globally exported* symbols
/// (types T/D/B/R/G/S), i.e. the actual ABI surface. Weak toolchain boilerplate
/// (`w`) and imports (`U`) are excluded.
fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (a, b) = (it.next(), it.next());
        let (kind, name) = match (a, b, it.next()) {
            // "<addr> T name"
            (Some(_), Some(k), Some(n)) => (k, n),
            // "         w name" / "U name"
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        // Keep only strong, defined, exported symbols.
        if matches!(kind, "T" | "D" | "B" | "R" | "G" | "S" | "i") {
            set.insert(name.to_string());
        }
    }
    set
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name. The diff must be EMPTY.
#[test]
fn phase_d_symbol_diff_is_empty() {
    let l = libs();
    let c = exported_symbols(&l.c_path);
    let r = exported_symbols(&l.rust_path);

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );

    // The C library exports exactly one function; make sure that is still true,
    // so this test fails loudly if the C surface ever grows.
    assert_eq!(
        c,
        ["driver".to_string()].into_iter().collect::<BTreeSet<_>>(),
        "the C .so's exported surface changed; re-derive SYMBOLS.md"
    );
    assert!(r.contains("driver"));
}

/// The Rust `.so` must not import anything outside libc / the compiler runtime.
#[test]
fn phase_d_no_unresolved_non_libc_imports() {
    let l = libs();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&l.rust_path)
        .output()
        .expect("failed to run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap_or(name);
        let ok = name.contains("@GLIBC_")
            || name.contains("@GCC_")
            || base.starts_with("_ITM_")
            || base.starts_with("_Unwind_")
            || base.starts_with("__")
            || base == "gettid"
            || base == "statx";
        if !ok {
            suspicious.push(name.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so imports non-libc/non-runtime symbols: {suspicious:?}"
    );
}

/// `dlopen`-ing the Rust `.so` with `RTLD_NOW` proves every relocation resolves,
/// which is the runtime equivalent of `ldd -r` reporting nothing.
#[test]
fn phase_d_rust_so_loads_with_rtld_now() {
    let l = libs();
    // libloading's `Library::new` uses RTLD_NOW | RTLD_LOCAL on unix, so the fact
    // that `libs()` succeeded already proves this; assert the symbol is callable.
    let out = {
        let a = CBuf::new(b"hello");
        let b = CBuf::new(b"l");
        rust_out(a.ptr(), b.ptr())
    };
    assert_eq!(out, b"2\n".to_vec());
    assert!(l.rust_path.exists());
}
