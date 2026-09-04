//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//! The diff of C-exported symbols minus Rust-exported symbols must be EMPTY.

mod common;
use common::{c_so_path, rust_so_path};

use std::collections::BTreeSet;
use std::process::Command;

/// libc / CRT / toolchain bookkeeping that is not part of the library's own API.
fn is_runtime_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__gmon_start__",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "_Jv_RegisterClasses",
        "__register_frame_info",
        "__deregister_frame_info",
        "rust_eh_personality",
    ];
    EXACT.contains(&name)
        || name.starts_with("__rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
        || name.starts_with("__libc_")
}

/// Global dynamic symbols DEFINED by `so`.
fn exported(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            // "<addr> <kind> <name>"
            (Some(_), Some(k), Some(n)) => (k, n),
            // "<kind> <name>" (undefined-address form)
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        // Only strong global text/data symbols; skip weak (w/W/v/V) entries.
        if !matches!(kind, "T" | "D" | "B" | "R" | "G" | "S" | "i") {
            continue;
        }
        if is_runtime_symbol(name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn phase_d_symbol_parity() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    eprintln!("C   exports ({}): {:?}", c.len(), c);
    eprintln!("Rust exports ({}): {:?}", r.len(), r);

    // The C library must actually export something, or the test is vacuous.
    assert!(!c.is_empty(), "no symbols found in the C .so — is nm working?");
    assert!(
        c.contains("next_double"),
        "C .so does not export next_double: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or translate \
         the missing C source."
    );
}

/// `cn_rnd_next` is `static` in C, so it must NOT be exported by either side.
/// Exporting it from Rust would be an ABI mismatch, not an improvement.
#[test]
fn phase_d_static_helper_not_exported() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());
    assert!(!c.contains("cn_rnd_next"), "unexpected: C exports the static helper");
    assert!(
        !r.contains("cn_rnd_next"),
        "Rust exports `cn_rnd_next`, but the C helper is `static` and unexported"
    );
}

/// Undefined symbols in the Rust `.so` must all be satisfiable by the platform
/// runtime — nothing from the library's own translation may be left dangling.
///
/// Two independent checks:
///  1. Every undefined symbol carries a version tag from a system library
///     (`@GLIBC_*`, `@GCC_*`, ...) or is a known weak CRT/unwind hook, and none
///     is Rust-name-mangled (a mangled undefined symbol would mean a module the
///     translation references but never defines).
///  2. `dlopen(RTLD_NOW)` succeeds, which forces the loader to bind *every*
///     undefined symbol eagerly. This is the authoritative proof that nothing
///     is unresolvable, independent of any name allowlist.
#[test]
fn phase_d_no_undefined_non_libc_symbols() {
    let so = rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm --undefined-only failed");
    let text = String::from_utf8_lossy(&out.stdout);

    // Weak / unversioned hooks the toolchain always emits and the loader is
    // happy to leave unbound.
    const ALLOWED_UNVERSIONED: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__tls_get_addr",
        "_Jv_RegisterClasses",
    ];

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let kind = match it.next() {
            Some(k) => k,
            None => continue,
        };
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        // Weak undefined symbols never need to resolve.
        if matches!(kind, "w" | "v") {
            continue;
        }
        let bare = name.split('@').next().unwrap_or(name);

        // A Rust-mangled undefined symbol means a missing module.
        assert!(
            !(bare.starts_with("_ZN") && bare.contains("17h")) && !bare.starts_with("_RN"),
            "Rust .so has an UNDEFINED Rust-mangled symbol `{bare}` — a translated \
             module appears to be referenced but not defined"
        );
        assert_ne!(
            bare, "next_double",
            "Rust .so imports `next_double` instead of defining it"
        );
        assert_ne!(
            bare, "cn_rnd_next",
            "Rust .so imports `cn_rnd_next` — the static helper must be defined locally"
        );

        // Versioned system symbol (…@GLIBC_2.x, …@GCC_3.0) => platform runtime.
        if name.contains('@') || ALLOWED_UNVERSIONED.contains(&bare) {
            continue;
        }
        suspicious.push(bare.to_string());
    }

    // Anything left unversioned and strong is reported for inspection; on this
    // toolchain the set is expected to be empty or purely libc/unwind.
    let unexpected: Vec<&String> = suspicious
        .iter()
        .filter(|n| {
            !n.starts_with("__")
                && !n.starts_with("_Unwind_")
                && !n.starts_with("rust_eh_personality")
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexplained undefined symbols: {unexpected:?}"
    );

    // Authoritative: eager binding of every undefined symbol must succeed.
    let lib = unsafe {
        libloading::os::unix::Library::open(
            Some(&so),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
    };
    let lib = lib.unwrap_or_else(|e| {
        panic!(
            "dlopen(RTLD_NOW) on {} failed — some undefined symbol cannot be resolved: {e}",
            so.display()
        )
    });
    let sym: Result<libloading::os::unix::Symbol<unsafe extern "C" fn(*mut u8) -> f64>, _> =
        unsafe { lib.get(b"next_double\0") };
    assert!(sym.is_ok(), "next_double not resolvable under RTLD_NOW");
}
