// Promote linker-generated scaffolding symbols (_init, _fini, __bss_start,
// _edata, _end) to dynamic-table exports, so the Rust cdylib's `nm -D` output
// matches the C-built shared library's. These symbols are produced by
// crti.o/crtn.o and the default linker script; gcc exports them by default,
// rustc does not.
//
// We pass them through a linker version script via -Wl,--dynamic-list so the
// link succeeds even if those symbols don't exist (rather than --version-script,
// which is exhaustive).
use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" && target_os != "android" {
        return;
    }

    // Write a dynamic-list file. Using --dynamic-list (not --version-script)
    // means missing symbols don't break the link.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dyn_list = out_dir.join("dynamic-list.txt");
    std::fs::write(
        &dyn_list,
        "{\n    _init;\n    _fini;\n    __bss_start;\n    _edata;\n    _end;\n};\n",
    )
    .expect("write dynamic-list");

    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={}",
        dyn_list.display()
    );
    // -E / --export-dynamic forces all global symbols into the dynsym table,
    // which is what gcc does for executables. Without this, rustc's strip-pass
    // hides linker-scaffolding symbols (_init, _fini, etc.).
    println!("cargo:rustc-cdylib-link-arg=-Wl,-E");
    println!("cargo:rerun-if-changed=build.rs");
}
