//! Export parity: every dynamic symbol the C `libdriver.so` defines must also be
//! defined by the Rust cdylib, under the exact same name.
//!
//! `driver.h` has no namespacing macros, so the linker names are the
//! source-level names verbatim; this test still reads them out of the built
//! object rather than hard-coding a list, so a symbol introduced by a macro (or
//! by a change to the C source) would be caught.

use std::path::{Path, PathBuf};
use std::process::Command;

fn c_lib() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_lib() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    for cand in [
        deps.join("libdriver.so"),
        deps.parent().unwrap().join("libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib not found next to {}", exe.display());
}

/// Names of the dynamic symbols *defined* (not imported) by an ELF shared
/// object, via `nm -D --defined-only`.
fn defined_dynamic_symbols(lib: &Path) -> Option<Vec<String>> {
    let out = match Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(lib)
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => panic!(
            "nm failed on {}: {}",
            lib.display(),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => {
            eprintln!("SKIP: cannot run `nm`: {e}");
            return None;
        }
    };

    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>"; the address is blank for some symbol
            // types, so take the trailing field and the type before it.
            let mut it = line.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            // Skip the ELF version/ABI bookkeeping symbols, which are not part
            // of any API and are emitted by the linker, not the source.
            if name.starts_with("_ITM_") || name.starts_with("__gmon") {
                return None;
            }
            let _ = kind;
            Some(name.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    Some(names)
}

/// C-defined symbols must be a subset of Rust-defined symbols.
#[test]
fn rust_exports_every_c_symbol() {
    let c_path = c_lib();
    assert!(
        c_path.exists(),
        "build the C library first: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    let Some(c_syms) = defined_dynamic_symbols(&c_path) else {
        return;
    };
    let rust_path = rust_lib();
    let Some(rust_syms) = defined_dynamic_symbols(&rust_path) else {
        return;
    };

    println!("C   ({}): {c_syms:?}", c_path.display());
    println!("Rust ({}): {rust_syms:?}", rust_path.display());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing {} symbol(s) exported by the C library: {missing:?}",
        missing.len()
    );

    // The four functions in driver.c are the whole surface; if the C object ever
    // grows or loses one, this reminds us to re-check the translation.
    let expected = ["bad", "driver", "good", "printLine"];
    assert_eq!(
        c_syms, expected,
        "the C library's exported surface changed; re-check the translation"
    );
}

/// The exported functions must be usable as ordinary global text symbols in both
/// objects (type `T`), not weak or data symbols.
#[test]
fn exported_functions_are_global_text_symbols() {
    for lib in [c_lib(), rust_lib()] {
        if !lib.exists() {
            continue;
        }
        let out = match Command::new("nm").args(["-D", "--defined-only"]).arg(&lib).output() {
            Ok(o) if o.status.success() => o,
            Ok(_) => continue,
            Err(e) => {
                eprintln!("SKIP: cannot run `nm`: {e}");
                return;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout);
        for func in ["printLine", "bad", "good", "driver"] {
            let line = text
                .lines()
                .find(|l| l.split_whitespace().next_back() == Some(func))
                .unwrap_or_else(|| panic!("{} does not export {func}", lib.display()));
            let kind = line.split_whitespace().nth_back(1).unwrap_or("?");
            assert_eq!(
                kind,
                "T",
                "{}: {func} has symbol type `{kind}`, expected global text `T` (line: {line:?})",
                lib.display()
            );
        }
    }
}
