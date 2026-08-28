//! Symbol parity: every symbol the C `.so` exports must also be exported by the
//! Rust `.so` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Dynamic symbols defined (not imported) by `path`.
fn exported_symbols(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only strong, globally visible code/data symbols.
            matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = root().join("c_src/build/libdriver.so");
    let rs_release = root().join("translation/target/release/libdriver.so");
    let rs_debug = root().join("translation/target/debug/libdriver.so");
    let rs = if rs_release.exists() { rs_release } else { rs_debug };

    assert!(c.exists(), "build the C library first: {}", c.display());
    assert!(rs.exists(), "build the Rust cdylib first: {}", rs.display());

    let c_syms = exported_symbols(&c);
    let rs_syms = exported_symbols(&rs);

    // Symbols the C toolchain injects into every shared object; not part of the
    // library's API surface.
    let toolchain: BTreeSet<&str> = [
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__odr_asan_gen___",
    ]
    .into_iter()
    .collect();

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !toolchain.contains(s.as_str()) && !rs_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\n\
         C exports: {c_syms:?}\nRust exports: {rs_syms:?}"
    );

    // Sanity: the documented API really is there.
    for want in ["w_utf8_drop", "w_utf8_filter"] {
        assert!(c_syms.contains(want), "C .so lost {want}");
        assert!(rs_syms.contains(want), "Rust .so lost {want}");
    }
}

/// Both exports must be reachable through `dlopen`/`dlsym` on each library.
#[test]
fn both_libraries_resolve_via_dlsym() {
    let _ = common::Impls::load();
}
