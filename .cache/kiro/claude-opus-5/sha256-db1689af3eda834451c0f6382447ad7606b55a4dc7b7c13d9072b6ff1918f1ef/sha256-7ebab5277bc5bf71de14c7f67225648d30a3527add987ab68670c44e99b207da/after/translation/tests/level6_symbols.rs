//! Every symbol the C `.so` exports must also be exported, under the exact
//! same name, by the Rust `.so` — including anything produced by preprocessor
//! macros. Checked with `nm -D --defined-only` on both objects.
mod harness;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("STR_DUPS_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "STR_DUPS_RUST_SO={} does not exist", p.display());
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut cands: Vec<PathBuf> = ["release", "debug"]
        .iter()
        .map(|p| base.join(p).join("libstr_dups_lib.so"))
        .filter(|p| p.exists())
        .collect();
    assert!(
        !cands.is_empty(),
        "the Rust cdylib has not been built yet; run `cargo build --release`"
    );
    cands.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    cands.pop().unwrap()
}

/// Exported (dynamic, defined) symbol names, restricted to code/data symbols.
fn exported_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
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
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(_addr), Some(kind), Some(name)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        // T/t: text, D/d/B/b: data/bss, R/r: read-only data, W: weak
        if matches!(
            kind,
            "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "V" | "v"
        ) {
            set.insert(name.to_string());
        }
    }
    set
}

/// Symbols every ELF shared object gets from the toolchain rather than from
/// the translated source; they are not part of the API surface.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__bss_start__"
            | "_bss_end__"
            | "__bss_end__"
            | "__end__"
            | "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__gmon_start__"
            | "__register_frame_info"
            | "__deregister_frame_info"
    ) || name.starts_with("_rust_")
        || name.starts_with("rust_")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_so = find_c_so();
    let rust_so = find_rust_so();
    let c_syms = exported_symbols(&c_so);
    let r_syms = exported_symbols(&rust_so);

    let c_api: BTreeSet<&String> = c_syms.iter().filter(|s| !is_toolchain_symbol(s)).collect();
    assert!(
        !c_api.is_empty(),
        "no symbols read from {} — is nm working?",
        c_so.display()
    );

    let missing: Vec<&&String> = c_api.iter().filter(|s| !r_syms.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n  C   : {}\n  Rust: {}",
        missing.len(),
        missing,
        c_so.display(),
        rust_so.display()
    );
}

/// Sanity check: the documented public API and every non-static helper in
/// `lib.c` must be present in both objects.
#[test]
fn known_api_symbols_present_in_both() {
    let c_syms = exported_symbols(&find_c_so());
    let r_syms = exported_symbols(&find_rust_so());
    for name in [
        "str_dups",
        "strkey",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_stralloc",
        "stbds_strreset",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
    ] {
        assert!(c_syms.contains(name), "C .so does not export {name}");
        assert!(r_syms.contains(name), "Rust .so does not export {name}");
    }
}

/// `buffer` in `lib.c` is `static`, so it must stay private in both builds.
#[test]
fn static_c_symbols_are_not_exported() {
    let c_syms = exported_symbols(&find_c_so());
    for name in ["buffer", "stbds_hash_seed"] {
        assert!(
            !c_syms.contains(name),
            "unexpected: C .so exports the static {name}"
        );
    }
}
