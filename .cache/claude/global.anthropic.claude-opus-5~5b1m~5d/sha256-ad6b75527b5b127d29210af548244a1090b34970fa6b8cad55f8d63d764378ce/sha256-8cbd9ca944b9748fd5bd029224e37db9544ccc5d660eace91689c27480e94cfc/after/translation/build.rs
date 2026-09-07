// Reproduce the C build's `__FILE__` for the live `assert()`s.
//
// CMake compiles `c_src/src/lib.c` with an absolute path, so glibc's
// `__assert_fail` receives the absolute path of the C source.  To make the
// Rust translation's abort message byte-identical to the C library's, resolve
// the same path here and hand it to the crate as `CP_ASSERT_FILE`.
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let candidate = std::path::Path::new(&manifest)
        .parent()
        .map(|p| p.join("c_src").join("src").join("lib.c"));

    let file = match candidate {
        Some(p) => match std::fs::canonicalize(&p) {
            Ok(abs) => abs.to_string_lossy().into_owned(),
            // Fall back to the plain relative path CMake would use if the C
            // tree is not next to us (e.g. crate published on its own).
            Err(_) => "src/lib.c".to_string(),
        },
        None => "src/lib.c".to_string(),
    };

    println!("cargo:rustc-env=CP_ASSERT_FILE={}", file);
    println!("cargo:rerun-if-changed=build.rs");
}
