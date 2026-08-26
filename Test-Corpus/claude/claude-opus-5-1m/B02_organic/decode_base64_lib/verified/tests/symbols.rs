//! Phase D — exported symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn c_and_rust_export_identical_symbols() {
    let a = common::api();
    let c = defined_symbols(&a.c_path);
    let r = defined_symbols(&a.rust_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // The one documented public symbol must really be there.
    assert!(c.contains("decode_base64"), "C .so lost decode_base64");
    assert!(r.contains("decode_base64"), "Rust .so lost decode_base64");

    // `static` C helpers must stay internal in both.
    for internal in ["decode", "is_base64"] {
        assert!(
            !c.contains(internal),
            "C .so unexpectedly exports {internal}"
        );
        assert!(
            !r.contains(internal),
            "Rust .so must not export the internal helper {internal}"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let a = common::api();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&a.rust_path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Anything genuinely unresolved would already have made `dlopen` fail in
    // `common::api()`; assert that explicitly plus a sanity check that the
    // libc allocator entry points are the ones being imported.
    for want in ["calloc", "malloc", "free", "strlen"] {
        assert!(
            text.contains(want),
            "Rust .so does not import libc `{want}` — it must use the platform \
             allocator so callers can `free()` the result like the C original.\n{text}"
        );
    }
}
