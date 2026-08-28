//! Records the path the original C translation unit was compiled from.
//!
//! `c_src/src/lib.c` is built with asserts **enabled** (CMake adds no
//! `-DNDEBUG`, and `nm -D` shows `__assert_fail` among the imports). A failing
//! `assert` prints
//!
//! ```text
//! <progname>: <__FILE__>:<line>: <function>: Assertion `<expr>' failed.
//! ```
//!
//! and raises `SIGABRT`. To reproduce that byte-for-byte the Rust translation
//! calls glibc's `__assert_fail` directly and therefore needs the same
//! `__FILE__` string. CMake compiles the absolute path, so it is resolved here.
//!
//! If `c_src` is not present (the crate built standalone) the relative path from
//! `CMakeLists.txt` is used instead; only the diagnostic text is affected, never
//! the control flow or the termination signal.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let candidate = manifest.join("..").join("c_src").join("src").join("lib.c");
    let file = std::fs::canonicalize(&candidate)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "src/lib.c".to_string());
    println!("cargo:rustc-env=STBDS_C_SOURCE_FILE={file}");
}
