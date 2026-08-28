//! Step 8: every dynamic symbol the C `.so` exports must also be exported by
//! the Rust `cdylib` under the exact same name.
//!
//! Compiler/runtime-provided symbols (the C runtime's `_init`/`_fini`
//! bookkeeping and the toolchain's unwinding/personality helpers) are not part
//! of the translated API surface and are filtered out on both sides.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols injected by the linker/CRT rather than by the source under test.
const TOOLCHAIN_SYMBOLS: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__gmon_start__",
    "__cxa_finalize",
    "rust_eh_personality",
    "_Unwind_Resume",
];

fn is_toolchain(name: &str) -> bool {
    TOOLCHAIN_SYMBOLS.contains(&name)
        || name.starts_with("_ZN")            // Rust/C++ mangled internals
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_R")             // Rust v0 mangling
        || name.starts_with("__libc")
        || name.starts_with("_Unwind")
}

/// Defined, exported (`T`/`D`/`B`/`R`/`W`) dynamic symbols of a shared object.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Uppercase type letters denote global (externally visible) symbols.
            let kind = kind.chars().next()?;
            (kind.is_ascii_uppercase() && !is_toolchain(name)).then(|| name.to_string())
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    // Reuse the harness' discovery logic by loading the pair first: it panics
    // with actionable instructions if either artifact is missing.
    let _pair = common::Pair::load();

    let c_so = common::c_library_path();
    let rust_so = common::rust_library_path();

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(
        !c_syms.is_empty(),
        "no symbols detected in {} — nm parsing is broken",
        c_so.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {missing:?}\n  C: \
         {c_syms:?}\n  Rust: {rust_syms:?}",
        rust_so.display(),
        c_so.display()
    );
}
