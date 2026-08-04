use std::ffi::c_int;
use std::process::Command;

/// Build a tiny C helper that loads a .so, calls driver(x), and prints to stdout.
/// We compile it once and reuse it.
fn build_helper() -> String {
    let dir = std::env::temp_dir().join("ffi_test_helper");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("helper.c");
    let bin = dir.join("helper");
    if !bin.exists() {
        let code = r#"
#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
int main(int argc, char **argv) {
    if (argc != 3) return 1;
    void *lib = dlopen(argv[1], RTLD_NOW);
    if (!lib) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    void (*fn)(int) = dlsym(lib, "driver");
    if (!fn) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }
    fn(atoi(argv[2]));
    dlclose(lib);
    return 0;
}
"#;
        std::fs::write(&src, code).unwrap();
        let status = Command::new("cc")
            .args(&[src.to_str().unwrap(), "-ldl", "-o", bin.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success(), "failed to compile helper");
    }
    bin.to_str().unwrap().to_string()
}

fn call_driver(helper: &str, lib_path: &str, x: c_int) -> String {
    let output = Command::new(helper)
        .args(&[lib_path, &x.to_string()])
        .output()
        .expect("failed to run helper");
    assert!(output.status.success(), "helper failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn compare_driver() {
    let helper = build_helper();
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_so = format!("{}/c_src/build/libdriver.so", manifest);
    let rust_so = format!("{}/target/debug/libdriver.so", manifest);

    // Ensure Rust .so is built
    assert!(std::path::Path::new(&c_so).exists(), "C .so not found");
    assert!(std::path::Path::new(&rust_so).exists(), "Rust .so not found");

    for &x in &[0, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = call_driver(&helper, &c_so, x);
        let r_out = call_driver(&helper, &rust_so, x);
        assert_eq!(
            c_out, r_out,
            "mismatch for driver({x}):\n  C:    {c_out:?}\n  Rust: {r_out:?}"
        );
    }
}
