//! Verifies that the Rust `.so` exports every dynamic symbol the C `.so`
//! exports, under the same names (step 8 of the verification plan).

mod common;

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
        .unwrap_or_else(|e| panic!("cannot read {}: {}", build.display(), e))
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .unwrap_or_else(|| panic!("no C .so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap();
    let p = profile.join("libsh_geti_lib.so");
    assert!(
        p.exists(),
        "{} does not exist — run `cargo build` (or ./run_tests.sh) first; \
         `cargo test` does not build a cdylib target",
        p.display()
    );
    p
}

fn dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {}", so.display(), e));
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            // "<addr> <type> <name>" or "        <type> <name>"
            cols.last().map(|s| s.to_string()).filter(|_| cols.len() >= 2)
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = find_c_so();
    let r = find_rust_so();
    let c_syms = dynamic_symbols(&c);
    let r_syms = dynamic_symbols(&r);

    assert!(
        !c_syms.is_empty(),
        "nm reported no symbols for {}",
        c.display()
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}",
        r.display(),
        c.display(),
        missing
    );
}

/// Belt and braces: the documented public API plus every `stbds_*` helper the C
/// translation unit exposes must be resolvable by name.
#[test]
fn expected_symbols_are_present_in_both() {
    let expected = [
        "sh_geti",
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
    ];
    let c_syms = dynamic_symbols(&find_c_so());
    let r_syms = dynamic_symbols(&find_rust_so());
    for e in expected {
        assert!(c_syms.contains(&e.to_string()), "C .so lacks {}", e);
        assert!(r_syms.contains(&e.to_string()), "Rust .so lacks {}", e);
    }
    // And they must all be loadable through dlsym.
    let _g = common::serial();
    let _ = common::apis();
}
