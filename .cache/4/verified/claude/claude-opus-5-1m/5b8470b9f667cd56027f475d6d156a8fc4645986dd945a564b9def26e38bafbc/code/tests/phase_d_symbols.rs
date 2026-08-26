//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that every symbol the C library exports
//! is also exported by the Rust library under the exact same name, so the
//! `SYMBOLS.md` diff cannot silently rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D` symbol names, filtered by nm's class letter.
fn nm_symbols(so: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn rust_so_exports_every_symbol_the_c_so_exports() {
    let b = both();
    let c_defined = nm_symbols(&b.c.path, &["--defined-only"]);
    let rust_defined = nm_symbols(&b.rust.path, &["--defined-only"]);

    println!("C   exports ({}): {c_defined:?}", c_defined.len());
    println!("Rust exports ({}): {rust_defined:?}", rust_defined.len());

    let missing: Vec<&String> = c_defined.difference(&rust_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust library is missing {} symbol(s) that the C library exports: {missing:?}\n\
         Each one is either an un-exported implementation (add the \
         #[unsafe(no_mangle)] extern \"C\" wrapper) or an untranslated C source file.",
        missing.len()
    );

    // The C library's complete public ABI is `driver`; nothing more, nothing less.
    assert!(c_defined.contains("driver"), "the C library does not export `driver`");
    assert!(rust_defined.contains("driver"), "the Rust library does not export `driver`");

    let extra: Vec<&String> = rust_defined.difference(&c_defined).collect();
    assert!(
        extra.is_empty(),
        "the Rust library exports symbols the C library does not: {extra:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let b = both();
    let undefined = nm_symbols(&b.rust.path, &["--undefined-only"]);
    println!("Rust imports ({}): {undefined:?}", undefined.len());

    // Everything the Rust `.so` imports must come from libc / the unwinder /
    // the toolchain's weak CRT hooks — never from an untranslated module.
    const WEAK_TOOLCHAIN: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "gettid",
        "statx",
    ];

    let mut suspicious = Vec::new();
    for sym in &undefined {
        let versioned = sym.contains("@GLIBC") || sym.contains("@GCC") || sym.contains("@GLIBCXX");
        let weak_known = WEAK_TOOLCHAIN.iter().any(|w| sym.starts_with(w));
        if !versioned && !weak_known {
            suspicious.push(sym.clone());
        }
    }
    assert!(
        suspicious.is_empty(),
        "the Rust library imports symbols that are neither libc nor toolchain \
         weak hooks — these would be untranslated pieces of the library: {suspicious:?}"
    );

    // `dlopen` already succeeded (the harness loaded it), which proves every
    // import actually resolves at load time.
    assert!(b.rust.lookup(b"driver\0"), "dlsym(driver) failed on the Rust library");
}

#[test]
fn the_c_library_imports_only_the_libc_pieces_the_translation_reproduces() {
    // Documents the one asymmetry recorded in SYMBOLS.md: the C build (no -O,
    // so no __OPTIMIZE__) calls glibc's out-of-line `tolower`/`toupper`, whereas
    // the Rust translation indexes `__ctype_tolower_loc`/`__ctype_toupper_loc`
    // directly — the definitions glibc itself gives those functions.  The
    // behavioural equivalence is what Phase B asserts for all 256 inputs under
    // every locale; this test just pins the expectation in place.
    let b = both();
    let c_imports = nm_symbols(&b.c.path, &["--undefined-only"]);
    let rust_imports = nm_symbols(&b.rust.path, &["--undefined-only"]);

    let names = |set: &BTreeSet<String>| -> BTreeSet<String> {
        set.iter().map(|s| s.split('@').next().unwrap_or(s).to_string()).collect()
    };
    let c_names = names(&c_imports);
    let rust_names = names(&rust_imports);

    // `printf` may appear as glibc's fortified `__printf_chk` when the C side is
    // compiled with `-D_FORTIFY_SOURCE`; both are the same libc formatter.
    assert!(
        c_names.contains("printf") || c_names.contains("__printf_chk"),
        "C library no longer imports printf: {c_names:?}"
    );
    assert!(
        rust_names.contains("printf") || rust_names.contains("__printf_chk"),
        "the Rust library must use libc's printf, like the C code does"
    );
    for required in ["setlocale", "__ctype_b_loc"] {
        assert!(c_names.contains(required), "C library no longer imports {required}");
        assert!(
            rust_names.contains(required),
            "the Rust library must use libc's {required}, like the C code does"
        );
    }
    if c_names.contains("tolower") {
        assert!(
            rust_names.contains("__ctype_tolower_loc"),
            "the C library calls glibc's tolower(); the Rust translation must consult \
             the same table via __ctype_tolower_loc"
        );
    }
    if c_names.contains("toupper") {
        assert!(
            rust_names.contains("__ctype_toupper_loc"),
            "the C library calls glibc's toupper(); the Rust translation must consult \
             the same table via __ctype_toupper_loc"
        );
    }
}
