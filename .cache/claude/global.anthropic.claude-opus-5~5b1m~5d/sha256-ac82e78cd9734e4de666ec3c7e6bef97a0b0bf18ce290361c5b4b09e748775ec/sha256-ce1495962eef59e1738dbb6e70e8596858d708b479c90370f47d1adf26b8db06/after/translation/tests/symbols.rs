//! Phase D — symbol-parity checks between the C `.so` and the Rust `.so`.
//!
//! Enforces `SYMBOLS.md`: every symbol the C shared object exports must also be
//! exported, under the exact same name, by the Rust shared object; and the Rust
//! object must not have undefined references outside the C runtime.

mod common;

use common::{c_so_path, pair, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("`nm` must be available (binutils)");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Dynamic symbols *defined* (exported) by an object, as `nm -D --defined-only`
/// reports them. The `@VERSION` suffix is kept so aliasing differences show up.
fn exported(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Dynamic symbols *undefined* (imported) by an object.
fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], path)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Toolchain / CRT glue that is emitted by the compiler rather than by the
/// library's own source, and therefore is not part of the API contract.
fn is_toolchain_glue(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    base.starts_with("_ITM_")
        || base.starts_with("__gmon_start__")
        || base.starts_with("_init")
        || base.starts_with("_fini")
        || base.starts_with("__bss_start")
        || base.starts_with("_edata")
        || base.starts_with("_end")
        || base.starts_with("__cxa_")
        || base.starts_with("_Jv_")
        || base.starts_with("__deregister_frame_info")
        || base.starts_with("__register_frame_info")
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());

    let c_exp = exported(&c);
    let r_exp = exported(&r);

    let c_api: BTreeSet<&String> = c_exp.iter().filter(|s| !is_toolchain_glue(s)).collect();
    assert!(
        c_api.contains(&"synth_pair".to_string()),
        "sanity: the C .so must export `synth_pair`, exported = {c_exp:?}"
    );

    let missing: Vec<&String> = c_api
        .iter()
        .copied()
        .filter(|s| !r_exp.contains(*s))
        .collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_exp:?}\n\
         Rust exports: {r_exp:?}",
        missing.len()
    );
}

#[test]
fn no_unexpected_undefined_symbols() {
    let r = rust_so_path();
    let undef = undefined(&r);
    // Everything the Rust cdylib imports must come from the C runtime / libc,
    // i.e. carry a glibc version tag or be recognised toolchain glue.
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| !is_toolchain_glue(s) && !s.contains("@GLIBC") && !s.contains("@GCC"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn static_c_helper_is_not_exported() {
    // `mp3d_scale_pcm` is `static` in the C, so it must not appear in either
    // dynamic symbol table.
    for p in [c_so_path(), rust_so_path()] {
        let exp = exported(&p);
        assert!(
            !exp.iter().any(|s| s.contains("mp3d_scale_pcm")),
            "{} unexpectedly exports mp3d_scale_pcm: {exp:?}",
            p.display()
        );
    }
}

#[test]
fn both_libraries_resolve_the_symbol_through_dlsym() {
    // Redundant with the above but proves the symbol is *callable*, not merely
    // listed, from both objects.
    let p = pair();
    let z = vec![0.0f32; common::Z_MIN_LEN];
    let mut a = vec![0i16; 64];
    let mut b = vec![0i16; 64];
    unsafe {
        (p.c.synth_pair)(a.as_mut_ptr(), 2, z.as_ptr());
        (p.rust.synth_pair)(b.as_mut_ptr(), 2, z.as_ptr());
    }
    assert_eq!(a, b);
}

/// Regenerates the symbol diff that `SYMBOLS.md` documents, and asserts it is
/// empty. Printed so the artifact can be kept honest by eye as well.
#[test]
fn symbol_diff_is_empty() {
    let c_exp = exported(&c_so_path());
    let r_exp = exported(&rust_so_path());
    let diff: Vec<&String> = c_exp
        .iter()
        .filter(|s| !is_toolchain_glue(s) && !r_exp.contains(*s))
        .collect();
    eprintln!("C API symbols  : {:?}", c_exp.iter().filter(|s| !is_toolchain_glue(s)).collect::<Vec<_>>());
    eprintln!("Rust exports   : {r_exp:?}");
    eprintln!("Missing in Rust: {diff:?}");
    assert!(diff.is_empty(), "symbol diff must be empty, got {diff:?}");
}
