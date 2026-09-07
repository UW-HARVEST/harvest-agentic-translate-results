//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Uses `nm -D --defined-only` on both objects and asserts the exported symbol
//! set of the C library is fully contained in the Rust library's, and that each
//! symbol is actually resolvable through `dlsym` (i.e. callable, not just named).

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // exported code/data (T = text, D/B/R = data), skip ifunc/weak noise
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn c_so() -> PathBuf {
    find_c_so()
}

fn rust_so() -> PathBuf {
    find_rust_so()
}

#[test]
fn symbol_diff_is_empty() {
    let c = exported(&c_so());
    let r = exported(&rust_so());
    assert!(!c.is_empty(), "nm found no exported symbols in the C .so");
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let lib = rust_lib();
    for name in exported(&c_so()) {
        let sym: Result<libloading::Symbol<'_, *const ()>, _> =
            unsafe { lib.get(format!("{name}\0").as_bytes()) };
        assert!(sym.is_ok(), "Rust .so cannot dlsym `{name}`");
    }
}

// The typed accessors already prove resolvability for the six public functions;
// this keeps a direct handle for the generic loop above.
fn rust_lib() -> &'static libloading::Library {
    // `Impl` keeps its `Library` private; re-open the object instead (dlopen is
    // refcounted, so this is the same mapping).
    static L: std::sync::OnceLock<libloading::Library> = std::sync::OnceLock::new();
    L.get_or_init(|| unsafe { libloading::Library::new(rust_so()).expect("reopen rust .so") })
}

#[test]
fn all_six_exports_are_callable_through_both_sos() {
    let l = libs();
    let _g = lock();
    for imp in [&l.c, &l.rust] {
        let _ = imp.create_state();
        let _ = imp.destroy_state();
        let _ = imp.process_buffer();
        let _ = imp.update_flags();
        let _ = imp.confuse_types();
        let _ = imp.confusion();
    }
    // Smoke-call the whole surface once through each .so.
    let (rets, _out) = capture_stdout(|| unsafe {
        let mut v = vec![];
        for imp in [&l.c, &l.rust] {
            let p = imp.create_state()(7, 128);
            imp.update_flags()(p, 42);
            v.push(imp.process_buffer()(p, b'7' as std::ffi::c_char));
            v.push(imp.confuse_types()(p, 2));
            imp.destroy_state()(p);
            v.push(imp.confusion()(1, 2, 3, 4));
        }
        v
    });
    assert_eq!(&rets[0..3], &rets[3..6], "smoke results differ: {rets:?}");
}
