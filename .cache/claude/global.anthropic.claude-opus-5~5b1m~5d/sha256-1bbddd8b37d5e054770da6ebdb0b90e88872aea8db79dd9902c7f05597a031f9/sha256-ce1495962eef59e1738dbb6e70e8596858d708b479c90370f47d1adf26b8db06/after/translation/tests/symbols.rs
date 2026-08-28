//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces the completion gate from `SYMBOLS.md` as an executable test: every
//! symbol the C shared object exports must be exported by the Rust shared
//! object under the exact same name, and the Rust object must not have
//! unresolvable non-libc dependencies.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Symbols that `nm -D` reports for any shared object as toolchain/runtime
/// bookkeeping rather than library API.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__cxa_finalize"
            | "__gxx_personality_v0"
            | "_Unwind_Resume"
            | "__register_frame_info"
            | "__deregister_frame_info"
            | "__cxa_thread_atexit_impl"
            | "__tls_get_addr"
    ) || name.starts_with("_ZN")          // Rust/C++ mangled internals
        || name.starts_with("_R")         // Rust v0 mangling
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_GLOBAL_")
        || name.starts_with("__odr")
}

fn nm(path: &std::path::Path, extra: &str) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .args(["-D", extra, path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {} {} failed: {}",
        extra,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                // "<addr> <type> <name>"
                3 => Some((f[1].to_string(), f[2].to_string())),
                // "         U <name>"
                2 => Some((f[0].to_string(), f[1].to_string())),
                _ => None,
            }
        })
        .collect()
}

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
        .into_iter()
        .filter(|(ty, _)| {
            // Global/weak text & data: the actual ABI surface.
            matches!(ty.as_str(), "T" | "W" | "D" | "B" | "R" | "V" | "i")
        })
        .map(|(_, n)| n)
        .filter(|n| !is_toolchain_symbol(n))
        .collect()
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "no exported symbols found in the C .so — is it built?"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}\n\
         Fix by adding the #[no_mangle] extern \"C\" export, or by translating \
         the C module that was skipped.",
        missing.len(),
        missing,
        c,
        r
    );

    // Document the (allowed) extra direction too.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?} — \
         the ABI surface should match exactly"
    );
}

#[test]
fn symbol_parity_expected_surface_is_md5_digest() {
    // Pin the surface so that a future C addition cannot silently pass by being
    // absent from both sides.
    let c = exported(&c_so_path());
    assert!(
        c.contains("md5_digest"),
        "C .so does not export md5_digest: {c:?}"
    );
    assert_eq!(
        c.len(),
        1,
        "the C library is expected to export exactly one symbol (md5_digest); \
         found {c:?}. If the C gained new functions they must be translated \
         and re-verified."
    );
}

#[test]
fn rust_so_has_no_unresolvable_undefined_symbols() {
    let undef: Vec<String> = nm(&rust_so_path(), "--undefined-only")
        .into_iter()
        .map(|(_, n)| n)
        .filter(|n| !is_toolchain_symbol(n))
        .collect();

    // Everything remaining must be a plain libc/runtime import. Resolve each by
    // dlopen-ing the process itself and looking the name up.
    let this = libloading::os::unix::Library::this();
    let mut unresolved = Vec::new();
    for name in &undef {
        let mut c_name = name.clone().into_bytes();
        // nm may print versioned names as "memcpy@GLIBC_2.14"
        if let Some(pos) = c_name.iter().position(|&b| b == b'@') {
            c_name.truncate(pos);
        }
        c_name.push(0);
        let found = unsafe { this.get::<*const ()>(&c_name).is_ok() };
        if !found {
            unresolved.push(name.clone());
        }
    }
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolvable non-libc undefined symbols: {unresolved:?}"
    );
}

#[test]
fn both_libraries_load_and_expose_the_symbol() {
    // The end-to-end guarantee: an external consumer can dlopen either object
    // and dlsym the entry point.
    let libs = Libs::load();
    let _c = libs.digest(Impl::C);
    let _r = libs.digest(Impl::Rust);
}
