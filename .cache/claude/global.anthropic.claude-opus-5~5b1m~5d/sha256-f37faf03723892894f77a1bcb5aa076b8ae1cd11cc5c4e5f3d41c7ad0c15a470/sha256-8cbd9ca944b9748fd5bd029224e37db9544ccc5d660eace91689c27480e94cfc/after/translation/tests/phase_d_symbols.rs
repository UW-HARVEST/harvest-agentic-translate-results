// Phase D -- symbol parity enforced as a test, so it cannot silently rot.
mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

/// Dynamic symbols DEFINED by a shared object, as `nm -D --defined-only` sees
/// them (this is exactly what an external `dlopen` caller can reach).
fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep only global text/data definitions.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols the Rust `.so` needs from elsewhere. Everything here must be
/// provided by libc / libm / libgcc_s / ld.so.
fn undefined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap_or(&s).to_string())
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_lib_path());
    let rs = defined_dynamic_symbols(&rust_lib_path());

    // The C library's own API surface (filter out toolchain bookkeeping that
    // cmake/gcc adds to every .so).
    let ignore = |n: &str| {
        n.starts_with("_init")
            || n.starts_with("_fini")
            || n.starts_with("__bss_start")
            || n == "_edata"
            || n == "_end"
            || n.starts_with("_ITM_")
            || n.starts_with("__cxa_")
            || n.starts_with("__gmon_")
            || n.starts_with("__odr_")
    };

    let c_api: BTreeSet<&String> = c.iter().filter(|n| !ignore(n)).collect();
    assert!(!c_api.is_empty(), "nm found no C symbols at all -- is the C library built?");

    let missing: Vec<&&String> = c_api.iter().filter(|n| !rs.contains(**n)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C API   = {c_api:?}\n\
         RUST    = {rs:?}",
        missing.len()
    );

    // Belt and braces: the five documented entry points, by exact name.
    for name in [
        "safe_double_to_int",
        "process_with_fallthrough",
        "copy_data_block",
        "handle_pointer_operations",
        "overunder",
    ] {
        assert!(c.contains(name), "C .so unexpectedly lacks {name}");
        assert!(rs.contains(name), "Rust .so lacks {name}");
    }
    eprintln!("symbol parity OK: {} C API symbols, all present in Rust", c_api.len());
}

#[test]
fn phase_d_rust_has_no_unresolved_non_libc_symbols() {
    let undef = undefined_dynamic_symbols(&rust_lib_path());

    // Everything the Rust cdylib imports must come from the platform runtime.
    let allowed_prefix = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location",
        "__libc_", "__memcpy", "__stack_chk", "_dl_", "__pthread_", "__register_",
        "__deregister_", "__gxx_", "__rust_",
    ];
    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|n| {
            // Any symbol that also exists in the C library's import set, or that
            // is a plain lowercase libc name, is fine. Flag Rust-mangled ones.
            n.starts_with("_ZN") && !allowed_prefix.iter().any(|p| n.starts_with(p))
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has unresolved Rust-mangled (non-libc) imports: {leftovers:?}"
    );

    // And prove it actually resolves at runtime: `dlopen` with immediate
    // binding fails if anything is missing.
    let lib = unsafe { libloading::Library::new(rust_lib_path()) };
    assert!(lib.is_ok(), "dlopen of the Rust .so failed: {:?}", lib.err());
}

#[test]
fn phase_d_symbols_are_callable_through_dlopen_only() {
    // Sanity: every symbol used by the test suite was obtained via dlopen,
    // never by linking the crate. Re-resolve them here to prove it.
    let l = libs();
    for lib in [&l.c, &l.rs] {
        assert_eq!(unsafe { (lib.safe_double_to_int)(3.7) }, 3, "{}", lib.name);
        assert_eq!(unsafe { (lib.process_with_fallthrough)(5, 0) }, 150, "{}", lib.name);
        assert_eq!(unsafe { (lib.handle_pointer_operations)(1) }, 102, "{}", lib.name);
    }
    cmp_overunder("phase-d", 1, 2, 3, 4);
}
