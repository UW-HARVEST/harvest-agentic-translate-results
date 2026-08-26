// Phase D — exported-symbol parity between the C and the Rust shared object.
//
// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
// exact same name, and the Rust `.so` must have no unresolvable imports.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` reduced to the set of global (`T`/`D`/`B`/`R`)
/// symbol names.  Weak linker artefacts (`w`) are ignored: they are toolchain
/// scaffolding (`__gmon_start__`, `_ITM_*`, `__cxa_finalize`), not library API.
fn exported(so: &Path) -> BTreeSet<String> {
    ensure_built(so);
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"  (weak/undefined lines have no address)
        if cols.len() == 3 {
            let ty = cols[1];
            if matches!(ty, "T" | "D" | "B" | "R" | "G" | "S") {
                set.insert(cols[2].split('@').next().unwrap().to_string());
            }
        }
    }
    set
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "no symbols found in {} — was the C library built?",
        c_so_path().display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // The whole documented surface, spelled out so a silently-dropped export is
    // caught even if the C object were rebuilt differently.
    for name in [
        "check_permissions",
        "compare_operations",
        "complexmode",
        "copy_and_sum",
        "create_result_string",
        "multiply_with_log",
        "safe_add",
    ] {
        assert!(c.contains(name), "C .so lost {name}");
        assert!(r.contains(name), "Rust .so does not export {name}");
    }
    assert_eq!(c.len(), 7, "unexpected C export set: {c:?}");
}

#[test]
fn rust_so_has_no_unresolved_imports() {
    // RTLD_NOW forces every relocation to be resolved at load time, so a
    // missing (non-libc) dependency symbol makes this fail loudly.
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    let p = rust_so_path();
    let lib = unsafe { Library::open(Some(&p), RTLD_NOW | RTLD_LOCAL) };
    let lib = lib.unwrap_or_else(|e| panic!("dlopen(RTLD_NOW) {}: {e}", p.display()));
    // Keep it alive until the end of the test.
    std::mem::forget(lib);
}

#[test]
fn every_exported_symbol_is_callable_through_dlsym() {
    // `both()` resolves all seven symbols in each object; a missing or
    // mis-typed export would panic here.
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    let (cv, _) = capture(|| unsafe { (c.check_permissions)(0o644, 0o600) });
    let (rv, _) = capture(|| unsafe { (r.check_permissions)(0o644, 0o600) });
    assert_eq!(cv, 1);
    assert_eq!(cv, rv);
}
