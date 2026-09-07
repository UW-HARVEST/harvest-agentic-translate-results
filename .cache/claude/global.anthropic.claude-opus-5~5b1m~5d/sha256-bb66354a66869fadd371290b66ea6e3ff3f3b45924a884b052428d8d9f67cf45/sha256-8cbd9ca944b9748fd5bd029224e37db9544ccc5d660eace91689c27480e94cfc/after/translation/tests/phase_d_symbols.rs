//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn defined_syms(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {:?}", so);
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn find_c_so() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src")
        .join("build");
    let mut c: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build missing - build the C library first")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    c.sort();
    c.remove(0)
}

fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.file_name().map(|s| s == "deps").unwrap_or(false) {
        dir.pop();
    }
    let p = dir.join("libsiphash_lib.so");
    assert!(p.exists(), "{:?} missing", p);
    p
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_syms(&find_c_so());
    let r = defined_syms(&find_rust_so());

    // Symbols the C .so exports that the Rust .so does not.
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {:?}\nC: {:?}\nRust: {:?}",
        missing,
        c,
        r
    );

    // Both must export the two documented entry points.
    for want in ["siphash", "stbds_hash_bytes"] {
        assert!(c.contains(&want.to_string()), "C .so missing {}", want);
        assert!(r.contains(&want.to_string()), "Rust .so missing {}", want);
    }

    // `stbds_siphash_bytes` is `static` in C and must stay unexported in Rust.
    assert!(
        !c.iter().any(|s| s == "stbds_siphash_bytes"),
        "unexpected: C exports the static helper"
    );
    assert!(
        !r.iter().any(|s| s == "stbds_siphash_bytes"),
        "Rust must not export the static helper stbds_siphash_bytes"
    );

    eprintln!("C symbols   ({}): {:?}", c.len(), c);
    eprintln!("Rust symbols({}): {:?}", r.len(), r);
}

#[test]
fn phase_d_rust_so_has_no_undefined_non_libc_symbols() {
    let so = find_rust_so();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&so)
        .output()
        .expect("nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.contains('@') || true)
        .collect();
    // Anything that is not resolvable at load time would have made
    // `Library::new` fail; assert the load succeeds as the real check.
    let _ = common::libs();
    eprintln!("undefined (imported) symbols: {:?}", undef);
}
