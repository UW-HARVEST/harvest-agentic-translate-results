use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile(output: &Path, extra_source: Option<&Path>) {
    let mut command = Command::new("cc");
    command.args(["-shared", "-fPIC", "-Ic_src/include", "c_src/src/lib.c"]);
    if let Some(source) = extra_source {
        command.args(["-Dmalloc=dag_malloc", "-Dfree=dag_free"]);
        command.arg(source);
    }
    let status = command
        .arg("-o")
        .arg(output)
        .status()
        .expect("failed to invoke C compiler");
    assert!(status.success(), "C shared-library build failed");
}

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set"));
    let c_library = output.join("libdag_c.so");
    let fail_library = output.join("libdag_c_fail.so");

    compile(&c_library, None);
    compile(&fail_library, Some(Path::new("tests/support/alloc_shim.c")));

    println!("cargo:rustc-env=DAG_C_SO={}", c_library.display());
    println!("cargo:rustc-env=DAG_C_FAIL_SO={}", fail_library.display());
    println!("cargo:rerun-if-changed=c_src/src/lib.c");
    println!("cargo:rerun-if-changed=c_src/include/dag_lib.h");
    println!("cargo:rerun-if-changed=tests/support/alloc_shim.c");
}
