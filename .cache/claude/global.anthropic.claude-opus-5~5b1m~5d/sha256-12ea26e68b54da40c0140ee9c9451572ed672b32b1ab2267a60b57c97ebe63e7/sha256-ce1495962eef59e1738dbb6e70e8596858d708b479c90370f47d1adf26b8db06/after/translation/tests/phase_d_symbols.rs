//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The check is executed as a real test so it cannot rot: `nm -D` is run on both
//! shared objects and the set of exported names the C library defines must be a
//! subset of (in practice: equal to) what the Rust library defines.  In addition
//! every symbol is `dlsym`'d from both objects.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let p = manifest().join("../c_src/build/libdriver.so");
    assert!(p.exists(), "C .so missing at {p:?}");
    p
}

fn find_rust_so() -> PathBuf {
    let mut cands = vec![];
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1).take(3) {
            cands.push(anc.join("libdriver.so"));
        }
    }
    cands.push(manifest().join("target/release/libdriver.so"));
    cands.push(manifest().join("target/debug/libdriver.so"));
    cands
        .into_iter()
        .find(|p| p.exists())
        .expect("Rust cdylib libdriver.so not found")
}

/// Names of dynamic symbols DEFINED (i.e. not `U`/undefined) by `so`.
fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` — is binutils installed?");
    assert!(
        out.status.success(),
        "nm -D failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Undefined (imported) dynamic symbols of `so`.
fn undefined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The five functions `driver.c` defines with external linkage.
const EXPECTED_API: &[&str] = &["bad", "driver", "good", "printIntLine", "printLine"];

#[test]
fn d1_c_so_exports_exactly_the_expected_api() {
    let c = defined_dynamic_symbols(&find_c_so());
    let mut want: Vec<String> = EXPECTED_API.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        c, want,
        "the C .so's exported surface changed; SYMBOLS.md / the tests must be updated"
    );
}

#[test]
fn d2_symbol_diff_is_empty() {
    let c_so = find_c_so();
    let r_so = find_rust_so();
    let c = defined_dynamic_symbols(&c_so);
    let r = defined_dynamic_symbols(&r_so);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({r_so:?}) is MISSING symbols exported by the C .so ({c_so:?}): {missing:?}\n\
         C   = {c:?}\nRUST = {r:?}"
    );

    // Also report the reverse direction; the Rust cdylib must not leak extra
    // non-libc public symbols that could collide with a consumer's.
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports EXTRA public symbols not present in the C .so: {extra:?}"
    );
}

#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    let r_so = find_rust_so();
    // If any import could not be satisfied, dlopen (done by `rust_api`) would
    // already have failed; assert that explicitly.
    let _ = rust_api();

    // Every undefined symbol must be resolvable in the already-loaded process
    // image (libc / libdl / ld.so).  `ldd -r` reports unresolved ones.
    let out = Command::new("ldd").arg("-r").arg(&r_so).output();
    if let Ok(out) = out {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("undefined symbol"),
            "ldd -r reports unresolved symbols in {r_so:?}:\n{text}"
        );
        assert!(
            !text.contains("not found"),
            "ldd -r reports missing libraries for {r_so:?}:\n{text}"
        );
    }

    // Informational: the C .so imports `puts` because GCC rewrites
    // printf("%s\n", s); the Rust .so legitimately imports only `printf`.
    let c_undef = undefined_dynamic_symbols(&find_c_so());
    assert!(
        c_undef.iter().any(|s| s.starts_with("printf")) || c_undef.iter().any(|s| s.starts_with("puts")),
        "unexpected C import set: {c_undef:?}"
    );
}

#[test]
fn d4_every_symbol_is_dlsym_able_from_both_objects() {
    // `common::c_api()` / `common::rust_api()` resolve all five symbols and
    // panic with the offending name if any is absent.
    let c = c_api();
    let r = rust_api();
    assert_eq!(c.name, "C libdriver.so");
    assert_eq!(r.name, "Rust libdriver.so");
    // Sanity: the resolved pointers really are distinct implementations.
    assert_ne!(c.driver as usize, r.driver as usize);
    assert_ne!(c.print_line as usize, r.print_line as usize);
    assert_ne!(c.print_int_line as usize, r.print_int_line as usize);
    assert_ne!(c.bad as usize, r.bad as usize);
    assert_ne!(c.good as usize, r.good as usize);
}

/// `(name, bind, type, visibility)` for every defined dynamic symbol.
fn elf_dynsym_attrs(so: &PathBuf) -> Vec<(String, String, String, String)> {
    let out = Command::new("readelf")
        .arg("--wide")
        .arg("--dyn-syms")
        .arg(so)
        .output()
        .expect("failed to run `readelf`");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut v = Vec::new();
    for line in text.lines() {
        // Num: Value Size Type Bind Vis Ndx Name
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 8 || !f[0].ends_with(':') {
            continue;
        }
        let (ty, bind, vis, ndx, name) = (f[3], f[4], f[5], f[6], f[7]);
        if ndx == "UND" {
            continue;
        }
        // strip any @VERSION suffix
        let name = name.split('@').next().unwrap_or(name).to_string();
        v.push((name, bind.to_string(), ty.to_string(), vis.to_string()));
    }
    v.sort();
    v.dedup();
    v
}

#[test]
fn d6_elf_symbol_kind_and_binding_match() {
    let c = elf_dynsym_attrs(&find_c_so());
    let r = elf_dynsym_attrs(&find_rust_so());
    for name in EXPECTED_API {
        let cs = c
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("{name} not found in C .so dynsyms: {c:?}"));
        let rs = r
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("{name} not found in Rust .so dynsyms: {r:?}"));
        assert_eq!(
            (&cs.1, &cs.2, &cs.3),
            (&rs.1, &rs.2, &rs.3),
            "symbol `{name}` differs in (bind, type, visibility): C={cs:?} RUST={rs:?}"
        );
        assert_eq!(cs.2, "FUNC", "`{name}` should be a FUNC in the C .so");
        assert_eq!(cs.1, "GLOBAL", "`{name}` should be GLOBAL in the C .so");
        assert_eq!(cs.3, "DEFAULT", "`{name}` should have DEFAULT visibility");
    }
}

#[test]
fn d5_no_feature_gated_code_paths_exist() {
    // Guard the Phase-D claim in SYMBOLS.md: if a `[features]` table is ever
    // added to Cargo.toml, this test fails so the feature matrix gets extended.
    let toml = std::fs::read_to_string(manifest().join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml now declares [features]; extend the Phase D feature matrix \
         (check_all_features.sh) and SYMBOLS.md accordingly"
    );
    let lib = std::fs::read_to_string(manifest().join("src/lib.rs")).unwrap();
    assert!(
        !lib.contains("feature ="),
        "src/lib.rs now has cfg(feature = ..) gates; extend the Phase D feature matrix"
    );
}
