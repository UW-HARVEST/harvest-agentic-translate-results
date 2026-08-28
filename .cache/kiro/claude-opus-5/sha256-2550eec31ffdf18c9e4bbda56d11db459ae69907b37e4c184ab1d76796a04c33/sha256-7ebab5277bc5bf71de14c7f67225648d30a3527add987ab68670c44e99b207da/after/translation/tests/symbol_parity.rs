//! Verifies that every dynamic symbol the C `.so` exports is also exported by
//! the Rust `.so` under the exact same name.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
    }
    candidates.extend([
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ]);
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    panic!("Rust libdriver.so not found; run `cargo build --release`");
}

/// Defined (`--defined-only`) dynamic symbols of a shared object.
fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        c.exists(),
        "C libdriver.so not built; see the cmake instructions in the task"
    );
    let r = rust_so();

    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    // Symbols the C toolchain injects into every shared object; not part of the
    // library's own API surface.
    const TOOLCHAIN: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
    ];

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !TOOLCHAIN.contains(&s.as_str()))
        .filter(|s| !r_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c_syms:?}\nRust: {r_syms:?}"
    );

    // The public API must actually be there.
    assert!(
        c_syms.iter().any(|s| s == "encode_base64"),
        "C .so unexpectedly lacks encode_base64"
    );
    assert!(
        r_syms.iter().any(|s| s == "encode_base64"),
        "Rust .so lacks encode_base64"
    );
}
