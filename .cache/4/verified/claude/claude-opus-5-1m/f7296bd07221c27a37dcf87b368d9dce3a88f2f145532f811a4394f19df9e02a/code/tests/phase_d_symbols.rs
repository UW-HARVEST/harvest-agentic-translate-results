//! Phase D — dynamic-symbol parity between the C `.so`s and the Rust `.so`.
//!
//! Fails if the Rust `.so` is missing any symbol that the C build exports, or
//! if it has undefined symbols that are not plain libc / libgcc imports.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &str, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {path}: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {path} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn manifest(rel: &str) -> String {
    format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c_lib = std::env::var("CJSON_C_SO").unwrap_or_else(|_| manifest("c_src/build/libcjson.so.1.7.19"));
    let c_drv = c_driver_so_path();
    let rust = rust_driver_so_path();

    let mut expected = nm(&c_lib, &["-D", "--defined-only"]);
    expected.extend(nm(&c_drv, &["-D", "--defined-only"]));
    let actual = nm(&rust, &["-D", "--defined-only"]);

    let missing: Vec<&String> = expected.difference(&actual).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} symbol(s) exported by the C build: {missing:?}",
        missing.len()
    );

    // sanity: the reference set must be the full cJSON API, not a handful
    assert!(
        expected.len() >= 79,
        "expected at least 79 C symbols, found {}: {expected:?}",
        expected.len()
    );
    assert!(expected.contains("driver"), "driver missing from the C set");
    assert!(
        expected.contains("cJSON_ParseWithLengthOpts"),
        "cJSON_ParseWithLengthOpts missing from the C set"
    );
}

#[test]
fn phase_d_no_unresolved_non_libc_symbols_in_rust() {
    let rust = rust_driver_so_path();
    let undefined = nm(&rust, &["-D", "--undefined-only"]);
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
    ];
    let libc_like = |s: &str| {
        // glibc symbols come through as name@GLIBC_x.y
        s.contains("@GLIBC") || s.contains("@GCC") || allowed_prefixes.iter().any(|p| s.starts_with(p))
    };
    let bad: Vec<&String> = undefined.iter().filter(|s| !libc_like(s)).collect();
    assert!(
        bad.is_empty(),
        "the Rust .so has undefined non-libc symbols: {bad:?}"
    );
    // no cJSON symbol may be undefined
    let cjson_undef: Vec<&String> = undefined
        .iter()
        .filter(|s| s.starts_with("cJSON") || s.as_str() == "driver")
        .collect();
    assert!(
        cjson_undef.is_empty(),
        "cJSON symbols are undefined in the Rust .so: {cjson_undef:?}"
    );
}
