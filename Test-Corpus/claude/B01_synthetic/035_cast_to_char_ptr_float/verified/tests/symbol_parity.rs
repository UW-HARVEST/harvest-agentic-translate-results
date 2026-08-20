//! Phase D — symbol parity between the C and Rust artifacts.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// The C *executable* (what `c_src/CMakeLists.txt` actually builds) must not
/// export any non-libc dynamic symbol — and neither must the Rust one. This
/// pins down the claim in SYMBOLS.md §1 rather than taking it on trust.
#[test]
fn test_c_exe_dynamic_symbols_are_libc_only() {
    let c = exported_symbols(&c_exe());
    let r = exported_symbols(&rust_exe());
    assert!(
        c.is_empty(),
        "C executable unexpectedly exports dynamic symbols: {c:?}"
    );
    assert!(
        r.is_empty(),
        "Rust executable unexpectedly exports dynamic symbols: {r:?}"
    );
}

/// Undefined (imported) symbols of the C executable must all come from libc /
/// the toolchain, which is what makes the empty-export claim meaningful.
#[test]
fn test_c_exe_imports_are_all_libc() {
    let out = Command::new("nm").arg("-D").arg(c_exe()).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let known = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__isoc99_scanf",
        "__libc_start_main",
        "printf",
        "putchar",
        "puts",
        "scanf",
        "__stack_chk_fail",
        "fwrite",
        "putc",
    ];
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let Some(kind) = it.next() else { continue };
        // undefined entries have no address, so the first field is the kind
        if kind != "U" && kind != "w" {
            continue;
        }
        let Some(name) = it.next() else { continue };
        let base = name.split('@').next().unwrap();
        assert!(
            known.contains(&base),
            "unexpected non-libc import in the C executable: {name}"
        );
    }
}

/// The application symbols with external linkage in the C translation unit
/// (`driver`, `main`) must also be present with external linkage in the Rust
/// executable, and the `static` one (`print_hex`) must be local in both.
#[test]
fn test_global_text_symbols_match() {
    fn globals(path: &std::path::Path) -> BTreeSet<String> {
        let out = Command::new("nm").arg(path).output().unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (_a, kind, name) = (it.next()?, it.next()?, it.next()?);
                if kind == "T" && (name == "main" || name == "driver") {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
    let c = globals(&c_exe());
    let r = globals(&rust_exe());
    assert_eq!(
        c,
        BTreeSet::from(["driver".to_string(), "main".to_string()]),
        "unexpected C global text symbols"
    );
    assert_eq!(c, r, "global text symbols differ:\n  C: {c:?}\n  RUST: {r:?}");
}

/// `print_hex` is `static` in C, so it must not be a global symbol on either
/// side. A translation that exported it would be widening the API surface.
#[test]
fn test_print_hex_is_not_global() {
    for path in [c_exe(), rust_exe(), c_so(), rust_so()] {
        let out = Command::new("nm").arg("-D").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.lines().any(|l| l.split_whitespace().nth(2) == Some("print_hex")),
            "print_hex is a dynamic export of {path:?}, but it is `static` in C"
        );
    }
}

/// The shared-object form: every symbol the C `.so` exports must also be
/// exported by the Rust `cdylib`, with the exact same name. The diff must be
/// empty. `main` only appears when the `c_main` feature is on (see
/// SYMBOLS.md §3), so the assertion is scoped accordingly.
#[test]
fn test_c_so_exports_are_all_present_in_rust_so() {
    let c: BTreeSet<String> = exported_symbols(&c_so()).into_iter().collect();
    let r: BTreeSet<String> = exported_symbols(&rust_so()).into_iter().collect();

    assert!(
        c.contains("driver"),
        "C .so should export `driver`, got {c:?}"
    );
    assert!(
        r.contains("driver"),
        "Rust .so must export `driver`, got {r:?}"
    );

    let expected: BTreeSet<String> = if cfg!(feature = "c_main") {
        c.clone()
    } else {
        // Without the feature the binary crate owns `main`; everything else
        // must still match exactly.
        c.iter().filter(|s| *s != "main").cloned().collect()
    };

    let missing: Vec<_> = expected.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n  C:    {c:?}\n  RUST: {r:?}"
    );
}

/// The Rust `cdylib` must not have undefined non-libc symbols — i.e. it is a
/// self-contained translation, not something that links back to the C code.
#[test]
fn test_rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("ldd").arg("-r").arg(rust_so()).output().unwrap();
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !all.contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{all}"
    );

    // And it must not depend on the C object in any way.
    assert!(
        !all.contains("cdriver"),
        "Rust .so links against the C shared object:\n{all}"
    );
}
