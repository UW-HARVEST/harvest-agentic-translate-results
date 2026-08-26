use std::path::PathBuf;
use std::process::Command;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug/ or target/release/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

/// Helper: write a small C program that dlopen's a .so, calls `run(extra)`,
/// and prints the captured output. We compile and run it to get the output.
/// This avoids all the complexity of capturing stdout in-process.
fn call_run_via_so(lib_path: &std::path::Path, extra_bedrooms: i32) -> String {
    // Use a helper program that loads the .so and calls run
    let helper_src = format!(
        r#"
#include <stdio.h>
#include <dlfcn.h>
int main() {{
    void *h = dlopen("{}", RTLD_NOW);
    if (!h) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*run_fn)(int) = dlsym(h, "run");
    if (!run_fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    run_fn({});
    fflush(stdout);
    dlclose(h);
    return 0;
}}
"#,
        lib_path.display(),
        extra_bedrooms
    );

    let tmp_dir = std::env::temp_dir();
    let src_path = tmp_dir.join("test_helper.c");
    let bin_path = tmp_dir.join("test_helper");
    std::fs::write(&src_path, &helper_src).unwrap();

    let compile = Command::new("gcc")
        .args([
            src_path.to_str().unwrap(),
            "-o", bin_path.to_str().unwrap(),
            "-ldl",
        ])
        .output()
        .unwrap();
    assert!(compile.status.success(), "gcc failed: {}", String::from_utf8_lossy(&compile.stderr));

    let run = Command::new(&bin_path)
        .env("LD_LIBRARY_PATH", lib_path.parent().unwrap())
        .output()
        .unwrap();
    if !run.status.success() {
        panic!(
            "helper failed (exit {:?}):\nstdout: {}\nstderr: {}",
            run.status.code(),
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }
    String::from_utf8(run.stdout).unwrap()
}

#[test]
fn test_run_output_matches() {
    // Build Rust .so first
    let build = Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(build.status.success(), "cargo build failed: {}", String::from_utf8_lossy(&build.stderr));

    let c_lib = c_lib_path();
    let rust_lib = rust_lib_path();
    assert!(c_lib.exists(), "C .so not found at {:?}", c_lib);
    assert!(rust_lib.exists(), "Rust .so not found at {:?}", rust_lib);

    // Test with several input values
    for extra in [0, 1, 3, -1, 10] {
        let c_out = call_run_via_so(&c_lib, extra);
        let rust_out = call_run_via_so(&rust_lib, extra);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for run({extra}):\n--- C ---\n{c_out}\n--- Rust ---\n{rust_out}"
        );
    }
}
