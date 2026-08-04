fn main() {
    // Mirror the C `libdriver.so` which exports `_init` and `_fini` from
    // crti.o. By default Rust links the shared library but those symbols
    // remain local. Add `-Wl,-u,_init` / `-Wl,-u,_fini` to force them to
    // be retained, and re-add them to the dynamic symbol table via
    // `--export-dynamic`. The combination produces a .so whose `nm -D`
    // output matches the C build for these startup hooks.
    println!("cargo:rustc-link-arg=-Wl,-u,_init");
    println!("cargo:rustc-link-arg=-Wl,-u,_fini");
    println!("cargo:rustc-link-arg=-Wl,--export-dynamic");
}
