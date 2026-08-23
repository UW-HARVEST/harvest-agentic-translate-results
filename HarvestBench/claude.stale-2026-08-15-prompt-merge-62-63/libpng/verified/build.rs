use std::process::Command;

fn main() {
    // libpng calls into the system zlib for DEFLATE/INFLATE so that the
    // compressed output is byte-identical to the reference C build (which also
    // links the system zlib).
    println!("cargo:rustc-link-lib=dylib=z");
    // Math functions (pow, floor, modf, frexp) come from libm.
    println!("cargo:rustc-link-lib=dylib=m");

    // Compile a tiny setjmp shim: the simplified-API png_safe_execute needs a
    // real setjmp landing pad that png_safe_error's longjmp returns to.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let obj = format!("{out_dir}/shim.o");
    let lib = format!("{out_dir}/libpngshim.a");
    println!("cargo:rerun-if-changed=csupport/shim.c");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .args(["-c", "-O2", "-fPIC", "csupport/shim.c", "-o", &obj])
        .status()
        .expect("failed to run C compiler for shim");
    assert!(status.success(), "shim compilation failed");

    let ar = std::env::var("AR").unwrap_or_else(|_| "ar".to_string());
    let status = Command::new(&ar)
        .args(["crus", &lib, &obj])
        .status()
        .expect("failed to run ar for shim");
    assert!(status.success(), "shim archiving failed");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=pngshim");
}
