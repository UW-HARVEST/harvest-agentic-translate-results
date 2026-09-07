//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (not from a hand-maintained list) that every dynamic
//! symbol the C library exports is also exported by the Rust library under the
//! exact same name, and that each one is actually resolvable via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Global (`T`/`D`/`B`/`W`) defined dynamic symbols of `so`, per `nm -D`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm; is binutils installed?");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Skip the Rust/C runtime bookkeeping symbols that are not part of
            // the library's own API surface.
            let noise = [
                "_init",
                "_fini",
                "__bss_start",
                "_edata",
                "_end",
                "__cxa_finalize",
                "_ITM_registerTMCloneTable",
                "_ITM_deregisterTMCloneTable",
                "__gmon_start__",
                "rust_eh_personality",
            ];
            if noise.contains(&name) || name.starts_with("_ZN") || name.starts_with("__rust") {
                return None;
            }
            match kind {
                "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i" => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm reported no exported symbols for the C library — build is wrong"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );

    // Every C symbol must also be resolvable through dlsym on the Rust lib.
    let lib = unsafe { libloading::Library::new(rust_so_path()) }.expect("dlopen rust .so");
    for name in &c {
        let mut z = name.clone().into_bytes();
        z.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&z) };
        assert!(sym.is_ok(), "`{name}` not resolvable via dlsym in the Rust .so");
    }

    println!("symbol parity OK: {} C symbol(s), 0 missing -> {c:?}", c.len());
}

#[test]
fn phase_d_rust_has_no_unresolved_non_libc_imports() {
    // `ldd -r` reports unresolved relocations; there must be none beyond what
    // the dynamic loader satisfies from libc/libgcc.
    let out = std::process::Command::new("ldd")
        .arg("-r")
        .arg(rust_so_path())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "Rust .so has undefined symbols:\n{text}"
    );

    let out = std::process::Command::new("ldd")
        .arg("-r")
        .arg(c_so_path())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "C .so has undefined symbols:\n{text}"
    );
}

#[test]
fn phase_d_project_builds_no_binary_so_stdout_gate_is_vacuous() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; the C-vs-Rust stdout comparison gate is \
         no longer vacuous and must be implemented"
    );
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[[bin]]"),
        "the crate now builds a binary; add a stdout differential test"
    );
    assert!(
        !root.join("src/main.rs").exists(),
        "src/main.rs appeared; add a stdout differential test"
    );
}

#[test]
fn phase_d_no_feature_flags_means_one_configuration() {
    let toml =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml gained a [features] table; Phases B and C must be re-run for \
         every feature combination (see run_all_features.sh)"
    );
}
