//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `cdylib` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut syms = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // "<addr> <type> <name>" for defined symbols.
        let mut fields = line.split_whitespace();
        let (Some(_addr), Some(kind), Some(name)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // Exported code/data: text, data, bss, rodata, weak, indirect.
        if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "i" | "I") {
            continue;
        }
        if name.starts_with("_") || name.contains("@") {
            // Toolchain/glibc plumbing (_init, _fini, _edata, ...).
            continue;
        }
        syms.insert(name.to_string());
    }
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let (c, rust) = common::both();

    let c_syms = dynamic_symbols(&c.path);
    let rust_syms = dynamic_symbols(&rust.path);

    assert!(
        !c_syms.is_empty(),
        "no symbols parsed from {}",
        c.path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}\nC symbols: {:?}",
        rust.path.display(),
        c.path.display(),
        missing,
        c_syms
    );
}

#[test]
fn documented_public_api_is_exported() {
    // From c_src/include/lib.h plus the file-scope functions in src/lib.c.
    let expected = [
        "convert_double_to_int",
        "find_value_in_buffer",
        "process_negation",
        "create_numeric_buffer",
        "calculate_with_doubles",
        "doubleneg",
    ];
    let (c, rust) = common::both();
    let c_syms = dynamic_symbols(&c.path);
    let rust_syms = dynamic_symbols(&rust.path);
    for name in expected {
        assert!(c_syms.contains(name), "C .so lacks {name}");
        assert!(rust_syms.contains(name), "Rust .so lacks {name}");
    }
}
