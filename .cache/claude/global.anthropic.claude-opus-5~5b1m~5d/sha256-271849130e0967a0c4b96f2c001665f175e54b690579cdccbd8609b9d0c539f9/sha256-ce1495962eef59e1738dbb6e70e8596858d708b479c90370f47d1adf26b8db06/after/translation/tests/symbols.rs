// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Re-derives both symbol sets with `nm -D` at test time (rather than trusting
// the snapshot in SYMBOLS.md) and requires the C-minus-Rust difference to be
// empty. Also checks that every symbol is genuinely callable through `dlsym`
// and that the Rust object imports nothing outside libc.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm` on {path:?}: {e}"));
    assert!(
        out.status.success(),
        "`nm {args:?} {path:?}` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         U <name>"
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let b = it.next();
            match b {
                Some(name) if a == "U" || a == "w" || a == "v" => Some(name.to_string()),
                Some(_ty) => it.next().map(|n| n.to_string()),
                None => None,
            }
        })
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
}

/// `nm -D` prints versioned imports as `malloc@GLIBC_2.2.5`; the bare name is
/// what `dlsym` takes.
fn bare(name: &str) -> &str {
    name.split('@').next().unwrap_or(name)
}

/// Symbols supplied by the platform C runtime / unwinder / dynamic loader
/// rather than by the translated library.
fn is_platform_runtime(name: &str) -> bool {
    let n = bare(name);
    n.starts_with("_Unwind_")
        || n.starts_with("__cxa_")
        || n.starts_with("_ITM_")
        || n.starts_with("__tls_get_addr")
        || n == "__gmon_start__"
        || n == "__errno_location"
        || n == "dl_iterate_phdr"
}

/// Rust `cdylib`s always carry these ELF/compiler-runtime housekeeping symbols;
/// they are not part of the translated API and the C object has no equivalent.
fn is_toolchain_noise(name: &str) -> bool {
    name.starts_with("_ZN")           // mangled Rust internals
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_R")     // v0 Rust mangling
        || matches!(
            name,
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_ITM_registerTMCloneTable"
                | "_ITM_deregisterTMCloneTable" | "__gmon_start__" | "__cxa_finalize"
        )
}

#[test]
fn c_and_rust_export_identical_symbol_sets() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();

    let c_syms = defined(&c_path);
    let r_syms = defined(&r_path);

    println!("C   .so {c_path:?}\n  defined: {c_syms:?}");
    println!("Rust.so {r_path:?}\n  defined: {r_syms:?}");

    // ---- the gate: every C symbol must exist in Rust, exact name ----
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
         translate the missing C source.",
        missing.len(),
        missing
    );

    // The C object must not have shrunk either (guards against a stale build).
    let c_expected: BTreeSet<String> = [
        "safe_double_to_int",
        "process_array_reverse",
        "switch_fallthrough_calculator",
        "allocate_and_compute",
        "foreach_sum",
        "fallcalc",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_syms, c_expected,
        "the C .so's exported set changed; SYMBOLS.md needs regenerating"
    );

    // Report (without failing) anything Rust exports beyond the C surface, so
    // accidental extra public API is visible.
    let extra: Vec<&String> = r_syms
        .difference(&c_syms)
        .filter(|n| !is_toolchain_noise(n))
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports non-toolchain symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r_path = common::rust_so_path();
    let u = undefined(&r_path);
    println!("Rust.so undefined: {u:?}");

    // Everything the Rust object imports must be resolvable from the process's
    // global namespace (libc / libgcc_s / ld.so), i.e. it must be a platform
    // symbol and not an untranslated piece of the library itself.
    let this = libloading::os::unix::Library::this();
    let unresolved: Vec<&String> = u
        .iter()
        .filter(|n| !is_toolchain_noise(n) && !is_platform_runtime(n))
        .filter(|n| {
            unsafe { this.get::<*const ()>(format!("{}\0", bare(n)).as_bytes()) }.is_err()
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc undefined symbols: {unresolved:?}"
    );

    // None of the six API symbols may be *imported* -- each must be defined here.
    let api = [
        "safe_double_to_int",
        "process_array_reverse",
        "switch_fallthrough_calculator",
        "allocate_and_compute",
        "foreach_sum",
        "fallcalc",
    ];
    for name in api {
        assert!(
            !u.iter().any(|n| bare(n) == name),
            "Rust .so IMPORTS `{name}` instead of defining it"
        );
    }

    // It must import malloc/free from libc (that is what makes the
    // allocation-failure behaviour match the C bit-for-bit).
    for want in ["malloc", "free"] {
        assert!(
            u.iter().any(|n| bare(n) == want),
            "expected the Rust .so to import `{want}` from libc, imports: {u:?}"
        );
    }
}

#[test]
fn every_c_symbol_is_dlsym_callable_in_both() {
    // `common::both()` resolves all six symbols in both objects and panics with
    // a precise message if any is absent, so simply loading proves callability.
    let (c, r) = common::both();

    // Touch each function pointer once so none is dead-code-eliminated and each
    // is proven to actually execute across the FFI boundary.
    let mut buf = [1i32, 2, 3];
    for api in [&c, &r] {
        let _ = unsafe { (api.safe_double_to_int)(1.25) };
        let _ = unsafe { (api.process_array_reverse)(buf.as_mut_ptr().wrapping_add(2), 3) };
        let _ = unsafe { (api.switch_fallthrough_calculator)(5, 2) };
        let _ = unsafe { (api.allocate_and_compute)(4, 1.5) };
        let _ = unsafe { (api.foreach_sum)(buf.as_mut_ptr(), 3) };
        let _ = unsafe { (api.fallcalc)(1, 2, 3, 4) };
        println!("all six symbols executed in the {} .so", api.which);
    }
}

#[test]
fn no_stub_or_unimplemented_in_translation() {
    // Phase A forbids faking a symbol just to make it appear in `nm -D`.
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs"),
    )
    .expect("read src/lib.rs");
    for needle in ["unimplemented!", "todo!", "unreachable!(\"stub", "panic!(\"stub"] {
        assert!(
            !src.contains(needle),
            "src/lib.rs contains `{needle}` -- a stub that lies about behaviour \
             is worse than a missing symbol"
        );
    }
}
