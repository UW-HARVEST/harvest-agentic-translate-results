//! Phase A / Phase D — every symbol exported by the C `.so` must also be
//! exported by the Rust `.so`, with the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const TAG: &str = "symbols";

fn nm(args: &[&str], lib: &Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(lib)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {lib:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Global (upper-case type letter) defined symbols, ignoring the weak
/// toolchain hooks that both toolchains emit for their own bookkeeping.
fn exported(lib: &Path) -> BTreeSet<String> {
    let text = nm(&["-D", "--defined-only"], lib);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        let (ty, name) = (cols[cols.len() - 2], cols[cols.len() - 1]);
        // Skip weak/local and compiler bookkeeping symbols.
        if ty.len() != 1 || !ty.chars().next().unwrap().is_ascii_uppercase() {
            continue;
        }
        if matches!(ty, "W" | "V" | "U") {
            continue;
        }
        if name.starts_with("_ITM_")
            || name.starts_with("__gmon_start__")
            || name.starts_with("_init")
            || name.starts_with("_fini")
            || name.starts_with("__bss_start")
            || name == "_edata"
            || name == "_end"
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported(&common::c_so(TAG));
    let r = exported(&common::rust_so());

    assert_eq!(
        c,
        BTreeSet::from(["main".to_string(), "static_sum".to_string()]),
        "unexpected C export set (SYMBOLS.md is stale)"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_symbols() {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(common::rust_so())
        .output()
        .expect("ldd must be available");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{text}"
    );
}

#[test]
fn both_libraries_resolve_the_documented_entry_points() {
    // dlsym must succeed for every documented symbol in *both* libraries.
    let pair = common::fresh_pair(TAG);
    for name in [&b"static_sum\0"[..], &b"main\0"[..]] {
        let pretty = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
        assert!(pair.c.has_symbol(name), "C .so lacks {pretty}");
        assert!(pair.rust.has_symbol(name), "Rust .so lacks {pretty}");
    }
    // And the resolved `static_sum` really is callable in both.
    assert_eq!(pair.c.static_sum(0), pair.rust.static_sum(0));
    assert_eq!(pair.c.static_sum(3), pair.rust.static_sum(3));
}
