//! Every dynamic symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and must be loadable via `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn dynamic_defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (_addr, kind, name) = (parts.next()?, parts.next()?, parts.next()?);
            // Only global text/data definitions; skip Rust/toolchain-internal noise.
            matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_string())
        })
        .filter(|n| {
            // Symbols the platform/toolchain injects into any shared object.
            !n.starts_with('_')
                && !matches!(
                    n.as_str(),
                    "rust_eh_personality" | "rust_metadata" | "__bss_start" | "_edata" | "_end"
                )
        })
        .collect()
}

#[test]
fn rust_exports_superset_of_c() {
    let libs = common::libs();
    let c_path = libs.c_path();
    let rust_path = libs.rust_path();

    let c_syms = dynamic_defined_symbols(&c_path);
    let rust_syms = dynamic_defined_symbols(&rust_path);

    assert!(
        !c_syms.is_empty(),
        "no symbols parsed from C .so {}",
        c_path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\n  C:    {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}

/// The C header's public API, plus the non-static function in lib.c.
#[test]
fn documented_api_is_dlsym_able_in_both() {
    let libs = common::libs();
    for name in [&b"siphash\0"[..], &b"stbds_hash_bytes\0"[..]] {
        unsafe {
            libs.c
                .get::<*const ()>(name)
                .unwrap_or_else(|e| panic!("C dlsym {:?}: {e}", String::from_utf8_lossy(name)));
            libs.rust
                .get::<*const ()>(name)
                .unwrap_or_else(|e| panic!("Rust dlsym {:?}: {e}", String::from_utf8_lossy(name)));
        }
    }
}
