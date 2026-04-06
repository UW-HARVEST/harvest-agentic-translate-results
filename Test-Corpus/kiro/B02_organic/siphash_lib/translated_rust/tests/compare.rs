use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

#[test]
fn test_stbds_hash_bytes() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_hash: Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize) -> usize> =
        unsafe { c_lib.get(b"stbds_hash_bytes").unwrap() };

    for seed in [0usize, 1, 42, 0xdeadbeef, usize::MAX] {
        for len in 0..=64 {
            let data: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37).wrapping_add(7)).collect();
            let c_result = unsafe { c_hash(data.as_ptr() as *mut _, data.len(), seed) };
            let r_result = siphash_lib::stbds_hash_bytes(data.as_ptr() as *mut _, data.len(), seed);
            assert_eq!(
                c_result, r_result,
                "stbds_hash_bytes mismatch: seed={seed}, len={len}"
            );
        }
    }
}

#[test]
fn test_stbds_hash_bytes_high_bytes() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_hash: Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize) -> usize> =
        unsafe { c_lib.get(b"stbds_hash_bytes").unwrap() };

    for len in 1..=8 {
        let data: Vec<u8> = (0..len).map(|i| 0x80u8.wrapping_add(i as u8)).collect();
        let c_result = unsafe { c_hash(data.as_ptr() as *mut _, data.len(), 0) };
        let r_result = siphash_lib::stbds_hash_bytes(data.as_ptr() as *mut _, data.len(), 0);
        assert_eq!(c_result, r_result, "high-byte mismatch: len={len}");
    }
}

/// Test siphash() output by comparing via subprocess
#[test]
fn test_siphash_output() {
    use std::process::Command;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so_dir = manifest.join("c_src/build");

    // Write and compile a C driver
    std::fs::write("/tmp/siphash_driver.c", r#"
#include <stdlib.h>
extern void siphash(int init);
int main(int argc, char **argv) {
    siphash(argc > 1 ? atoi(argv[1]) : 0);
    return 0;
}
"#).unwrap();
    let ok = Command::new("gcc")
        .args(["/tmp/siphash_driver.c", "-o", "/tmp/siphash_driver_c",
               "-L", c_so_dir.to_str().unwrap(), "-ltranslated_rust",
               &format!("-Wl,-rpath,{}", c_so_dir.display())])
        .status().unwrap();
    assert!(ok.success(), "Failed to compile C driver");

    // Write and compile a Rust driver linking against our cdylib
    let rust_so_dir = manifest.join("target/debug");
    std::fs::write("/tmp/siphash_driver_rs.rs", r#"
extern "C" { fn siphash(init: i32); }
fn main() {
    let init: i32 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(0);
    unsafe { siphash(init); }
}
"#).unwrap();
    let ok = Command::new("rustc")
        .args(["/tmp/siphash_driver_rs.rs", "-o", "/tmp/siphash_driver_rs",
               "-L", rust_so_dir.to_str().unwrap(), "-l", "siphash_lib",
               "--edition", "2021"])
        .status().unwrap();
    assert!(ok.success(), "Failed to compile Rust driver. Ensure `cargo build` was run first.");

    for init in [0, 1, -1, 42, 127] {
        let c_out = Command::new("/tmp/siphash_driver_c")
            .arg(init.to_string())
            .output().unwrap();
        let r_out = Command::new("/tmp/siphash_driver_rs")
            .arg(init.to_string())
            .env("LD_LIBRARY_PATH", &rust_so_dir)
            .output().unwrap();
        assert!(c_out.status.success(), "C driver failed for init={init}");
        assert!(r_out.status.success(), "Rust driver failed for init={init}: {}", String::from_utf8_lossy(&r_out.stderr));
        assert_eq!(
            c_out.stdout, r_out.stdout,
            "siphash({init}) stdout mismatch"
        );
    }
}
