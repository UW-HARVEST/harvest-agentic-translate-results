//! Phase D — symbol parity between the C and Rust shared libraries.

mod harness;

use harness::*;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("C_DRIVER_SO").map(PathBuf::from).unwrap_or_else(|_| {
        manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so")
    })
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust cdylib found");
}

/// `nm -D --defined-only <so>` -> sorted list of exported symbol names.
fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        // Filter out the linker/CRT bookkeeping symbols that are an artefact of
        // the toolchain rather than of the translated source.
        .filter(|s| {
            !matches!(
                s.as_str(),
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__gnu_lto_slim"
                    | "_ITM_registerTMCloneTable"
                    | "_ITM_deregisterTMCloneTable"
            )
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`
/// under the exact same name. The diff must be empty.
fn symbol_parity_c_subset_of_rust(_libs: &Libs) {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    assert!(!c.is_empty(), "C .so exported no symbols; is it built?");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );
    // The C library's entire public surface is `driver`.
    assert_eq!(c, vec!["driver".to_string()], "unexpected C symbol surface");
}

/// The `static` C helper `print_hex` must NOT be exported by either library —
/// exporting it would be a surface mismatch in the other direction.
fn static_helper_is_not_exported(_libs: &Libs) {
    for so in [c_so(), rust_so()] {
        let syms = exported_symbols(&so);
        assert!(
            !syms.iter().any(|s| s == "print_hex"),
            "{so:?} must not export the file-local helper `print_hex`"
        );
    }
}

/// The Rust `.so` must have no undefined symbol that is not provided by libc /
/// the platform runtime (i.e. no missing *project* symbol).
fn rust_has_no_missing_project_symbols(_libs: &Libs) {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // Everything a Rust cdylib legitimately imports: versioned glibc
            // symbols, the unwinder, ITM/CRT weak hooks, and bare libc names.
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && !s.starts_with("__")
                && !s.starts_with("_Unwind_")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined non-libc symbols: {suspicious:?}"
    );
}

/// Sanity: both `.so` files resolve `driver` through `dlsym` and it is callable.
fn both_libraries_expose_callable_driver(libs: &Libs) {    assert_same(libs, 0x0badc0deu32 as i32, "phase D dlsym smoke");
}

/// Aggregate entry point — see `harness::run_rows` for why every row runs
/// inside a single `#[test]` (process-wide stdout redirection must be serial).
#[test]
fn all_rows() {
    run_rows(&[
        ("symbol_parity_c_subset_of_rust", symbol_parity_c_subset_of_rust),
        ("static_helper_is_not_exported", static_helper_is_not_exported),
        ("rust_has_no_missing_project_symbols", rust_has_no_missing_project_symbols),
        ("both_libraries_expose_callable_driver", both_libraries_expose_callable_driver),
    ]);
}
