//! Phase D — symbol parity, enforced mechanically rather than by hand.
//!
//! Shells out to `nm -D` on both `.so`s and requires that the set of dynamic
//! symbols the C library *defines* is a subset of what the Rust library
//! defines, and that every one of them is actually resolvable via `dlsym`
//! with the exact same name.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols glibc/ld.so/libgcc inject into every shared object; not part of the
/// library's own API surface.
const TOOLCHAIN_NOISE: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "__cxa_finalize",
    "__gmon_start__",
    "_ITM_registerTMCloneTable",
    "_ITM_deregisterTMCloneTable",
];

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm -D (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep only strong, globally visible definitions.
            if !matches!(kind, "T" | "D" | "B" | "R" | "G" | "S") {
                return None;
            }
            if TOOLCHAIN_NOISE.contains(&name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Symbols that are `t`/local in the C `.so` (i.e. `static` functions) and
/// therefore must NOT be exported by the Rust `.so` either.
fn nm_local_text(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            if kind == "t" && !name.starts_with('_') && !name.contains('.') {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_syms = nm_defined(&l.c_path);
    let rust_syms = nm_defined(&l.rust_path);

    assert!(
        !c_syms.is_empty(),
        "nm found no exported symbols in {:?} — bad build?",
        l.c_path
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {:?}",
        missing.len(),
        c_syms.len(),
        rust_syms.len(),
        rust_syms.intersection(&c_syms).collect::<Vec<_>>(),
    );

    // The five documented entry points, spelled out, so a future refactor that
    // silently renames one cannot pass by accident.
    for expected in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert!(c_syms.contains(expected), "C .so lost {expected}");
        assert!(rust_syms.contains(expected), "Rust .so lost {expected}");
    }
}

#[test]
fn d02_rust_does_not_export_c_static_helpers() {
    let l = libs();
    let c_locals = nm_local_text(&l.c_path);
    // Sanity: the two `static` helpers really are local in the C build.
    for helper in ["goodG2B", "goodB2G"] {
        assert!(
            c_locals.contains(helper),
            "expected {helper} to be a local (static) symbol in the C .so; found locals: {c_locals:?}"
        );
    }
    let rust_syms = nm_defined(&l.rust_path);
    for helper in ["goodG2B", "goodB2G"] {
        assert!(
            !rust_syms.contains(helper),
            "Rust .so exports {helper}, but it is `static` in the C source — \
             the ABI surface must not be larger than C's"
        );
    }
}

#[test]
fn d03_every_c_symbol_resolves_via_dlsym_in_rust() {
    // Stronger than `nm`: the symbol must actually be callable through the
    // dynamic linker under its exact C name.
    let l = libs();
    for name in nm_defined(&l.c_path) {
        let cname = format!("{name}\0");
        let in_c = unsafe {
            l.c.get::<unsafe extern "C" fn()>(cname.as_bytes())
                .is_ok()
        };
        let in_rust = unsafe {
            l.rust
                .get::<unsafe extern "C" fn()>(cname.as_bytes())
                .is_ok()
        };
        assert!(in_c, "dlsym({name}) failed on the C .so");
        assert!(
            in_rust,
            "dlsym({name}) failed on the Rust .so although nm listed it"
        );
    }
}

#[test]
fn d04_no_non_libc_undefined_symbols_in_rust() {
    let l = libs();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&l.rust_path)
        .output()
        .expect("run nm -D --undefined-only");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Everything the Rust .so imports must come from libc / libgcc-unwind /
    // ld.so. Anything else would mean an unresolvable dependency at load time.
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_", "pthread_"];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "puts", "read", "readlink", "realloc", "realpath", "stat", "stat64", "statx",
        "strlen", "syscall", "write", "writev", "sysconf", "fwrite", "fputs", "fflush",
        "poll", "pipe2", "sigaction", "sigaltstack", "signal", "mprotect", "getrandom",
    ];
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_prefixes.iter().any(|p| s.starts_with(p))
                && !allowed_exact.contains(&s.as_str())
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has non-libc undefined symbols: {bad:?}"
    );

    // And the crucial one: output really goes through the C runtime, exactly
    // like the C library does.
    assert!(
        undef.iter().any(|s| s == "printf") || undef.iter().any(|s| s == "puts"),
        "Rust .so does not import printf/puts — output would not be byte-identical \
         to the C library's buffering. Imports: {undef:?}"
    );
}

#[test]
fn d05_no_extra_public_symbols_derived_from_driver_c() {
    // The Rust .so legitimately exports Rust runtime symbols (rust_eh_*, etc.),
    // but must not export any *additional* lowercase C-style name that the C
    // .so does not have, since that would be a wider ABI than the original.
    let l = libs();
    let c_syms = nm_defined(&l.c_path);
    let rust_syms = nm_defined(&l.rust_path);
    let suspicious: Vec<&String> = rust_syms
        .difference(&c_syms)
        .filter(|s| {
            !s.starts_with('_')
                && !s.starts_with("rust_")
                && !s.contains("::")
                && !s.starts_with("DW.")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so exports C-looking symbols absent from the C .so: {suspicious:?}"
    );
}

#[test]
fn d06_project_builds_no_binary_target() {
    // Documents the "if the project builds a binary, compare stdout" gate:
    // it does not. `Cargo.toml` declares only `crate-type = ["cdylib"]`, there
    // is no `src/main.rs` and no `[[bin]]`; `c_src/CMakeLists.txt` declares
    // only `add_library(driver SHARED src/driver.c)` and no `add_executable`.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!root.join("src/main.rs").exists(), "unexpected src/main.rs");
    assert!(!root.join("src/bin").exists(), "unexpected src/bin/");
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!cargo.contains("[[bin]]"), "unexpected [[bin]] target");
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; add a stdout comparison for it"
    );
}
