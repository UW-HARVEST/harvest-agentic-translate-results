// Reproduce the `__FILE__` string that `<assert.h>` bakes into the reference C
// library, so that a failing `assert()` prints a byte-identical message.
//
// CMake compiles the translation unit by absolute path
// (`/usr/bin/cc ... -c <root>/c_src/src/lib.c`, see
// `c_src/build/CMakeFiles/*/build.make`), therefore the C `__FILE__` expands to
// `<root>/c_src/src/lib.c` where `<root>` is the parent of this crate.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    );
    let root = manifest.parent().map(PathBuf::from).unwrap_or_else(|| manifest.clone());
    let file = root.join("c_src").join("src").join("lib.c");

    println!("cargo:rustc-env=CP_ASSERT_FILE={}", file.display());
}
