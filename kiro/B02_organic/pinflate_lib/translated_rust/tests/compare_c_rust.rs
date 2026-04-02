use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};

fn c_lib_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libpinflate_lib.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // Find the Rust cdylib in target/debug/deps or target/debug
    let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    for entry in ["libpinflate_lib.so"] {
        let p = target_dir.join(entry);
        if p.exists() {
            return p;
        }
    }
    // Try glob
    for entry in std::fs::read_dir(&target_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("libpinflate_lib") && name.ends_with(".so") {
            return entry.path();
        }
    }
    panic!("Could not find Rust .so in {:?}", target_dir);
}

/// Call pinflate from a given .so library
unsafe fn call_pinflate(lib: &Library, input: &[u8], out_size: usize) -> (i32, Vec<u8>) {
    let func: Symbol<unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int> =
        lib.get(b"pinflate").unwrap();
    let mut output = vec![0u8; out_size];
    let ret = func(
        input.as_ptr() as *mut c_void,
        input.len() as c_int,
        output.as_mut_ptr() as *mut c_void,
        out_size as c_int,
    );
    (ret, output)
}

/// Read a data symbol from a library as a byte slice
unsafe fn read_symbol_bytes<'a>(lib: &'a Library, name: &[u8], len: usize) -> &'a [u8] {
    let sym: Symbol<*const u8> = lib.get(name).unwrap();
    std::slice::from_raw_parts(*sym, len)
}

unsafe fn read_symbol_u32s<'a>(lib: &'a Library, name: &[u8], count: usize) -> &'a [u32] {
    let sym: Symbol<*const u32> = lib.get(name).unwrap();
    std::slice::from_raw_parts(*sym, count)
}

// ---- Static data comparison tests ----

#[test]
fn test_cp_fixed_table() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_bytes(&c_lib, b"cp_fixed_table", 320);
        let r_data = read_symbol_bytes(&rust_lib, b"cp_fixed_table", 320);
        assert_eq!(c_data, r_data, "cp_fixed_table mismatch");
    }
}

#[test]
fn test_cp_permutation_order() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_bytes(&c_lib, b"cp_permutation_order", 19);
        let r_data = read_symbol_bytes(&rust_lib, b"cp_permutation_order", 19);
        assert_eq!(c_data, r_data, "cp_permutation_order mismatch");
    }
}

#[test]
fn test_cp_len_extra_bits() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_bytes(&c_lib, b"cp_len_extra_bits", 31);
        let r_data = read_symbol_bytes(&rust_lib, b"cp_len_extra_bits", 31);
        assert_eq!(c_data, r_data, "cp_len_extra_bits mismatch");
    }
}

#[test]
fn test_cp_len_base() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_u32s(&c_lib, b"cp_len_base", 31);
        let r_data = read_symbol_u32s(&rust_lib, b"cp_len_base", 31);
        assert_eq!(c_data, r_data, "cp_len_base mismatch");
    }
}

#[test]
fn test_cp_dist_extra_bits() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_bytes(&c_lib, b"cp_dist_extra_bits", 32);
        let r_data = read_symbol_bytes(&rust_lib, b"cp_dist_extra_bits", 32);
        assert_eq!(c_data, r_data, "cp_dist_extra_bits mismatch");
    }
}

#[test]
fn test_cp_dist_base() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_data = read_symbol_u32s(&c_lib, b"cp_dist_base", 32);
        let r_data = read_symbol_u32s(&rust_lib, b"cp_dist_base", 32);
        assert_eq!(c_data, r_data, "cp_dist_base mismatch");
    }
}

// ---- pinflate functional tests ----

fn run_pinflate_comparison(compressed: &[u8], expected_decompressed_len: usize, label: &str) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();

        let (c_ret, c_out) = call_pinflate(&c_lib, compressed, expected_decompressed_len);
        let (r_ret, r_out) = call_pinflate(&rust_lib, compressed, expected_decompressed_len);

        assert_eq!(c_ret, r_ret, "{}: return code mismatch (C={}, Rust={})", label, c_ret, r_ret);
        assert_eq!(c_out, r_out, "{}: output mismatch", label);
    }
}

#[test]
fn test_pinflate_hello_world() {
    // "Hello, World!" compressed with fixed Huffman
    let compressed: Vec<u8> = vec![243,72,205,201,201,215,81,8,207,47,202,73,81,4,0];
    run_pinflate_comparison(&compressed, 13, "hello_world");
}

#[test]
fn test_pinflate_repeated_pattern() {
    // "ABCDEFGHIJ" * 100 compressed with dynamic Huffman
    let compressed: Vec<u8> = vec![115,116,114,118,113,117,115,247,240,244,114,28,101,141,178,70,89,195,148,5,0];
    run_pinflate_comparison(&compressed, 1000, "repeated_pattern");
}

#[test]
fn test_pinflate_all_zeros() {
    // 256 zero bytes compressed
    let compressed: Vec<u8> = vec![99,96,24,217,0,0];
    run_pinflate_comparison(&compressed, 256, "all_zeros");
}

#[test]
fn test_pinflate_stored_block() {
    // "Hello, World!" stored (uncompressed) block
    let compressed: Vec<u8> = vec![1,13,0,242,255,72,101,108,108,111,44,32,87,111,114,108,100,33];
    run_pinflate_comparison(&compressed, 13, "stored_block");
}
