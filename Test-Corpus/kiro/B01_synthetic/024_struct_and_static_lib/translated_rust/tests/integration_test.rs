use std::process::Command;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    project_root().join("target/debug/libdriver.so")
}

fn call_via_dlopen(lib_path: &PathBuf, func: &str, arg: i32) -> String {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir();
    let c_file = tmp_dir.join(format!("test_drv_{id}.c"));
    let bin_file = tmp_dir.join(format!("test_drv_{id}"));

    let test_prog = format!(
        r#"#include <stdio.h>
#include <dlfcn.h>
int main() {{
    void *lib = dlopen("{lib}", RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*fn)(int) = dlsym(lib, "{func}");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn({arg});
    fflush(stdout);
    dlclose(lib);
    return 0;
}}"#,
        lib = lib_path.display(),
        func = func,
        arg = arg,
    );

    std::fs::write(&c_file, &test_prog).unwrap();
    let compile = Command::new("gcc")
        .args(&[c_file.to_str().unwrap(), "-o", bin_file.to_str().unwrap(), "-ldl"])
        .output()
        .expect("gcc failed");
    assert!(compile.status.success(), "gcc: {}", String::from_utf8_lossy(&compile.stderr));

    let run = Command::new(&bin_file)
        .output()
        .expect("run failed");
    assert!(run.status.success(), "run failed: {}", String::from_utf8_lossy(&run.stderr));

    let _ = std::fs::remove_file(&c_file);
    let _ = std::fs::remove_file(&bin_file);

    String::from_utf8(run.stdout).unwrap()
}

fn compare(func: &str, arg: i32) {
    let c_out = call_via_dlopen(&c_lib_path(), func, arg);
    let rust_out = call_via_dlopen(&rust_lib_path(), func, arg);
    assert_eq!(c_out, rust_out, "{func}({arg}) mismatch.\nC:\n{c_out}\nRust:\n{rust_out}");
}

#[test]
fn test_run_3() { compare("run", 3); }

#[test]
fn test_driver_3() { compare("driver", 3); }

#[test]
fn test_driver_0() { compare("driver", 0); }

#[test]
fn test_run_neg2() { compare("run", -2); }
