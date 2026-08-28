//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-derives both symbol lists with `nm -D` at test time so SYMBOLS.md cannot
//! silently rot as the crate changes.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse `nm -D --defined-only` output into `(type, name)` pairs.
fn parse_defined(text: &str) -> Vec<(char, String)> {
    let mut v = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>" or "<type> <name>"
        let (ty, name) = match fields.len() {
            3 => (fields[1], fields[2]),
            2 => (fields[0], fields[1]),
            _ => continue,
        };
        let ty = match ty.chars().next() {
            Some(c) => c,
            None => continue,
        };
        v.push((ty, name.to_string()));
    }
    v
}

/// Symbols an external C consumer can actually link against: global (uppercase
/// nm type) definitions, excluding Rust name-mangled and runtime-internal ones.
fn public_c_abi_symbols(text: &str) -> BTreeSet<String> {
    parse_defined(text)
        .into_iter()
        .filter(|(ty, _)| ty.is_uppercase() && *ty != 'U')
        .map(|(_, n)| n)
        .filter(|n| {
            // Rust-mangled and runtime hooks are not part of the C ABI surface.
            !n.starts_with("_ZN")
                && !n.starts_with("_R")
                && !n.starts_with("__rust")
                && !n.starts_with("rust_")
                && !n.starts_with("_ITM_")
                && !n.starts_with("__cxa_")
                && !n.starts_with("__gmon_")
                && !n.starts_with("DW.ref.")
                && !n.starts_with("_GLOBAL_")
                && !n.starts_with("__do_global")
                && !n.starts_with("_fini")
                && !n.starts_with("_init")
                && !n.starts_with("__bss_start")
                && !n.starts_with("_edata")
                && !n.starts_with("_end")
                && !n.starts_with("__TMC_END__")
                && !n.starts_with("__dso_handle")
                && !n.starts_with("__odr_asan")
        })
        .collect()
}

/// The definitive check: every symbol the C `.so` exports must also be exported
/// by the Rust `.so`, with the exact same name.
#[test]
fn c_and_rust_export_identical_symbol_sets() {
    let c_path = c_so_path();
    let rust_path = rust_so_path();

    let c_text = nm(&["-D", "--defined-only"], &c_path);
    let rust_text = nm(&["-D", "--defined-only"], &rust_path);

    let c_syms = public_c_abi_symbols(&c_text);
    // For the Rust side, look at ALL defined dynamic symbols so a C symbol that
    // happens to look "internal" would still be found.
    let rust_all: BTreeSet<String> =
        parse_defined(&rust_text).into_iter().map(|(_, n)| n).collect();

    assert!(
        !c_syms.is_empty(),
        "no public symbols parsed from {} -- nm output was:\n{c_text}",
        c_path.display()
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_all.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C  = {}\nRust = {}\n\
         Fix by adding the #[no_mangle] extern \"C\" wrapper, or by translating \
         the C source that was skipped.",
        missing.len(),
        c_path.display(),
        rust_path.display()
    );

    // Pin the expected surface so an accidental export is noticed too.
    assert_eq!(
        c_syms.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["premultiply"],
        "the C ABI surface changed; update SYMBOLS.md and the test suite"
    );

    let rust_public = public_c_abi_symbols(&rust_text);
    assert_eq!(
        rust_public, c_syms,
        "Rust exports a different public C-ABI set than C.\n\
         only in Rust: {:?}\nonly in C: {:?}",
        rust_public.difference(&c_syms).collect::<Vec<_>>(),
        c_syms.difference(&rust_public).collect::<Vec<_>>()
    );
}

/// No unresolved non-libc symbols: `ldd -r` performs a full relocation check
/// and reports any symbol that cannot be bound.
#[test]
fn rust_so_has_no_unresolved_symbols() {
    for path in [rust_so_path(), c_so_path()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&path)
            .output()
            .expect("run ldd -r");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = combined
            .lines()
            .filter(|l| {
                let low = l.to_lowercase();
                low.contains("undefined symbol") || low.contains("not found")
            })
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved symbols:\n{}",
            path.display(),
            bad.join("\n")
        );
    }
}

/// Both libraries must depend only on the C runtime, so the Rust `.so` is a
/// drop-in replacement for the C one.
#[test]
fn rust_so_only_needs_the_c_runtime() {
    let out = Command::new("ldd").arg(rust_so_path()).output().expect("ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    let allowed = [
        "linux-vdso.so",
        "libgcc_s.so",
        "libc.so",
        "libm.so",
        "libdl.so",
        "libpthread.so",
        "ld-linux",
    ];
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let name = line.split_whitespace().next().unwrap_or("");
        assert!(
            allowed.iter().any(|a| name.contains(a)),
            "unexpected shared-library dependency {name:?} in:\n{text}"
        );
    }
}

/// `dlopen` of the Rust `.so` plus a successful `dlsym` of `premultiply` proves
/// the `#[no_mangle] extern "C"` export really is reachable by an external
/// consumer -- which is how every other test in this suite calls it.
#[test]
fn both_libraries_dlopen_and_dlsym_cleanly() {
    let l = common::libs();
    assert!(l.c_path.is_file(), "missing {}", l.c_path.display());
    assert!(l.rust_path.is_file(), "missing {}", l.rust_path.display());
    // Distinct function addresses -> we really loaded two implementations.
    assert_ne!(
        l.c as usize, l.rust as usize,
        "C and Rust resolved to the same address; only one library was loaded"
    );
}
