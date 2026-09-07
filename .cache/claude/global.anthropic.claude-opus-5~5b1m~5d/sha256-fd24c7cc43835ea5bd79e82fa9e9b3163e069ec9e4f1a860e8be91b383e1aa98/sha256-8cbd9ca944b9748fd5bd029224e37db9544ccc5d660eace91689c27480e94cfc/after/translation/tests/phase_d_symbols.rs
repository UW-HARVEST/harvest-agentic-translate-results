//! Phase D — symbol parity, enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same linker name, and every one must be callable through
//! `dlsym`.

mod common;

use common::*;
use std::process::Command;

/// All exported (defined, dynamic) symbol names of `so`, via `nm -D`.
fn exported(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|n| {
            // Ignore CRT/compiler bookkeeping that is not part of either API.
            !matches!(
                n.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src")
        .join("build");
    std::fs::read_dir(&dir)
        .expect("build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .expect("no .so in c_src/build")
}

fn rust_so() -> std::path::PathBuf {
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["debug", "release"] {
        let c = target.join(p).join("libcomplexmode_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("run `cargo build` first");
}

/// The complete list of functions defined in `c_src/src/lib.c`, transcribed by
/// hand from the source so the test fails if either `.so` silently loses one.
const EXPECTED: [&str; 7] = [
    "check_permissions",
    "compare_operations",
    "complexmode",
    "copy_and_sum",
    "create_result_string",
    "multiply_with_log",
    "safe_add",
];

#[test]
fn d01_c_exports_exactly_the_expected_set() {
    let got = exported(&c_so());
    assert_eq!(
        got, EXPECTED,
        "the C .so's export set changed; SYMBOLS.md and the tests need updating"
    );
}

#[test]
fn d02_rust_exports_every_c_symbol() {
    let c = exported(&c_so());
    let r = exported(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}",
        missing.len()
    );
}

#[test]
fn d03_every_symbol_is_dlsym_resolvable_in_both() {
    for name in EXPECTED {
        let mut key = name.as_bytes().to_vec();
        key.push(0);
        for imp in BOTH {
            let r = unsafe {
                imp.lib()
                    .get::<unsafe extern "C" fn()>(&key)
                    .map(|s| *s as usize)
            };
            assert!(
                r.is_ok(),
                "{name} is not dlsym-resolvable in the {} .so",
                imp.name()
            );
        }
    }
}

#[test]
fn d04_rust_has_no_unresolvable_undefined_symbols() {
    // Loading the .so at all proves the dynamic linker resolved every
    // undefined symbol; `dlopen` would have failed otherwise.
    let _ = libs();
    // Belt and braces: check `ldd` reports no "not found" entries.
    let out = Command::new("ldd")
        .arg(rust_so())
        .output()
        .expect("ldd must be available");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "unresolved shared-library dependency in the Rust .so:\n{text}"
    );
}
