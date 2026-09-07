//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Executable version of `SYMBOLS.md`: the set of dynamic symbols the C library
//! DEFINES must be a subset of what the Rust library defines, with the exact
//! same names, and the Rust library must not import any non-libc symbol.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Names that are part of the language runtime / linker plumbing rather than
/// the library's own ABI, so they are excluded from the parity comparison.
fn is_plumbing(s: &str) -> bool {
    s.starts_with("__")
        || s.starts_with("_ITM_")
        || s.starts_with("_Unwind")
        || s.starts_with("_init")
        || s.starts_with("_fini")
        || s == "_edata"
        || s == "_end"
        || s == "_DYNAMIC"
        || s == "_GLOBAL_OFFSET_TABLE_"
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let cpath = c_so_path();
    let rpath = rust_so_path();

    let cdef: BTreeSet<String> = nm(&cpath, "--defined-only")
        .into_iter()
        .filter(|s| !is_plumbing(s))
        .collect();
    let rdef: BTreeSet<String> = nm(&rpath, "--defined-only")
        .into_iter()
        .filter(|s| !is_plumbing(s))
        .collect();

    let missing: Vec<&String> = cdef.difference(&rdef).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}\n\
         C defines:    {:?}\n\
         Rust defines: {:?}",
        rpath.display(),
        missing.len(),
        cpath.display(),
        missing,
        cdef,
        rdef
    );

    // The documented surface must be present on both sides.
    for want in EXPECTED_SYMBOLS {
        assert!(cdef.contains(*want), "C .so missing documented `{want}`");
        assert!(rdef.contains(*want), "Rust .so missing documented `{want}`");
    }

    // And the C surface must be exactly the documented ten (guards against
    // SYMBOLS.md drifting away from the source).
    let documented: BTreeSet<String> = EXPECTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        cdef, documented,
        "the C .so's exported surface changed; update SYMBOLS.md"
    );
}

/// Every undefined symbol in the Rust `.so` must be satisfiable from the
/// system libraries it declares (`ldd -r` reports unresolved ones), i.e. there
/// are 0 dangling non-libc imports. The Rust std runtime legitimately imports
/// a wider libc surface than the C library does (`mmap64`, `pthread_key_*`,
/// ...), so the check is "resolvable", not a hand-written allow-list.
#[test]
fn phase_d_rust_has_no_unresolved_imports() {
    for path in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&path)
            .output()
            .unwrap_or_else(|e| panic!("cannot run ldd on {}: {e}", path.display()));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved imports:\n{}",
            path.display(),
            bad.join("\n")
        );
    }
}

/// The Rust `.so` must import every libc function the C `.so` imports that the
/// translation actually relies on for byte-identical behaviour (`printf` for
/// formatting, `malloc`/`free` for caller-`free`-able buffers, `memchr` for the
/// search, `strlen`/`strcpy` for the copy).
#[test]
fn phase_d_rust_reuses_the_same_libc_primitives() {
    let rundef: BTreeSet<String> = nm(&rust_so_path(), "--undefined-only")
        .into_iter()
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .collect();
    for f in ["printf", "malloc", "free", "memchr", "strlen", "strcpy"] {
        assert!(
            rundef.contains(f),
            "Rust .so does not import libc `{f}`; formatting/heap semantics \
             could then differ from the C. Imports: {rundef:?}"
        );
    }
}
