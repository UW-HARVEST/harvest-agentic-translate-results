//! Phase D — symbol parity between the C and Rust artifacts (`SYMBOLS.md`).
//!
//! These tests never redirect fd 1, so they are safe to run in parallel; the
//! `run()` capture lives in `tests/ffi_capture.rs`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Symbols the Rust *runtime* defines that have no counterpart in the C
/// translation unit: they are the analogue of the libc symbols the C object
/// imports rather than defines, not translated code.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_RNv")
        || s.starts_with("__rust")
        || s.starts_with("rust_")
        || s == "rust_eh_personality"
}

/// CRT / linker artifacts that `gcc` or `rustc` contribute, in neither case from
/// `main.c`.
fn is_crt_symbol(s: &str) -> bool {
    matches!(
        s,
        "_init"
            | "_fini"
            | "_start"
            | "_dl_relocate_static_pie"
            | "__libc_csu_init"
            | "__libc_csu_fini"
            | "_IO_stdin_used"
    ) || s.starts_with("_ITM_")
        || s.starts_with("__gmon_start__")
}

fn nm(args: &[&str], path: &Path) -> String {
    let out = std::process::Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Defined dynamic symbols (`nm -D --defined-only`), minus runtime/CRT noise.
fn defined_dyn_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .filter(|s| !is_rust_runtime_symbol(s) && !is_crt_symbol(s))
        .collect()
}

/// Global text symbols (`nm` class `T`), minus runtime/CRT noise.
fn global_text_symbols(obj: &Path) -> BTreeSet<String> {
    nm(&[], obj)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let name = it.next().or(Some(b))?;
            // "<addr> T <name>"
            if b == "T" {
                let _ = a;
                Some(name.to_string())
            } else {
                None
            }
        })
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .filter(|s| !is_rust_runtime_symbol(s) && !is_crt_symbol(s))
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined_dyn_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "-u"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .collect()
}

// ---------------------------------------------------------------------------

#[test]
fn symbol_parity_shared_objects() {
    let c = defined_dyn_symbols(c_so());
    let r = defined_dyn_symbols(rust_so());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );

    // The C translation unit's external linkage is exactly {main, run}; assert
    // both are really there so the test cannot pass on two empty sets.
    for want in ["main", "run"] {
        assert!(c.contains(want), "C .so should export {want}, got {c:?}");
        assert!(r.contains(want), "Rust .so should export {want}, got {r:?}");
    }
    assert_eq!(c.len(), 2, "unexpected extra C exports: {c:?}");
}

#[test]
fn symbol_parity_executables() {
    let c = global_text_symbols(c_exe());
    let r = global_text_symbols(&rust_exe());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "global text symbols in the C executable but not the Rust executable: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );
    for want in ["main", "run"] {
        assert!(c.contains(want), "C exe should define {want}, got {c:?}");
        assert!(r.contains(want), "Rust exe should define {want}, got {r:?}");
    }
    assert_eq!(c.len(), 2, "unexpected extra C exe symbols: {c:?}");
}

#[test]
fn shared_objects_have_no_unresolved_symbols() {
    let (c_so, r_so) = (c_so(), rust_so());

    // `RTLD_NOW` resolves *every* relocation at load time, so a successful
    // `dlopen` is proof that nothing the object references is missing — a much
    // better check than an allow-list of libc names, which drifts with the libc
    // version (`mmap64`, `statx`, `realpath`, ... all appear or not by version).
    const RTLD_NOW: std::os::raw::c_int = 2;
    const RTLD_LOCAL: std::os::raw::c_int = 0;
    for so in [c_so, r_so] {
        let lib = unsafe {
            libloading::os::unix::Library::open(Some(so), RTLD_NOW | RTLD_LOCAL)
        };
        let lib = lib.unwrap_or_else(|e| {
            panic!("dlopen(RTLD_NOW) of {} failed — unresolved symbols: {e}", so.display())
        });
        drop(lib);
    }

    // Belt and braces: no *Rust*-mangled name may be left undefined, which is
    // what a partially translated crate would look like.
    for so in [c_so, r_so] {
        let dangling: Vec<String> = undefined_dyn_symbols(so)
            .into_iter()
            .filter(|s| s.starts_with("_ZN") || s.starts_with("_RNv"))
            .collect();
        assert!(
            dangling.is_empty(),
            "{} has undefined Rust symbols: {dangling:?}",
            so.display()
        );
    }
}

#[test]
fn cargo_built_cdylib_exports_the_same_symbols() {
    // Check the artifact cargo itself ships, not just the rustc-built copy.
    let base = rust_exe().parent().unwrap().to_path_buf();
    let candidates = [
        base.join("libdriver_ffi.so"),
        manifest_dir().join("target/release/libdriver_ffi.so"),
        manifest_dir().join("target/debug/libdriver_ffi.so"),
    ];
    let mut checked = 0;
    for cand in candidates.iter() {
        if cand.exists() {
            let syms = defined_dyn_symbols(cand);
            for want in ["main", "run"] {
                assert!(
                    syms.contains(want),
                    "{} does not export {want}: {syms:?}",
                    cand.display()
                );
            }
            checked += 1;
        }
    }
    eprintln!("checked {checked} cargo-built cdylib artifact(s)");
}

#[test]
fn no_stubbed_exports_in_the_translation() {
    // A stub that lies about behaviour is worse than a missing symbol, so make
    // sure none of the translated sources contain one.
    for rel in ["src/lib.rs", "src/main.rs", "src/house.rs", "src/parse.rs", "ffi/src/lib.rs"] {
        let path = manifest_dir().join(rel);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for needle in ["unimplemented!", "todo!", "unreachable!", "not implemented"] {
            assert!(
                !text.contains(needle),
                "{rel} contains {needle:?} — the translation must be real code"
            );
        }
    }
}
