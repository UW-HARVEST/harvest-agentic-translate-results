use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libsillymain.so")
}

#[test]
fn test_helloworld_return_value() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let c_helloworld: Symbol<unsafe extern "C" fn() -> i32> =
            lib.get(b"helloworld").expect("Failed to find helloworld");
        let c_ret = c_helloworld();
        let rust_ret = sillymain::helloworld();
        assert_eq!(c_ret, rust_ret, "Return value mismatch: C={c_ret}, Rust={rust_ret}");
    }
}

#[test]
fn test_helloworld_stdout() {
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Run C binary
    let c_out = Command::new(manifest_dir.join("c_src/build/driver"))
        .output()
        .expect("Failed to run C binary");

    // Run Rust binary
    let rust_bin = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    let rust_out = Command::new(&rust_bin)
        .output()
        .expect("Failed to run Rust binary");

    assert_eq!(c_out.stdout, rust_out.stdout, 
        "stdout mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out.stdout),
        String::from_utf8_lossy(&rust_out.stdout));
    assert_eq!(c_out.status.code(), rust_out.status.code(),
        "Exit code mismatch");
}
