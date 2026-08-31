//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name (`nm -D --defined-only`).

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::process::Command;

/// Symbols emitted by the toolchain/runtime rather than by the translated
/// source. These are not part of the API surface being verified.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__gmon_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Jv_")
        || matches!(
            name,
            "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "__TMC_END__"
        )
}

fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or " <type> <name>" for undefined-value syms
            let mut parts = line.split_whitespace().collect::<Vec<_>>();
            let name = parts.pop()?;
            let ty = parts.pop()?;
            // Only exported code/data, not local (lowercase) or undefined.
            if ty.len() != 1 || !ty.chars().next()?.is_ascii_uppercase() || ty == "U" {
                return None;
            }
            if is_toolchain_symbol(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(&c_lib_path());
    let rust_syms = exported_symbols(&rust_lib_path());

    assert!(
        c_syms.contains("driver") && c_syms.contains("run"),
        "sanity check failed; C exports were {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {rust_syms:?}"
    );
}
