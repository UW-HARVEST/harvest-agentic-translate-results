// The C code's assert() diagnostics embed __FILE__, which is the path the C
// compiler was invoked with (cmake uses the absolute path of src/lib.c).
// Reproduce it as closely as possible so failing STBDS_ASSERTs print the same
// text they do in the C library.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut file = String::from("src/lib.c");
    for base in [manifest.join("../c_src"), manifest.join("c_src")] {
        let candidate = base.join("src/lib.c");
        if candidate.exists() {
            if let Ok(canon) = candidate.canonicalize() {
                file = canon.to_string_lossy().into_owned();
            } else {
                file = candidate.to_string_lossy().into_owned();
            }
            break;
        }
    }
    println!("cargo:rustc-env=STBDS_C_FILE={}", file);
}
