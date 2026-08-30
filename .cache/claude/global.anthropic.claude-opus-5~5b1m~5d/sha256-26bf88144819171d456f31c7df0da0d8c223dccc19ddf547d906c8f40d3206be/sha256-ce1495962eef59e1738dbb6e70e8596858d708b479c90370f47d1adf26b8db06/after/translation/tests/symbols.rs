// Phase D -- symbol parity and build-configuration gates.
//
// Re-derives SYMBOLS.md mechanically at test time so the artifact cannot rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols the object *defines* and exports dynamically (types T/D/B/R/W/V/i).
fn exported_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(so)
        .output()
        .expect("`nm` is required for the symbol-parity test");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let c = it.next();
            // "<addr> <type> <name>" == defined; "<type> <name>" == undefined.
            let (ty, name) = match c {
                Some(name) if a.chars().all(|ch| ch.is_ascii_hexdigit()) => (b, name),
                _ => return None,
            };
            match ty {
                "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "V" | "W" | "i" => {
                    Some(name.split('@').next().unwrap().to_string())
                }
                _ => None,
            }
        })
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm").arg("-D").arg(so).output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            match t.as_slice() {
                // `U` = hard undefined, `w` = weak undefined (optional).
                [ty, name] if *ty == "U" => Some(name.split('@').next().unwrap().to_string()),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn c_exports_are_all_present_in_rust() {
    let c = exported_defined(&c_lib());
    let r = exported_defined(&rust_lib());

    // Sanity: the C library really does export the documented entry point.
    assert!(
        c.contains("sieve"),
        "C .so does not export `sieve`; symbol extraction is broken: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let so = rust_lib();
    // `ldd -r` performs both data and function relocation checks.
    let out = Command::new("ldd").arg("-r").arg(&so).output();
    if let Ok(out) = out {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(bad.is_empty(), "unresolved symbols in {}: {bad:?}", so.display());
    }

    // Mechanically: every hard-undefined symbol must be *defined* by one of
    // the shared libraries the loader actually binds (libc, libm, libgcc_s,
    // ld.so). No hand-maintained allowlist.
    let ldd = Command::new("ldd").arg(&so).output().unwrap();
    let mut provided: BTreeSet<String> = BTreeSet::new();
    for line in String::from_utf8_lossy(&ldd.stdout).lines() {
        // "\tlibc.so.6 => /lib64/libc.so.6 (0x00007f...)"  |  "\t/lib64/ld-linux...(0x...)"
        let path = match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            [_, "=>", p, ..] => p.to_string(),
            [p, ..] if p.starts_with('/') => p.to_string(),
            _ => continue,
        };
        if Path::new(&path).exists() {
            provided.extend(exported_defined(Path::new(&path)));
        }
    }
    assert!(
        provided.len() > 100,
        "could not enumerate the loader-provided symbols ({} found)",
        provided.len()
    );
    let unexpected: Vec<String> = undefined_symbols(&so)
        .into_iter()
        .filter(|s| !provided.contains(s))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined symbols not provided by any linked library: {unexpected:?}"
    );
}

#[test]
fn library_file_names_match_the_c_target() {
    assert_eq!(c_lib().file_name().unwrap(), "libSieve.so");
    assert_eq!(rust_lib().file_name().unwrap(), "libSieve.so");
}

/// Phase D: enumerate build configurations. The crate declares no features, so
/// the default build is the only configuration -- assert that stays true, and
/// that the artifact kind/name is unchanged.
#[test]
fn no_feature_axes_exist() {
    let toml = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    let has_features = toml
        .lines()
        .any(|l| l.trim() == "[features]" || l.trim().starts_with("[features."));
    assert!(
        !has_features,
        "Cargo.toml grew a [features] table -- Phases B and C must be re-run \
         for every feature combination (see SYMBOLS.md)"
    );
    assert!(toml.contains("crate-type = [\"cdylib\"]"));
    assert!(toml.contains("name = \"Sieve\""));
}

/// Guard: the C project must still be a single translation unit exporting a
/// single function, otherwise SYMBOLS.md/ERRORS.md/CONFIGS.md are stale.
#[test]
fn c_project_shape_is_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut c_files: Vec<String> = std::fs::read_dir(root.join("c_src/src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".c"))
        .collect();
    c_files.sort();
    assert_eq!(
        c_files,
        vec!["sieve.c".to_string()],
        "new C translation units appeared -- they must be translated too"
    );
}
