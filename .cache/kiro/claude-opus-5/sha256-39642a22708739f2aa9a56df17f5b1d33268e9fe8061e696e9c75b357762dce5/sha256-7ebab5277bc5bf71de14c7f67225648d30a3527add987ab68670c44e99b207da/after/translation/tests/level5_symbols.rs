//! Step 8: every dynamic symbol the C shared object defines must also be
//! defined by the Rust shared object, under the exact same name.

mod harness;

use harness::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;

fn dynamic_defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>"
        let mut it = line.split_whitespace();
        let _addr = it.next();
        let kind = match it.next() {
            Some(k) => k,
            None => continue,
        };
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        if kind.len() != 1 {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Symbols that come from the C runtime / build glue rather than from the
/// translated source itself.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_IO_stdin_used"
    ) || name.starts_with("__gnu")
        || name.starts_with("_ITM_")
        || name.starts_with("__cxa")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dynamic_defined_symbols(&c_so_path());
    let r = dynamic_defined_symbols(&rust_so_path());

    let wanted: BTreeSet<&String> = c.iter().filter(|s| !is_toolchain_symbol(s)).collect();
    assert!(
        !wanted.is_empty(),
        "no symbols found in the C .so — is it built?"
    );

    let missing: Vec<&&String> = wanted.iter().filter(|s| !r.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}

/// The full stb_ds public surface must be present in both.
#[test]
fn expected_api_surface_present() {
    let expected = [
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_string",
        "stbds_hash_bytes",
        "stbds_hmfree_func",
        "stbds_hmget_key_ts",
        "stbds_hmget_key",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_shmode_func",
        "stbds_hmdel_key",
        "stbds_stralloc",
        "stbds_strreset",
        "strkey",
        "intput",
    ];
    let c = dynamic_defined_symbols(&c_so_path());
    let r = dynamic_defined_symbols(&rust_so_path());
    for name in expected {
        assert!(c.contains(name), "C .so is missing {name}");
        assert!(r.contains(name), "Rust .so is missing {name}");
    }
}
