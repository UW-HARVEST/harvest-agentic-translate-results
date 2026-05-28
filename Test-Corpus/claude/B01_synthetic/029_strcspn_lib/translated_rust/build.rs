use std::io::Write;

fn main() {
    // Match the exported linker-defined symbols that the C shared library
    // exposes (_init, _fini, _edata, _end, __bss_start). Rust's
    // auto-generated cdylib version script tags symbols anonymously and
    // marks every other symbol as local, so we cannot easily override it
    // with a second version script. Instead, install a tiny linker
    // wrapper script that runs `cc` as usual and then runs `objcopy
    // --globalize-symbol=...` on the output to flip the binding from
    // local to global.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let wrapper_path = format!("{}/linker_wrapper.sh", out_dir);
    let wrapper = r#"#!/usr/bin/env bash
set -euo pipefail
out=""
prev=""
for arg in "$@"; do
    if [ "$prev" = "-o" ]; then
        out="$arg"
    fi
    prev="$arg"
done
cc "$@"
if [ -n "$out" ] && [ -f "$out" ]; then
    if file "$out" 2>/dev/null | grep -q "shared object"; then
        for sym in _init _fini _edata _end __bss_start; do
            objcopy --globalize-symbol="$sym" "$out" "$out" 2>/dev/null || true
        done
    fi
fi
"#;
    let mut f = std::fs::File::create(&wrapper_path).expect("create wrapper");
    f.write_all(wrapper.as_bytes()).expect("write wrapper");
    drop(f);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod wrapper");
    println!("cargo:rustc-linker={}", wrapper_path);
    println!("cargo:rerun-if-changed=build.rs");
}
