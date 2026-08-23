// Build script: locate the system zlib and tell rustc to link it.
//
// libpng does not implement DEFLATE itself; it calls zlib.  The C reference
// build links the system zlib, so this translation links exactly the same
// library in order to produce byte-identical compressed output.
//
// Some systems ship only the runtime SONAME (libz.so.1) without the linker
// symlink (libz.so).  In that case a symlink is created inside OUT_DIR and the
// directory is added to the link search path.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let candidates = [
        "/usr/lib64",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib",
        "/lib64",
        "/lib/x86_64-linux-gnu",
        "/lib",
        "/usr/local/lib64",
        "/usr/local/lib",
    ];

    // If a linkable libz.so (or libz.a) already exists, the default search path
    // handles everything.
    for dir in candidates.iter() {
        let d = Path::new(dir);
        if d.join("libz.so").exists() || d.join("libz.a").exists() {
            println!("cargo:rustc-link-search=native={}", dir);
            println!("cargo:rustc-link-lib=dylib=z");
            return;
        }
    }

    // Otherwise look for a versioned runtime library and create a symlink.
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string());
    for dir in candidates.iter() {
        let d = Path::new(dir);
        for name in ["libz.so.1", "libz.so.1.2.11", "libz.so.1.3", "libz.so.1.3.1"] {
            let p = d.join(name);
            if p.exists() {
                let link: PathBuf = Path::new(&out_dir).join("libz.so");
                let _ = std::fs::remove_file(&link);
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(&p, &link);
                println!("cargo:rustc-link-search=native={}", out_dir);
                println!("cargo:rustc-link-lib=dylib=z");
                return;
            }
        }
    }

    // Last resort: hope the linker can find it on its own.
    println!("cargo:rustc-link-lib=dylib=z");
}
