fn main() {
    println!("cargo:rerun-if-changed=shim.c");
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // Only the cdylib needs the version script (it forces the C shim's
    // variadic symbols to be exported). Applying it to test binaries would
    // fail, since they do not link the shim.
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}/export.map",
        dir
    );
    cc::Build::new()
        .file("shim.c")
        .flag("-fexceptions")
        .flag("-funwind-tables")
        .warnings(false)
        .compile("mujs_shim");
}
