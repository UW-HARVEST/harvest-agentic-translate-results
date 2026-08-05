fn main() {
    println!("cargo:rerun-if-changed=shim.c");
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-Wl,--version-script={}/export.map", dir);
    cc::Build::new()
        .file("shim.c")
        .flag("-fexceptions")
        .flag("-funwind-tables")
        .warnings(false)
        .compile("mujs_shim");
}
