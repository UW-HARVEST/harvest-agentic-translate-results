//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`,
//! plus a smoke test proving the harness really resolves the *library's*
//! `wcscat` and not glibc's same-named 2-argument function.

mod common;

use common::*;
use std::process::Command;

fn dynamic_globals(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>" or "         <type> <name>"
            match it.next() {
                Some(name) => {
                    let _ = a;
                    let _ = b;
                    Some(name.to_string())
                }
                None => Some(b.to_string()),
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols the Rust `cdylib` runtime unavoidably exports; they are not part of
/// the library's API surface and have no counterpart in the C build.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_R")
        || s.starts_with("rust_")
        || s.starts_with("__rust")
        || s.starts_with("_ITM_")
        || s.starts_with("__cxa")
        || s.starts_with("_Unwind")
        || matches!(
            s,
            "_init"
                | "_fini"
                | "__bss_start"
                | "_edata"
                | "_end"
                | "__gmon_start__"
                | "_IO_stdin_used"
        )
}

#[test]
fn phase_a_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_syms = dynamic_globals(&l.c_path);
    let rs_syms = dynamic_globals(&l.rs_path);

    println!("C  .so: {}", l.c_path.display());
    println!("RS .so: {}", l.rs_path.display());
    println!("C  dynamic globals ({}): {:?}", c_syms.len(), c_syms);
    println!("RS dynamic globals ({}): {:?}", rs_syms.len(), rs_syms);

    // The C library's only API symbol.
    assert!(
        c_syms.iter().any(|s| s == "wcscat"),
        "sanity: the C .so must export wcscat, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_rust_runtime_symbol(s))
        .filter(|s| !rs_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         (Phase A rule: add the #[no_mangle] wrapper, or translate the missing C module.)"
    );
}

#[test]
fn phase_a_no_undefined_non_libc_symbols_in_rust() {
    let l = libs();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&l.rs_path)
        .output()
        .expect("nm -D --undefined-only");
    let text = String::from_utf8_lossy(&out.stdout);
    let undef: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .collect();
    println!("RS undefined ({}): {undef:?}", undef.len());

    // Everything undefined must be resolvable, which dlopen already proved by
    // succeeding with RTLD_LAZY + an explicit dlsym of `wcscat`. Assert here that
    // no *library API* symbol is left undefined.
    assert!(
        !undef.contains(&"wcscat"),
        "the Rust .so must DEFINE wcscat, not import it"
    );
}

#[test]
fn smoke_resolved_symbol_is_the_library_not_glibc() {
    // glibc's `wcscat(dst, src)` would dereference `src == (const wchar_t*)0`
    // here and crash; the library's 3-argument version returns 22 for numElem==0.
    let mut dst = vec![7i32, 8, 9];
    let src = vec![1i32, 0];
    let l = libs();
    let c_ret = unsafe { (l.c)(dst.as_mut_ptr(), 0, src.as_ptr()) };
    let rs_ret = unsafe { (l.rs)(dst.as_mut_ptr(), 0, src.as_ptr()) };
    assert_eq!(c_ret, 22, "resolved the wrong `wcscat` from the C .so?");
    assert_eq!(rs_ret, 22, "resolved the wrong `wcscat` from the Rust .so?");
    assert_eq!(dst, vec![7, 8, 9], "numElem==0 must not write to dst");
}

#[test]
fn smoke_basic_append_matches() {
    let out = check(&Case::new(
        "smoke_basic_append",
        vec![b'a' as i32, b'b' as i32, 0, GUARD, GUARD, GUARD, GUARD, GUARD],
        8,
        Src::Buf(vec![b'c' as i32, b'd' as i32, 0]),
    ));
    assert_eq!(out.ret, 0);
    assert_eq!(
        out.dst.unwrap()[..5],
        [b'a' as i32, b'b' as i32, b'c' as i32, b'd' as i32, 0]
    );
}

#[test]
fn platform_abi_assumptions_hold() {
    // The C header uses `wchar_t` from <stddef.h>; the Rust picks i32 on non-Windows.
    assert_eq!(
        std::mem::size_of::<WcharT>(),
        4,
        "wchar_t width assumption broken for this target"
    );
    assert!((WcharT::MIN) < 0, "wchar_t must be signed on this target");
    assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of::<u64>());
}
