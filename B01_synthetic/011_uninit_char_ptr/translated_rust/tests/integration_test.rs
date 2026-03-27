use libloading::{Library, Symbol};
use std::ffi::CString;
use std::process::{Command, Stdio};
use std::io::Write;

fn c_lib_path() -> String {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", dir)
}

fn rust_lib_path() -> String {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // cdylib is built in target/debug/
    format!("{}/target/debug/libdriver.so", dir)
}

/// Helper: run a small C program that dlopen's a .so, calls a function, captures stdout
fn call_void_fn_via_helper(lib_path: &str, fn_name: &str) -> Vec<u8> {
    // We write a tiny C helper, compile and run it
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let helper_src = format!("{}/target/test_helper.c", dir);
    let helper_bin = format!("{}/target/test_helper", dir);

    let src = format!(r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main() {{
    void *lib = dlopen("{lib_path}", RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*fn)() = dlsym(lib, "{fn_name}");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn();
    fflush(stdout);
    dlclose(lib);
    return 0;
}}
"#);

    std::fs::write(&helper_src, &src).unwrap();
    let status = Command::new("gcc")
        .args(&[&helper_src, "-o", &helper_bin, "-ldl"])
        .status()
        .expect("gcc");
    assert!(status.success(), "Failed to compile test helper");

    let output = Command::new(&helper_bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run helper");

    if !output.status.success() {
        panic!("Helper failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    output.stdout
}

fn call_printline_via_helper(lib_path: &str, arg: &str) -> Vec<u8> {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let helper_src = format!("{}/target/test_helper_pl.c", dir);
    let helper_bin = format!("{}/target/test_helper_pl", dir);

    let src = format!(r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main() {{
    void *lib = dlopen("{lib_path}", RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*fn)(const char*) = dlsym(lib, "printLine");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn("{arg}");
    fflush(stdout);
    dlclose(lib);
    return 0;
}}
"#);

    std::fs::write(&helper_src, &src).unwrap();
    let status = Command::new("gcc")
        .args(&[&helper_src, "-o", &helper_bin, "-ldl"])
        .status()
        .expect("gcc");
    assert!(status.success());

    let output = Command::new(&helper_bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run helper");

    if !output.status.success() {
        panic!("Helper failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    output.stdout
}

fn call_printline_null_via_helper(lib_path: &str) -> Vec<u8> {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let helper_src = format!("{}/target/test_helper_pln.c", dir);
    let helper_bin = format!("{}/target/test_helper_pln", dir);

    let src = format!(r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main() {{
    void *lib = dlopen("{lib_path}", RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*fn)(const char*) = dlsym(lib, "printLine");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn(NULL);
    fflush(stdout);
    dlclose(lib);
    return 0;
}}
"#);

    std::fs::write(&helper_src, &src).unwrap();
    let status = Command::new("gcc")
        .args(&[&helper_src, "-o", &helper_bin, "-ldl"])
        .status()
        .expect("gcc");
    assert!(status.success());

    let output = Command::new(&helper_bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run helper");

    if !output.status.success() {
        panic!("Helper failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    output.stdout
}

// ---- printLine tests ----

#[test]
fn test_printline_with_string() {
    let c_out = call_printline_via_helper(&c_lib_path(), "hello");
    let r_out = call_printline_via_helper(&rust_lib_path(), "hello");
    assert_eq!(c_out, r_out,
        "printLine('hello') mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_printline_null() {
    let c_out = call_printline_null_via_helper(&c_lib_path());
    let r_out = call_printline_null_via_helper(&rust_lib_path());
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

// ---- good() test ----

#[test]
fn test_good() {
    let c_out = call_void_fn_via_helper(&c_lib_path(), "good");
    let r_out = call_void_fn_via_helper(&rust_lib_path(), "good");
    assert_eq!(c_out, r_out,
        "good() mismatch:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

// ---- binary output comparison ----

#[test]
fn test_binary_output_good_path() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let c_bin = format!("{}/c_src/build/driver_bin", dir);
    let rust_bin = format!("{}/target/debug/driver", dir);

    let c_output = Command::new(&c_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(b"1\n").unwrap();
            child.wait_with_output()
        })
        .expect("run C binary");

    let rust_output = Command::new(&rust_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(b"1\n").unwrap();
            child.wait_with_output()
        })
        .expect("run Rust binary");

    assert_eq!(
        c_output.stdout, rust_output.stdout,
        "Binary output mismatch for input '1':\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_output.stdout),
        String::from_utf8_lossy(&rust_output.stdout)
    );
}
