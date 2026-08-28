//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `cdylib` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {} ({e})", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no *.so in {}", build.display());
    found.remove(0)
}

fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let path = exe
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("libcircle_collide_lib.so");
    let so_mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| {
            panic!(
                "{} unusable ({e}) — `cargo test` does not build the cdylib; \
                 run `cargo build` first (see ./verify.sh)",
                path.display()
            )
        });
    let src = manifest_dir().join("src/lib.rs");
    let src_mtime = std::fs::metadata(&src).and_then(|m| m.modified()).unwrap();
    assert!(
        so_mtime >= src_mtime,
        "{} is stale relative to {} — run `cargo build` first (see ./verify.sh)",
        path.display(),
        src.display()
    );
    path
}

/// Names of defined dynamic symbols, minus the linker/loader boilerplate that
/// is not part of the library's API surface.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep code/data symbols; drop absolute/undefined bookkeeping.
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "R" | "r") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|n| {
            // Toolchain-generated entries present in every ELF shared object.
            !matches!(
                n.as_str(),
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__dso_handle"
                    | "_DYNAMIC"
                    | "_GLOBAL_OFFSET_TABLE_"
                    | "__TMC_END__"
                    | "__gmon_start__"
                    | "_ITM_deregisterTMCloneTable"
                    | "_ITM_registerTMCloneTable"
                    | "__cxa_finalize"
            )
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_so = c_library_path();
    let rs_so = rust_library_path();
    assert!(
        rs_so.exists(),
        "{} not found — run `cargo build` before `cargo test`",
        rs_so.display()
    );

    let c_syms = exported_symbols(&c_so);
    let rs_syms = exported_symbols(&rs_so);

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rs_syms:?}",
        c_so.display(),
        rs_so.display()
    );

    // The API functions from lib.c must actually be there (guards against the
    // filter above accidentally emptying both sets).
    for expected in [
        "c2V",
        "c2Mulvs",
        "c2Maxv",
        "c2Minv",
        "c2Clampv",
        "c2Sub",
        "c2Dot",
        "c2CircletoCircle",
        "c2CircletoAABB",
        "c2CircletoCapsule",
        "c2Collided",
        "circle_collide",
    ] {
        assert!(c_syms.contains(expected), "C .so lacks {expected}");
        assert!(rs_syms.contains(expected), "Rust .so lacks {expected}");
    }
}
