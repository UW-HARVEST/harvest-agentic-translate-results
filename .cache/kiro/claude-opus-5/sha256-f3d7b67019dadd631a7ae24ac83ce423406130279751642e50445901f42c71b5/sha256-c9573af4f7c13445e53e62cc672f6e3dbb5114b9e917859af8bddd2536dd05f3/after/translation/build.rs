fn main() {
    // libpng defers DEFLATE/INFLATE and CRC-32 to zlib.  The reference C build
    // links the system zlib; do the same so the compressed byte streams are
    // bit-for-bit identical.
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=m");
}
