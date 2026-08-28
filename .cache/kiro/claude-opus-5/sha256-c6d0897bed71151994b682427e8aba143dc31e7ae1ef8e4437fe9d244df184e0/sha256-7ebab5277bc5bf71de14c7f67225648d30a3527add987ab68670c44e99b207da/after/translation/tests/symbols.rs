//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name - including the ones that only exist
//! because of preprocessor expansion.

mod common;

use common::*;
use std::process::Command;

fn dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Skip the linker-synthesised bookkeeping symbols.
            if matches!(name, "_init" | "_fini" | "__bss_start" | "_edata" | "_end") {
                return None;
            }
            let _ = kind;
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let _g = guard();
    // Force both libraries to be located/built.
    let _ = libs();

    let c = dynamic_symbols(&c_so_path_pub());
    let r = dynamic_symbols(&rust_so_path_pub());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c:?}\nRust: {r:?}"
    );

    // The C library's whole public surface, for the record.
    for expected in [
        "arr_push",
        "strkey",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ] {
        assert!(
            c.contains(&expected.to_string()),
            "test is stale: C .so no longer exports {expected}"
        );
        assert!(
            r.contains(&expected.to_string()),
            "Rust .so does not export {expected}"
        );
    }
}
