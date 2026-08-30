//! Every dynamic symbol the C .so exports must also be exported by the Rust
//! .so under the exact same name, and must be resolvable via `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Names defined and dynamically visible in `path`, excluding the toolchain /
/// runtime boilerplate that is not part of the translated API.
fn exported_names(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
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
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only code/data symbols in the text and data sections.
            if !matches!(kind, "T" | "t" | "D" | "B" | "R" | "W") {
                return None;
            }
            if name.starts_with('_') || name.contains('@') {
                return None; // _init, _fini, __bss_start, versioned refs, ...
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_names = exported_names(&root().join("c_src/build/libdriver.so"));

    let exe = std::env::current_exe().expect("test exe");
    let mut dir = exe.parent().expect("deps").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let rust_names = exported_names(&dir.join("libdriver.so"));

    assert!(
        !c_names.is_empty(),
        "no symbols parsed from the C .so; check the cmake build"
    );

    let missing: Vec<_> = c_names.difference(&rust_names).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {missing:?}\n  C   : {c_names:?}\n  Rust: {rust_names:?}"
    );

    // The five non-static functions in src/driver.c.
    for expected in ["driver", "good", "bad", "printLine", "printIntLine"] {
        assert!(
            c_names.contains(expected),
            "C .so unexpectedly lacks `{expected}`"
        );
        assert!(
            rust_names.contains(expected),
            "Rust .so lacks `{expected}`"
        );
    }
}

/// `nm` shows the symbol table; this confirms the symbols are actually usable
/// through `dlsym` with the expected ABI in both libraries.
#[test]
fn all_symbols_resolve_via_dlsym_in_both() {
    let l = common::libs();
    for name in ["driver", "good", "bad", "printLine", "printIntLine"] {
        let n = std::ffi::CString::new(name).unwrap();
        unsafe {
            let c: Result<libloading::Symbol<*const ()>, _> = l.c.get(n.to_bytes_with_nul());
            let r: Result<libloading::Symbol<*const ()>, _> = l.rust.get(n.to_bytes_with_nul());
            assert!(c.is_ok(), "dlsym `{name}` failed in C .so");
            assert!(r.is_ok(), "dlsym `{name}` failed in Rust .so");
        }
    }
}

/// The Rust `static` equivalents must stay private, matching the C `static`
/// helpers `goodG2B` / `goodB2G` which have no external linkage.
#[test]
fn static_helpers_are_not_exported_by_either() {
    let c_names = exported_names(&root().join("c_src/build/libdriver.so"));
    for hidden in ["goodG2B", "goodB2G"] {
        assert!(
            !c_names.contains(hidden),
            "assumption broken: C exports `{hidden}`"
        );
    }
}
