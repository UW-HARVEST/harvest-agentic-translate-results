use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built by `cargo build` into target/debug/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so")
}

#[test]
fn test_main_return_value() {
    // Ensure the Rust .so is built first
    let rust_so = rust_lib_path();
    assert!(rust_so.exists(), "Rust .so not found at {:?}. Run `cargo build` first.", rust_so);

    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust library");

        let c_main: Symbol<unsafe extern "C" fn() -> i32> =
            c_lib.get(b"main").expect("C main not found");
        let rust_main: Symbol<unsafe extern "C" fn() -> i32> =
            rust_lib.get(b"main").expect("Rust main not found");

        let c_ret = c_main();
        let rust_ret = rust_main();
        assert_eq!(c_ret, rust_ret, "main() return values differ: C={c_ret}, Rust={rust_ret}");
    }
}
