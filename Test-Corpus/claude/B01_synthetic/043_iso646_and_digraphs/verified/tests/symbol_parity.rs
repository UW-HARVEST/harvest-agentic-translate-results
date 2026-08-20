// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Phase D - symbol parity between the C shared object and the Rust cdylib.
//
// Keeping this as a test rather than a one-off shell command means the parity
// claim in SYMBOLS.md cannot silently rot.

mod common;

use common::{c_exe, c_so, rust_so};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols an ELF object gets from the toolchain rather than from its source.
/// They are not part of anybody's API and are absent or present depending on how
/// the object was linked, so they are excluded from the comparison.
const TOOLCHAIN_NOISE: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__gmon_start__",
    "__cxa_finalize",
    "_init",
    "_fini",
    "_edata",
    "_end",
    "__bss_start",
    "__TMC_END__",
    "_DYNAMIC",
    "_GLOBAL_OFFSET_TABLE_",
];

fn nm(args: &[&str], obj: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(obj)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {obj:?}: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {obj:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        // Strip the `@GLIBC_2.x` version suffix so names compare cleanly.
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .filter(|s| !TOOLCHAIN_NOISE.contains(&s.as_str()))
        .collect()
}

fn defined_dynamic(obj: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], obj).into_iter().collect()
}

/// The completion gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name.
#[test]
fn c_exports_are_all_present_in_rust() {
    let c = defined_dynamic(c_so());
    let r = defined_dynamic(&rust_so());

    // Guard against a vacuous pass: if `nm` produced nothing, the comparison
    // would trivially "succeed".
    assert!(
        !c.is_empty(),
        "no dynamic symbols found in the C .so at {:?} - the parity check would be vacuous",
        c_so()
    );
    assert!(
        c.contains("driver") && c.contains("main"),
        "the C .so must export `driver` and `main`; got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   exports: {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );
}

/// Every undefined symbol in the Rust `.so` must be satisfiable, i.e. none of
/// its dependencies are dangling. `ldd -r` performs both data and function
/// relocation checks and names anything it cannot resolve.
#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let so = rust_so();
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&so)
        .output()
        .unwrap_or_else(|e| panic!("run ldd -r on {so:?}: {e}"));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "the Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}

/// The C *executable* defines the same two functions with external linkage. This
/// documents where the FFI surface came from and fails if the C source ever
/// stops providing them (which would mean the tables in SYMBOLS.md are stale).
#[test]
fn c_executable_defines_driver_and_main() {
    let defined: BTreeSet<String> = nm(&["--defined-only"], c_exe()).into_iter().collect();
    for want in ["driver", "main"] {
        assert!(
            defined.contains(want),
            "the C executable must define `{want}`; got {defined:?}"
        );
    }
}

/// Both objects must agree on the *number* of API symbols too, so the Rust side
/// exporting a superset of unrelated names does not go unnoticed.
#[test]
fn api_surface_is_exactly_driver_and_main() {
    let expected: BTreeSet<String> = ["driver", "main"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        defined_dynamic(c_so()),
        expected,
        "the C .so's exported API changed"
    );
    assert_eq!(
        defined_dynamic(&rust_so()),
        expected,
        "the Rust .so exports something other than the C API surface"
    );
}
