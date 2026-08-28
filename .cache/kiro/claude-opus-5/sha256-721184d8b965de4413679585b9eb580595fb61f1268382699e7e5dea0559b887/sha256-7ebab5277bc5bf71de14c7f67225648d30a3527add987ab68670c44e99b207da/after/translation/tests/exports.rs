//! Symbol-export parity: every dynamic symbol the C `.so` exports must also be
//! exported by the Rust `.so` under the exact same name, and must be resolvable
//! via `dlsym` (which is what `libloading` uses).

mod common;

use common::*;
use std::path::Path;
use std::process::Command;

fn find_c_so() -> std::path::PathBuf {
    c_so_path()
}

fn find_rust_so() -> std::path::PathBuf {
    rust_so_path()
}

/// Defined, exported dynamic symbols, filtered down to the ones that come from
/// the translation unit rather than the toolchain's own runtime scaffolding.
fn dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {}", so.display(), e));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    // Symbols emitted by the linker / language runtime, not by the source.
    const IGNORED: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "rust_eh_personality",
        "__rust_no_alloc_shim_is_unstable_v2",
        "rust_begin_unwind",
    ];

    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|n| !IGNORED.contains(&n.as_str()))
        // Rust's std brings its own mangled/internal symbols along; those are
        // additions, and only C's symbols are required to be present.
        .filter(|n| !n.starts_with("_Z") && !n.starts_with("__rust") && !n.starts_with("_R"))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Every function declared or defined with external linkage in c_src/src/lib.c.
const EXPECTED_C_API: &[&str] = &[
    "multiply_with_static",
    "add_with_static",
    "xor_operation",
    "shift_with_static",
    "get_operation",
    "execute_operation",
    "compute_checksum",
    "init_state",
    "apply_operation",
    "checkshift",
];

fn c_so_exports_the_expected_api() {
    let syms = dynamic_symbols(&find_c_so());
    for name in EXPECTED_C_API {
        assert!(
            syms.iter().any(|s| s == name),
            "C .so is missing {name}; nm reported {syms:?}"
        );
    }
}

fn rust_so_exports_every_c_symbol() {
    let c_so = find_c_so();
    let rs_so = find_rust_so();
    let c_syms = dynamic_symbols(&c_so);
    let rs_syms = dynamic_symbols(&rs_so);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}",
        rs_so.display(),
        c_so.display(),
        missing
    );
    assert!(
        !c_syms.is_empty(),
        "nm found no symbols in the C .so - the comparison would be vacuous"
    );
}

/// `nm` shows the symbol table; this confirms the symbols are actually usable
/// through `dlsym`, i.e. exported with default visibility and global binding.
fn every_symbol_resolves_via_dlsym() {
    let libs = impls();
    for name in EXPECTED_C_API {
        for which in BOTH {
            let _: libloading::Symbol<*const ()> = libs.sym(which, name);
        }
    }
}

/// Binding/visibility parity, so a caller linking against either library sees
/// the same kind of symbol (`T` = global text) for each entry point.
fn symbol_kinds_match() {
    let c_so = find_c_so();
    let rs_so = find_rust_so();

    let kinds = |so: &Path| -> std::collections::HashMap<String, char> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(so)
            .output()
            .expect("nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let _addr = it.next()?;
                let kind = it.next()?.chars().next()?;
                let name = it.next()?.to_string();
                Some((name, kind))
            })
            .collect()
    };

    let ck = kinds(&c_so);
    let rk = kinds(&rs_so);
    for name in EXPECTED_C_API {
        let c = ck.get(*name).copied();
        let r = rk.get(*name).copied();
        assert_eq!(
            c,
            r,
            "symbol kind mismatch for {name}: C={c:?} Rust={r:?}"
        );
        assert_eq!(c, Some('T'), "{name} should be a global text symbol in C");
    }
}

fn main() {
    let mut r = Runner::new();
    r.case("c_so_exports_the_expected_api", c_so_exports_the_expected_api);
    r.case("rust_so_exports_every_c_symbol", rust_so_exports_every_c_symbol);
    r.case("every_symbol_resolves_via_dlsym", every_symbol_resolves_via_dlsym);
    r.case("symbol_kinds_match", symbol_kinds_match);
    r.finish();
}
