fn main() {
    // libpng does not implement DEFLATE; it calls zlib.  Link the system zlib
    // so that the compressed output is bit-for-bit identical to the reference
    // C build.
    println!("cargo:rustc-link-lib=dylib=z");
}
