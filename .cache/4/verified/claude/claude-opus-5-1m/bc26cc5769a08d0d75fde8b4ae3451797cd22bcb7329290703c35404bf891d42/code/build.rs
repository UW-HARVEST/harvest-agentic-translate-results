// Reproduce the C translation unit's `__FILE__` so that the text emitted by
// `STBDS_ASSERT` (glibc `assert` -> `__assert_fail`) is byte-identical to the
// C shared library's.  CMake compiles `c_src/src/lib.c` with an absolute path,
// so `__FILE__` is the canonical absolute path of that file.
//
// If `c_src` is not available at build time we fall back to the source-relative
// spelling; the abort *behaviour* (SIGABRT, same assertion text, function name
// and line number) is unaffected either way.

use std::path::PathBuf;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let c_file = PathBuf::from(&manifest).join("c_src").join("src").join("lib.c");

    let path = std::fs::canonicalize(&c_file)
        .ok()
        .and_then(|p| p.to_str().map(str::to_owned))
        .filter(|s| !s.contains('\0'))
        .unwrap_or_else(|| "src/lib.c".to_owned());

    println!("cargo:rustc-env=STBDS_ASSERT_FILE={path}");
    println!("cargo:rerun-if-changed=c_src/src/lib.c");
}
