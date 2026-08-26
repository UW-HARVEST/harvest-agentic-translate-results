//! Phase D -- ABI parity between the C `.so` and the Rust `.so`.
//!
//! Everything here is derived from `nm -D` / `readelf -sW` on the two shared
//! objects, so `SYMBOLS.md` cannot drift away from reality without a test
//! failing.

mod common;

use common::shared::{hex, Lib, TABLES};
use common::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Dynamic symbols that are *defined* (not `U`/`w`) in `so`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 {
            continue;
        }
        // f = [addr, type, name]; skip weak symbols
        let ty = f[1];
        if ty == "w" || ty == "W" || ty == "U" || ty == "v" {
            continue;
        }
        set.insert(f[2].to_string());
    }
    set
}

/// `name -> (size, type, bind)` from `readelf -sW`, for the symbols we care about.
fn symbol_info(so: &Path) -> BTreeMap<String, (u64, String, String)> {
    let out = Command::new("readelf")
        .arg("-sW")
        .arg(so)
        .output()
        .expect("run readelf");
    assert!(out.status.success(), "readelf failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // Num: Value Size Type Bind Vis Ndx Name
        if f.len() < 8 || !f[0].ends_with(':') {
            continue;
        }
        let size: u64 = match f[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = f[7].split('@').next().unwrap_or(f[7]).to_string();
        map.entry(name).or_insert((size, f[3].to_string(), f[4].to_string()));
    }
    map
}

/// The complete list of externally visible names in `c_src/src/lib.c`: the one
/// function `include/lib.h` declares, plus every non-`static` global.
const EXPECTED: &[&str] = &[
    "pinflate",
    "cp_error_reason",
    "cp_fixed_table",
    "cp_permutation_order",
    "cp_len_extra_bits",
    "cp_len_base",
    "cp_dist_extra_bits",
    "cp_dist_base",
];

/// Everything `static` in the C source: these must NOT be exported by either
/// library.
const MUST_NOT_EXPORT: &[&str] = &[
    "cp_make_pixel_a",
    "cp_make_pixel",
    "cp_would_overflow",
    "cp_ptr",
    "cp_peak_bits",
    "cp_consume_bits",
    "cp_read_bits",
    "cp_rev16",
    "cp_build",
    "cp_stored",
    "cp_fixed",
    "cp_decode",
    "cp_dynamic",
    "cp_block",
];

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(c_so());
    let r = defined_dynamic_symbols(rust_so());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) the C .so exports: {:?}\n\
         C   ({}): {:?}\nRust ({}): {:?}",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r.iter().filter(|s| c.contains(*s)).collect::<Vec<_>>()
    );
}

#[test]
fn d2_symbol_set_is_exactly_the_c_source_surface() {
    let c = defined_dynamic_symbols(c_so());
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's exported set is not what the source says it should be"
    );
    let r = defined_dynamic_symbols(rust_so());
    for name in EXPECTED {
        assert!(
            r.contains(*name),
            "the Rust .so does not export {name}; it exports {r:?}"
        );
    }
}

#[test]
fn d3_static_functions_are_not_exported() {
    for so in [c_so(), rust_so()] {
        let set = defined_dynamic_symbols(so);
        for name in MUST_NOT_EXPORT {
            assert!(
                !set.contains(*name),
                "{} exports {name}, which is `static` in the C source",
                so.display()
            );
        }
    }
}

#[test]
fn d4_symbol_sizes_types_and_bindings_match() {
    let ci = symbol_info(c_so());
    let ri = symbol_info(rust_so());
    for name in EXPECTED {
        let c = ci
            .get(*name)
            .unwrap_or_else(|| panic!("{name} not found by readelf in the C .so"));
        let r = ri
            .get(*name)
            .unwrap_or_else(|| panic!("{name} not found by readelf in the Rust .so"));
        assert_eq!(c.2, "GLOBAL", "{name}: C binding is {} not GLOBAL", c.2);
        assert_eq!(
            r.2, "GLOBAL",
            "{name}: Rust binding is {} not GLOBAL",
            r.2
        );
        assert_eq!(c.1, r.1, "{name}: ELF type {} (C) vs {} (Rust)", c.1, r.1);
        if c.1 == "OBJECT" {
            assert_eq!(
                c.0, r.0,
                "{name}: st_size {} (C) vs {} (Rust) -- a caller indexing this \
                 exported table would go out of bounds",
                c.0, r.0
            );
        }
    }
}

#[test]
fn d5_rust_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut bad = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.is_empty() {
            continue;
        }
        // weak undefined (`w`) symbols are the usual toolchain hooks
        if f[0] == "w" || f[0] == "v" {
            continue;
        }
        let name = f.last().unwrap();
        let base = name.split('@').next().unwrap_or(name);
        let from_libc = name.contains("GLIBC")
            || name.contains("GCC")
            || base.starts_with("__")
            || base.starts_with("_ITM_")
            || matches!(
                base,
                "malloc"
                    | "calloc"
                    | "free"
                    | "realloc"
                    | "memcpy"
                    | "memset"
                    | "memmove"
                    | "memcmp"
                    | "abort"
                    | "write"
                    | "writev"
                    | "posix_memalign"
                    | "getenv"
                    | "dl_iterate_phdr"
            );
        if !from_libc {
            bad.push(name.to_string());
        }
    }
    assert!(
        bad.is_empty(),
        "the Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn d6_exported_table_contents_are_byte_identical() {
    // Not just the names and sizes: the *data* a caller reads through those
    // exported symbols must match too.
    let c = Lib::open(c_so().to_str().unwrap());
    let r = Lib::open(rust_so().to_str().unwrap());
    for spec in TABLES {
        let cb = c.table_bytes(spec.key);
        let rb = r.table_bytes(spec.key);
        assert_eq!(cb.len(), spec.len_bytes);
        assert_eq!(
            cb,
            rb,
            "{} differs:\n  C    = {}\n  Rust = {}",
            spec.symbol,
            hex(&cb),
            hex(&rb)
        );
    }
}

#[test]
fn d7_no_feature_flags_exist() {
    // Phase A: enumerate every build-time configuration. `Cargo.toml` declares
    // no `[features]` table and `c_src/CMakeLists.txt` has no `option()` or
    // `target_compile_definitions`, so the only valid combination is the default
    // (empty) one -- which is what every test in this crate runs under. This
    // test pins that down so a future feature cannot be added without also
    // extending the matrix.
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml now has a [features] table; the test matrix in \
         verify/feature_matrix.sh must be extended to cover every combination"
    );
    let cmake = std::fs::read_to_string(manifest_dir().join("c_src").join("CMakeLists.txt")).unwrap();
    for kw in ["option(", "add_definitions", "target_compile_definitions", "CMAKE_BUILD_TYPE"] {
        assert!(
            !cmake.contains(kw),
            "c_src/CMakeLists.txt now uses `{kw}`; the C side has build-time \
             configuration that the test matrix must cover"
        );
    }
    // Corollary of no CMAKE_BUILD_TYPE: no -DNDEBUG, so assert() is live.
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(c_so())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("__assert_fail"),
        "the C reference .so has no __assert_fail: the build acquired -DNDEBUG, \
         so the assert rows in ERRORS.md (E7-E16) no longer abort and the Rust \
         port's cp_assert_fail() must be removed to match"
    );
}
