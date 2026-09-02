//! Phase D — symbol parity between the two shared objects, checked as a test
//! rather than asserted in prose, so it cannot drift out of date.
//!
//! Three things are verified:
//!  1. Every symbol the C `.so` exports is exported by the Rust `.so` under the
//!     exact same name. The diff must be empty.
//!  2. The Rust `.so` exports no extra public symbols that would widen the ABI.
//!  3. The Rust `.so` has no unresolved dependencies: `dlopen` with `RTLD_NOW`
//!     resolves every relocation eagerly, so it fails outright if any undefined
//!     symbol cannot be satisfied by libc.

mod common;

use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, CString};
use std::path::Path;
use std::process::Command;

use common::{c_so_path, rust_so_path};

/// Runs `nm -D --defined-only` and returns the exported symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    parse_nm(&String::from_utf8_lossy(&out.stdout))
}

/// Runs `nm -D --undefined-only` and returns the imported symbol names.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(out.status.success(), "nm -D --undefined-only failed");
    parse_nm(&String::from_utf8_lossy(&out.stdout))
}

/// `nm` lines look like `0000000000001139 T fma_array` or `                 U printf@GLIBC_2.2.5`.
/// Take the last whitespace-separated field and drop any `@VERSION` suffix.
fn parse_nm(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Symbols the toolchain emits for its own bookkeeping rather than as part of
/// the library's interface. Neither side is expected to match the other on these.
fn is_toolchain_artifact(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Unwind_")
        || name == "__gmon_start__"
        || name == "_init"
        || name == "_fini"
        || name == "__bss_start"
        || name == "_edata"
        || name == "_end"
}

/// The complete public interface, taken from the C source. `nm` output is
/// compared against this too, so a future C change that adds a function is
/// caught rather than silently accepted by a set-equality check that happens to
/// still balance.
const EXPECTED_PUBLIC: &[&str] = &["fma_array", "call_fma", "driver"];

#[test]
fn d1_c_symbols_are_all_exported_by_rust() {
    let c_so = c_so_path();
    let r_so = rust_so_path();

    let c_syms: BTreeSet<String> = defined_symbols(&c_so)
        .into_iter()
        .filter(|s| !is_toolchain_artifact(s))
        .collect();
    let r_syms: BTreeSet<String> = defined_symbols(&r_so)
        .into_iter()
        .filter(|s| !is_toolchain_artifact(s))
        .collect();

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) the C .so exports: {missing:?}\n\
         Per the Phase A rule these must be fixed by adding the #[no_mangle] \
         wrapper if the implementation exists, or by translating the missing C \
         source if a whole module was skipped — never by stubbing.\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {r_syms:?}",
        missing.len(),
        c_so.display(),
        r_so.display(),
    );

    let extra: Vec<&String> = r_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports {} symbol(s) the C .so does not, widening the ABI: {extra:?}",
        extra.len()
    );

    assert_eq!(
        c_syms,
        EXPECTED_PUBLIC.iter().map(|s| s.to_string()).collect(),
        "the C .so's exported set is not the three functions defined in \
         c_src/src/driver.c; the artifacts need re-deriving"
    );
}

#[test]
fn d2_rust_so_has_no_unresolved_symbols() {
    let r_so = rust_so_path();

    // Eager binding: dlopen fails if any relocation cannot be resolved, which is
    // a stronger and less brittle check than filtering `nm` output by hand.
    const RTLD_NOW: c_int = 2;
    const RTLD_LOCAL: c_int = 0;
    unsafe extern "C" {
        fn dlopen(file: *const c_char, mode: c_int) -> *mut core::ffi::c_void;
        fn dlclose(handle: *mut core::ffi::c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }

    let path = CString::new(r_so.to_str().expect("utf-8 path")).expect("no NUL in path");
    let handle = unsafe {
        dlerror(); // clear any stale error
        dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL)
    };
    if handle.is_null() {
        let msg = unsafe {
            let e = dlerror();
            if e.is_null() {
                "unknown".to_string()
            } else {
                std::ffi::CStr::from_ptr(e).to_string_lossy().into_owned()
            }
        };
        panic!(
            "dlopen({}, RTLD_NOW) failed, so the Rust .so has unresolved symbols: {msg}",
            r_so.display()
        );
    }
    unsafe { dlclose(handle) };
}

#[test]
fn d3_rust_imports_the_same_libc_entry_points_as_c() {
    // `driver.c` uses exactly two libc facilities: `sscanf` and `printf`. glibc's
    // <stdio.h> redirects `sscanf` to `__isoc99_sscanf` for C99+, so the C object
    // imports that name — and the Rust must import the same one rather than the
    // legacy `sscanf`, since the two are distinct implementations at run time.
    let c_undef = undefined_symbols(&c_so_path());
    let r_undef = undefined_symbols(&rust_so_path());

    for name in ["__isoc99_sscanf", "printf"] {
        assert!(
            c_undef.contains(name),
            "expected the C .so to import {name}; it imports {c_undef:?}"
        );
        assert!(
            r_undef.contains(name),
            "the Rust .so does not import {name}, so it is not going through the \
             same libc code path as the C build"
        );
    }
    assert!(
        !r_undef.contains("sscanf"),
        "the Rust .so imports the legacy `sscanf`, but the C build imports \
         `__isoc99_sscanf`; these are different functions in glibc"
    );
}
