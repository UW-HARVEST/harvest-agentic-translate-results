//! Exported-symbol parity: every dynamic symbol the C `.so` defines must also
//! be defined by the Rust `cdylib`, under the exact same name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    v.sort();
    v.remove(0)
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("PARITY_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libupdate_md5_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("libupdate_md5_lib.so not built");
}

/// Dynamic, *defined* symbols, excluding the toolchain/runtime boilerplate that
/// neither implementation controls.
fn dynamic_defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only real code/data definitions.
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r") {
                return None;
            }
            Some(name.to_string())
        })
        .filter(|n| !is_toolchain_symbol(n))
        .collect()
}

fn is_toolchain_symbol(n: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "_init",
        "_fini",
        "__",
        "_ITM_",
        "_edata",
        "_end",
        "__bss_start",
        "rust_",
        "__rust",
        "_ZN",
        "_R",
        "call_weak_fn",
        "deregister_tm_clones",
        "register_tm_clones",
        "frame_dummy",
        "completed",
    ];
    PREFIXES.iter().any(|p| n.starts_with(p))
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dynamic_defined_symbols(&c_so());
    let r = dynamic_defined_symbols(&rust_so());

    assert!(
        !c.is_empty(),
        "sanity: expected the C .so to export something"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C   : {c:?}\n  Rust: {r:?}"
    );

    // The three documented entry points must be present on both sides.
    for want in ["tflac_pack_u64le", "tflac_md5_addsample", "update_md5"] {
        assert!(c.contains(want), "C .so lacks {want}");
        assert!(r.contains(want), "Rust .so lacks {want}");
    }
}
