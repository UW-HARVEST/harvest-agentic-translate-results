//! The Rust `cdylib` must export every dynamic symbol the C shared library
//! exports, under exactly the same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined dynamic symbols of `path`, as reported by `nm -D --defined-only`.
fn exported_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>", or " <type> <name>" for undefined values.
            let name = line.split_whitespace().last()?;
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn rust_library_exports_every_c_symbol() {
    let libs = common::libs();
    // Re-derive the paths the harness used, so the assertion covers the exact
    // artifacts the behavioural tests loaded.
    let _ = libs;

    let c_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so");
    let rust_path = {
        let exe = std::env::current_exe().unwrap();
        let mut dir = exe.parent().unwrap().to_path_buf();
        if dir.file_name().is_some_and(|n| n == "deps") {
            dir.pop();
        }
        dir.join("libdriver.so")
    };

    let c_syms = exported_symbols(&c_path);
    let rust_syms = exported_symbols(&rust_path);

    // Symbols the C source itself defines. Toolchain-injected entries such as
    // `_init`/`_fini` or `__bss_start` are not part of the translated API.
    let toolchain = [
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__odr_asan_gen_driver",
    ];
    let expected: BTreeSet<_> = c_syms
        .iter()
        .filter(|s| !toolchain.contains(&s.as_str()))
        .cloned()
        .collect();

    assert!(
        expected.contains("driver") && expected.contains("printLine"),
        "C library did not export the documented API: {expected:?}"
    );

    let missing: Vec<_> = expected.difference(&rust_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust library is missing C-exported symbols {missing:?}\n  C:    {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}
