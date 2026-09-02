//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. The diff must be empty.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> String {
    let out = Command::new("nm")
        .arg("-D")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of dynamically exported (defined) symbols.
fn exported(path: &Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"])
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

/// Names of undefined (imported) symbols, weak ones included.
fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-u"])
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn symbol_diff_is_empty() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so — the check would be vacuous"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\nC exports:    {:?}\nRust exports: {:?}",
        missing.len(),
        missing,
        c,
        r
    );

    // The C translation unit has exactly two external-linkage definitions;
    // everything else in driver.c is `static`. Assert that explicitly so this
    // test notices if the C surface ever grows.
    let expected: BTreeSet<String> = ["driver", "run"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C export set changed; SYMBOLS.md and the tests must be updated"
    );
    assert_eq!(
        r, expected,
        "the Rust .so exports a different set than the C .so"
    );
}

/// Every symbol the Rust `.so` imports must be resolvable, i.e. there is no
/// missing/undefined non-libc symbol left behind by an untranslated module.
#[test]
fn no_unresolved_non_libc_symbols() {
    let out = Command::new("ldd")
        .arg(rust_so_path())
        .output()
        .expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "the Rust .so has unresolved dependencies:\n{text}"
    );

    // Cross-check: any symbol the C .so imports that is NOT supplied by the
    // platform C runtime would indicate a helper the translation dropped. All
    // of the C .so's imports are glibc/compiler-runtime symbols, so this set is
    // expected to be empty — the assertion exists to catch a future C module
    // that pulls in a non-libc helper.
    //
    // Deliberately NOT asserted: that the Rust .so imports the *same* libc
    // symbols. gcc lowers `printf("literal\n")` to `puts("literal")`, so the C
    // .so imports `puts` while the Rust .so calls `printf` directly. Both emit
    // identical bytes, which is what the differential tests in configs.rs /
    // errors.rs actually verify. Requiring import parity here would be
    // asserting on a compiler optimisation, not on behaviour.
    let c_undef = undefined(&c_so_path());
    let r_undef = undefined(&rust_so_path());
    let r_def = exported(&rust_so_path());

    let non_runtime: Vec<&String> = c_undef
        .iter()
        .filter(|s| !is_platform_runtime_symbol(s))
        .collect();
    for s in &non_runtime {
        assert!(
            r_undef.contains(*s) || r_def.contains(*s),
            "the C .so imports the non-libc symbol `{s}` but the Rust .so neither imports nor defines it"
        );
    }

    // Sanity: the C .so's imports are exactly the libc/runtime set we expect,
    // so the filter above is not silently hiding something.
    assert!(
        non_runtime.is_empty(),
        "unexpected non-runtime imports in the C .so: {non_runtime:?} — SYMBOLS.md must be updated"
    );
}

/// True for glibc / libgcc / compiler-runtime imports, i.e. symbols that are
/// resolved by the platform rather than by the translated library.
fn is_platform_runtime_symbol(s: &str) -> bool {
    s.contains("@GLIBC")
        || s.contains("@GCC")
        || s.starts_with("_ITM_")
        || s.starts_with("__gmon_start__")
        || s.starts_with("_Unwind_")
        || s.starts_with("__cxa_")
}

/// Both `.so` files must actually be loadable and both symbols dlsym-able —
/// `nm` agreement alone does not prove the dynamic symbols are usable.
#[test]
fn both_symbols_are_callable_through_dlsym() {
    let p = pair();
    let (c, r) = p.run_step(0);
    assert!(is_four_house_lines(&c));
    same("symbol parity: run", &c, &r);
    let (c, r) = p.driver_step_raw(b"1");
    assert!(!c.is_empty());
    same("symbol parity: driver", &c, &r);
}
