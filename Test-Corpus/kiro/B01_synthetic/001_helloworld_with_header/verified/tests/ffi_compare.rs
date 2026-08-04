use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libsillymain.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so")
}

#[test]
fn test_helloworld_return_value() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_fn: Symbol<unsafe extern "C" fn() -> i32> =
            c_lib.get(b"helloworld").expect("C helloworld");
        let r_fn: Symbol<unsafe extern "C" fn() -> i32> =
            r_lib.get(b"helloworld").expect("Rust helloworld");

        let c_ret = c_fn();
        let r_ret = r_fn();
        assert_eq!(c_ret, r_ret, "return values differ: C={c_ret}, Rust={r_ret}");
    }
}

#[test]
fn test_helloworld_stdout() {
    // Capture only the library's stdout via python, flushing before/after
    let script = |path: &str| format!(
        "import ctypes,sys; sys.stdout.flush(); lib = ctypes.CDLL('{}'); lib.helloworld(); sys.stdout.flush()",
        path
    );

    let c_out = Command::new("python3")
        .arg("-c").arg(script(&c_lib_path().to_string_lossy()))
        .output().expect("run C subprocess");

    let r_out = Command::new("python3")
        .arg("-c").arg(script(&rust_lib_path().to_string_lossy()))
        .output().expect("run Rust subprocess");

    assert_eq!(c_out.stdout, r_out.stdout, "stdout differs:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out.stdout),
        String::from_utf8_lossy(&r_out.stdout));
}
