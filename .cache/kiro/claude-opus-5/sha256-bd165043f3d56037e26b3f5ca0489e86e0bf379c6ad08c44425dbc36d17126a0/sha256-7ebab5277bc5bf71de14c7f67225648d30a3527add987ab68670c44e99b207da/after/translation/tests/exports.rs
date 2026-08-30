//! Every dynamic symbol the C `.so` exports must also be exported, under the
//! exact same name, by the Rust `.so`.

mod common;

use common::{c_lib_path, rust_lib_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined (`nm -D` type letter is not `U`/`w`) dynamic symbol names.
fn defined_dynamic_symbols(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        lib.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(t), Some(name)) if t.len() == 1 => Some((t.to_string(), name)),
                // "         <type> <name>" (no address)
                (Some(t), Some(name), None) if t.len() == 1 => Some((t.to_string(), name)),
                _ => None,
            }
        })
        .filter(|(t, _)| t != "U" && t != "w")
        .map(|(_, name)| name.split('@').next().unwrap_or(name).to_string())
        .collect()
}

/// Symbols that every shared object gets from the linker / runtime rather than
/// from the translated source. They are not part of the library's API, so the
/// comparison ignores them.
fn is_toolchain_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__bss_start__",
        "_bss_end__",
        "__end__",
        "_DYNAMIC",
        "_GLOBAL_OFFSET_TABLE_",
    ];
    EXACT.contains(&name)
        || name.starts_with("__gnu")
        || name.starts_with("_ITM_")
        || name.starts_with("__cxa")
        || name.starts_with("__gmon")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = c_lib_path();
    let r = rust_lib_path();

    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    // Sanity check: the API from driver.h/driver.c must actually be there.
    for expected in ["driver", "good", "bad", "printLine", "printIntLine"] {
        assert!(
            c_syms.contains(expected),
            "C .so does not export {expected}; symbols: {c_syms:?}"
        );
    }

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .filter(|s| !r_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}",
        r.display(),
        c.display(),
        missing
    );
}
