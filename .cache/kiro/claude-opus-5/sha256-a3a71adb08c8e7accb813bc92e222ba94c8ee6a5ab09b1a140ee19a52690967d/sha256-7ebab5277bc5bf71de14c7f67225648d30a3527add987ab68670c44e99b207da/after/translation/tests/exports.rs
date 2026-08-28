//! Export parity: every dynamic symbol the C `.so` defines must also be defined
//! by the Rust `.so` under the exact same name.
//!
//! Verified two ways: by comparing `nm -D --defined-only` output, and by
//! resolving each C-defined symbol against the Rust library with `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use libloading::Library;

/// Symbols that belong to the toolchain/runtime rather than the translated API.
///
/// The C `.so` exports only `custom_strdup` plus these; they are emitted by the
/// linker or the C runtime and are not part of the library's interface.
const RUNTIME_SYMBOLS: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "__bss_start__",
    "_bss_end__",
    "__end__",
    "__gnu_lto_slim",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(p.exists(), "build the C library first: {}", p.display());
    p
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("target/<profile>/deps/");
    let uplifted = deps.parent().expect("target/<profile>/").join("libdriver.so");
    let in_deps = deps.join("libdriver.so");
    // `cargo test` leaves the artefact in deps/; `cargo build` uplifts it.
    for candidate in [in_deps, uplifted] {
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("build the Rust cdylib first (looked in {} and its parent)", deps.display());
}

/// Names of all globally-defined dynamic symbols in `path`, per `nm -D`.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm must be available on PATH");
    assert!(
        out.status.success(),
        "nm -D failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // Format: "<addr> <type> <name>"
            let mut parts = line.split_whitespace();
            let _addr = parts.next()?;
            let kind = parts.next()?;
            let name = parts.next()?;
            // Only real definitions (skip undefined 'U' and anything unnamed).
            if kind == "U" {
                return None;
            }
            Some(name.to_string())
        })
        .filter(|name| !RUNTIME_SYMBOLS.contains(&name.as_str()))
        .collect()
}

#[test]
fn c_exports_are_documented_api() {
    let c_syms = defined_dynamic_symbols(&c_so());
    assert!(
        c_syms.contains("custom_strdup"),
        "C .so must export custom_strdup, found: {c_syms:?}"
    );
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so());
    let rust_syms = defined_dynamic_symbols(&rust_so());

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {rust_syms:?}"
    );
}

#[test]
fn every_c_symbol_resolves_in_rust_so() {
    let c_syms = defined_dynamic_symbols(&c_so());

    // SAFETY: loading the crate's own cdylib; it has no initialisers.
    let rust_lib = unsafe { Library::new(rust_so()) }.expect("load Rust .so");

    for name in &c_syms {
        let mut cstr = name.clone().into_bytes();
        cstr.push(0);
        // SAFETY: resolved as an opaque pointer only, never called here.
        let sym = unsafe { rust_lib.get::<*const ()>(&cstr) };
        assert!(
            sym.is_ok(),
            "symbol {name:?} is exported by the C .so but not resolvable in the Rust .so"
        );
    }
}
