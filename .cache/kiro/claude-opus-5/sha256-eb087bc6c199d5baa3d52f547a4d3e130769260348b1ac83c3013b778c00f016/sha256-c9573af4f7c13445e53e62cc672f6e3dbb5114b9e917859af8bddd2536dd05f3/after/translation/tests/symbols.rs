// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every symbol the C library exports must be exported by the Rust library under
// the exact same name, and the Rust library must have no unresolved
// (non-libc/libgcc) imports.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    root().join("c_src/build/libdriver.so")
}

fn rust_sos() -> Vec<PathBuf> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let v: Vec<PathBuf> = ["release", "debug"]
        .iter()
        .map(|p| target.join(p).join("libdriver.so"))
        .filter(|p| p.is_file())
        .collect();
    assert!(!v.is_empty(), "no Rust cdylib built; run `cargo build --release`");
    v
}

/// `nm -D --defined-only <so>` reduced to the set of symbol names.
fn defined_symbols(so: &Path) -> Vec<String> {
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
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    assert!(
        c.contains(&"driver".to_string()),
        "sanity: C .so should export `driver`, got {c:?}"
    );
    for so in rust_sos() {
        let r = defined_symbols(&so);
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "{}: symbols exported by the C .so but missing from the Rust .so: {:?}",
            so.display(),
            missing
        );
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_imports() {
    // Anything the Rust cdylib imports must come from the C runtime / unwinder,
    // i.e. it must resolve at load time.  `ldd -r` reports genuinely unresolved
    // symbols; an empty report is the requirement.
    for so in rust_sos() {
        let out = Command::new("ldd").arg("-r").arg(&so).output().expect("run ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("undefined symbol") && !text.contains("not found"),
            "{}: unresolved imports reported by `ldd -r`:\n{}",
            so.display(),
            text
        );
        // Belt and braces: nothing imported may be a `driver`-namespace symbol,
        // which would mean part of the library was left un-translated.
        let undef = undefined_symbols(&so);
        let suspicious: Vec<&String> = undef
            .iter()
            .filter(|s| s.starts_with("driver") || s.starts_with("raw_double"))
            .collect();
        assert!(
            suspicious.is_empty(),
            "{}: library-internal symbols left undefined: {:?}",
            so.display(),
            suspicious
        );
    }
}

#[test]
fn phase_d_both_profiles_are_covered_when_built() {
    // Guards against silently testing only one artifact.
    let sos = rust_sos();
    eprintln!(
        "Rust cdylibs under test: {:?}",
        sos.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()
    );
    assert!(!sos.is_empty());
}
