// Force the linker to export the same set of standard symbols that a C-built
// shared library exports by default: _init, _fini, __bss_start, _edata, _end.
// Rust's cdylib build narrows the dynamic symbol table via a version script,
// hiding these. We add them back through a `--dynamic-list` file.

use std::io::Write;

fn main() {
    // Only meaningful when we're producing an ELF shared library on linux.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" && target_os != "android" {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let dynlist_path = format!("{}/extra-dynamic-list.txt", out_dir);
    {
        let mut f = std::fs::File::create(&dynlist_path).expect("create dynamic list");
        // dynamic-list syntax: { sym1; sym2; ...; };
        writeln!(f, "{{").unwrap();
        for sym in &["_init", "_fini", "__bss_start", "_edata", "_end"] {
            writeln!(f, "    {};", sym).unwrap();
        }
        writeln!(f, "}};").unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-Wl,--dynamic-list={}", dynlist_path);
}
