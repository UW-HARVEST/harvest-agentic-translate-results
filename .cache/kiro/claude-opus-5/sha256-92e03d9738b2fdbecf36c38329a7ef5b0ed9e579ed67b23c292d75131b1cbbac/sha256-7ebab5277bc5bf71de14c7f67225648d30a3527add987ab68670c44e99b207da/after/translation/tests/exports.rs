//! Export parity: every dynamic symbol the C shared library defines must also
//! be defined by the Rust cdylib under the exact same name, and must be
//! resolvable through `dlsym`.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::libs;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("libdriver.so")
}

/// Defined (`T`/`t`/`D`/`B`/...) dynamic symbols, excluding undefined imports
/// and toolchain-internal entries.
fn defined_dynamic_symbols(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm -D --defined-only");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_string)
        .filter(|n| !is_toolchain_symbol(n))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Symbols emitted by the linker / language runtime rather than by the
/// translation unit itself.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__")
        || name.starts_with("_ITM_")
        || name.starts_with("_Unwind_")
        || name.starts_with("rust_eh_personality")
        || name == "_edata"
        || name == "_end"
        || name == "_IO_stdin_used"
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_library_path());
    let rust_syms = defined_dynamic_symbols(&rust_library_path());

    assert!(
        !c_syms.is_empty(),
        "expected the C library to export something"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust cdylib is missing C exports {missing:?}\n  C:    {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let libs = libs();
    for name in defined_dynamic_symbols(&c_library_path()) {
        let lookup: Result<libloading::Symbol<'_, *const ()>, _> =
            unsafe { libs.rust.get(name.as_bytes()) };
        assert!(
            lookup.is_ok(),
            "symbol {name:?} exists in the C library but is not dlsym-resolvable in the Rust cdylib"
        );
    }
}

#[test]
fn expected_public_api_is_present() {
    // The header declares `driver`; `driver.c` additionally gives external
    // linkage to printLine, printHexCharLine, bad and good. goodG2B/goodB2G are
    // `static` in C and must therefore stay unexported in Rust as well.
    let c_syms = defined_dynamic_symbols(&c_library_path());
    for expected in ["driver", "good", "bad", "printLine", "printHexCharLine"] {
        assert!(
            c_syms.contains(&expected.to_string()),
            "C library unexpectedly lacks {expected}"
        );
    }

    let rust_syms = defined_dynamic_symbols(&rust_library_path());
    for internal in ["goodG2B", "goodB2G", "good_g2b", "good_b2g"] {
        assert!(
            !c_syms.contains(&internal.to_string()),
            "static C function {internal} should not be exported"
        );
        assert!(
            !rust_syms.contains(&internal.to_string()),
            "internal Rust function {internal} should not be exported"
        );
    }
}
