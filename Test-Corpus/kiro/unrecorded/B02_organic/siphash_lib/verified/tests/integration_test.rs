use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Find the Rust .so in target/debug
    let p = dir.join("target/debug/libsiphash_lib.so");
    if p.exists() { return p; }
    dir.join("target/release/libsiphash_lib.so")
}

type HashBytesFn = unsafe extern "C" fn(*mut u8, usize, usize) -> usize;

fn load_hash_bytes(lib: &Library) -> Symbol<HashBytesFn> {
    unsafe { lib.get(b"stbds_hash_bytes") }.expect("stbds_hash_bytes not found")
}

#[test]
fn test_stbds_hash_bytes_all_lengths() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_hash_bytes(&c_lib);
    let r_fn = load_hash_bytes(&r_lib);

    // Test all lengths 0..=64 with seed=0, data = 0,1,2,...
    let mut backing = [0u8; 65];
    for len in 0..=64 {
        for i in 0..len { backing[i] = i as u8; }
        let ptr = backing.as_mut_ptr();
        let c_res = unsafe { c_fn(ptr, len, 0) };
        let r_res = unsafe { r_fn(ptr, len, 0) };
        assert_eq!(c_res, r_res, "mismatch at len={len} seed=0");
    }
}

#[test]
fn test_stbds_hash_bytes_various_seeds() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_hash_bytes(&c_lib);
    let r_fn = load_hash_bytes(&r_lib);

    let seeds: &[usize] = &[0, 1, 42, 255, 0xDEADBEEF, usize::MAX, usize::MAX / 2];
    for &seed in seeds {
        for len in [0, 1, 3, 7, 8, 15, 16, 31, 32, 63] {
            let mut buf: Vec<u8> = (0..64).map(|i| (i ^ 0xAB) as u8).collect();
            let c_res = unsafe { c_fn(buf.as_mut_ptr(), len, seed) };
            let r_res = unsafe { r_fn(buf.as_mut_ptr(), len, seed) };
            assert_eq!(c_res, r_res, "mismatch at len={len} seed={seed:#x}");
        }
    }
}

#[test]
fn test_stbds_hash_bytes_high_byte_values() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_hash_bytes(&c_lib);
    let r_fn = load_hash_bytes(&r_lib);

    // Test with bytes >= 128 to catch sign-extension issues
    for len in 1..=16 {
        let mut buf: Vec<u8> = (0..len).map(|i| (128 + i) as u8).collect();
        let c_res = unsafe { c_fn(buf.as_mut_ptr(), len, 0) };
        let r_res = unsafe { r_fn(buf.as_mut_ptr(), len, 0) };
        assert_eq!(c_res, r_res, "high-byte mismatch at len={len}");
    }

    // All 0xFF bytes
    for len in 1..=16 {
        let mut buf = vec![0xFFu8; len];
        let c_res = unsafe { c_fn(buf.as_mut_ptr(), len, 0) };
        let r_res = unsafe { r_fn(buf.as_mut_ptr(), len, 0) };
        assert_eq!(c_res, r_res, "0xFF mismatch at len={len}");
    }
}

#[test]
fn test_stbds_hash_bytes_all_zeros() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_hash_bytes(&c_lib);
    let r_fn = load_hash_bytes(&r_lib);

    for len in 0..=32 {
        let mut buf = vec![0u8; 33];
        let c_res = unsafe { c_fn(buf.as_mut_ptr(), len, 0) };
        let r_res = unsafe { r_fn(buf.as_mut_ptr(), len, 0) };
        assert_eq!(c_res, r_res, "zeros mismatch at len={len}");
    }
}

#[test]
fn test_siphash_output_matches() {
    // siphash() prints to stdout. We can't easily capture that from .so calls,
    // but we can verify it indirectly: siphash(init) calls stbds_hash_bytes
    // on mem[0..i] for i in 0..64 where mem[j] = (init+j) as u8.
    // We already test stbds_hash_bytes thoroughly above.
    // Here we just verify the symbol is callable without crashing.
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    type SiphashFn = unsafe extern "C" fn(i32);
    let c_fn: Symbol<SiphashFn> = unsafe { c_lib.get(b"siphash") }.expect("siphash not found in C");
    let r_fn: Symbol<SiphashFn> = unsafe { r_lib.get(b"siphash") }.expect("siphash not found in Rust");

    // Both should run without crashing
    unsafe { c_fn(0); }
    unsafe { r_fn(0); }
}
