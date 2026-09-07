use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // The reference C library is compiled by the supplied CMakeLists.txt
    // without NDEBUG, so `assert()` is live.  glibc's failing-assertion
    // message embeds `__FILE__`, which cmake supplies as the absolute path of
    // `c_src/src/lib.c`.  Reproduce that path so the diagnostic emitted on
    // stderr before aborting is byte-identical too.
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let candidates = [
        manifest.join("..").join("c_src").join("src").join("lib.c"),
        manifest.join("c_src").join("src").join("lib.c"),
        manifest.join("..").join("..").join("c_src").join("src").join("lib.c"),
    ];
    for c in candidates.iter() {
        if let Ok(p) = c.canonicalize() {
            println!("cargo:rustc-env=CP_C_SOURCE_PATH={}", p.display());
            return;
        }
    }
    println!("cargo:rustc-env=CP_C_SOURCE_PATH=src/lib.c");
}
