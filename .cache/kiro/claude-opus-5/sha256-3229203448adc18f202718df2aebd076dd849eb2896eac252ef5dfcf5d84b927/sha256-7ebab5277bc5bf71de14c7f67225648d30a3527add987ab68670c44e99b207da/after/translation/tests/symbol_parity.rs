//! Verifies that the Rust `cdylib` exports every dynamic symbol the reference
//! C shared library exports, under the same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::Implementations;

/// Dynamic symbols *defined* by a shared object, as reported by `nm -D`.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("`nm` must be available to compare exported symbols");

    assert!(
        output.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("nm output must be UTF-8");
    stdout
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>"
            let mut fields = line.split_whitespace();
            let _addr = fields.next()?;
            let kind = fields.next()?;
            let name = fields.next()?;
            // Ignore the linker/loader bookkeeping symbols that neither library
            // declares in its source.
            if is_toolchain_symbol(name) {
                return None;
            }
            Some(format!("{kind} {name}"))
        })
        .collect()
}

/// Symbols injected by the C runtime, the compiler or the linker rather than by
/// the translated source.
fn is_toolchain_symbol(name: &str) -> bool {
    const PREFIXES: [&str; 4] = ["_ITM_", "__gmon_", "__cxa_", "_Jv_"];
    const EXACT: [&str; 6] = [
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__odr_asan_gen_",
    ];

    PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || EXACT.contains(&name)
}

#[test]
fn rust_exports_every_c_symbol() {
    let impls = Implementations::load();

    let c_symbols = defined_dynamic_symbols(&impls.c_path);
    let rust_symbols = defined_dynamic_symbols(&impls.rust_path);

    println!("C   ({}): {c_symbols:?}", impls.c_path.display());
    println!("Rust({}): {rust_symbols:?}", impls.rust_path.display());

    assert!(
        c_symbols.contains("T hsv_to_rgb"),
        "the C library should export hsv_to_rgb as a text symbol, got {c_symbols:?}"
    );

    let missing: Vec<&String> = c_symbols.difference(&rust_symbols).collect();
    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing symbols exported by the C library: {missing:?}\n\
         C   : {c_symbols:?}\n\
         Rust: {rust_symbols:?}"
    );
}
