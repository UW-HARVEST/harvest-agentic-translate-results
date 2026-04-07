use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libhello.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libhello.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/release/libhello.so");
    }
    p
}

#[test]
fn test_helloworld_return_value() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            c_lib.get(b"helloworld").expect("C helloworld");
        let r_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            rust_lib.get(b"helloworld").expect("Rust helloworld");

        let c_ret = c_fn();
        let r_ret = r_fn();
        assert_eq!(c_ret, r_ret, "return value mismatch");
    }
}

#[test]
fn test_helloworld_stdout_output() {
    use std::process::Command;

    // Helper program that loads a .so via dlopen and calls helloworld,
    // capturing its stdout. We use a small inline C program for this.
    let helper = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_caller");
    let helper_src = format!(
        r#"
#include <stdio.h>
#include <dlfcn.h>
int main(int argc, char **argv) {{
    void *lib = dlopen(argv[1], RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    int (*fn)() = dlsym(lib, "helloworld");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn();
    dlclose(lib);
    return 0;
}}
"#
    );
    let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_caller.c");
    std::fs::write(&src_path, &helper_src).unwrap();
    let status = Command::new("gcc")
        .args(&[
            src_path.to_str().unwrap(),
            "-o", helper.to_str().unwrap(),
            "-ldl",
        ])
        .status()
        .expect("gcc");
    assert!(status.success(), "compile helper");

    let c_out = Command::new(&helper)
        .arg(c_lib_path().to_str().unwrap())
        .output()
        .expect("run C");
    let r_out = Command::new(&helper)
        .arg(rust_lib_path().to_str().unwrap())
        .output()
        .expect("run Rust");

    assert_eq!(c_out.stdout, r_out.stdout, "stdout mismatch");
    assert_eq!(c_out.status.code(), r_out.status.code(), "exit code mismatch");
}
