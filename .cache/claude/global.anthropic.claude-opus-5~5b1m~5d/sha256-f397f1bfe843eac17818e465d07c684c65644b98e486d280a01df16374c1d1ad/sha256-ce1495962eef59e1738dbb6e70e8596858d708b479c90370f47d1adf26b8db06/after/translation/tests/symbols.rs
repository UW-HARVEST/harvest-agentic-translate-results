//! Phase D — symbol parity between the C and the Rust shared library.
//!
//! Asserts mechanically (via `nm -D`) that every symbol the C `.so` exports is
//! also exported by the Rust `.so` under the exact same name, and that the Rust
//! `.so` has no undefined non-libc dependencies.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Toolchain/compiler bookkeeping symbols that are not part of the library API.
fn is_toolchain(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Unwind_")
        || name.starts_with("__gmon_")
        || name == "_init"
        || name == "_fini"
        || name == "__bss_start"
        || name == "_edata"
        || name == "_end"
        || name == "__tls_get_addr"
        || name == "__errno_location"
}

fn nm(path: &std::path::Path, args: &[&str]) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("failed to run nm — is binutils installed?");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                // "<addr> <type> <name>"
                3 => Some((f[1].to_string(), f[2].to_string())),
                // "         <type> <name>"  (undefined / weak)
                2 => Some((f[0].to_string(), f[1].to_string())),
                _ => None,
            }
        })
        .map(|(t, n)| (t, n.split('@').next().unwrap().to_string()))
        .collect()
}

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .into_iter()
        .filter(|(t, n)| t != "w" && !is_toolchain(n))
        .map(|(_, n)| n)
        .collect()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_lib_path());
    let r = exported(&rust_lib_path());

    assert!(
        !c.is_empty(),
        "no exported symbols found in the C library — build it first"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:#?}\n\
         C exports  : {c:#?}\nRust exports: {r:#?}",
        missing.len()
    );

    // The known API surface, spelled out so an accidental rename is loud.
    assert!(
        c.contains("decode_base64"),
        "C .so must export decode_base64, got {c:#?}"
    );
    assert!(
        r.contains("decode_base64"),
        "Rust .so must export decode_base64, got {r:#?}"
    );

    println!("C exports {} symbol(s); all present in Rust: {c:?}", c.len());
}

#[test]
fn d02_rust_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol in the Rust .so must be satisfiable from the
    // platform C/unwind runtime — i.e. no dangling references to code that was
    // never translated.
    let undef = nm(&rust_lib_path(), &["-D", "-u"]);
    let mut offenders = Vec::new();
    for (t, n) in undef {
        if t == "w" || is_toolchain(&n) {
            continue;
        }
        // Resolvable from libc/libm/libgcc/ld.so?
        let known = [
            "calloc", "malloc", "realloc", "free", "posix_memalign", "strlen", "memcpy", "memmove",
            "memset", "bcmp", "memcmp", "abort", "getenv", "getcwd", "readlink", "realpath",
            "open64", "close", "read", "write", "writev", "lseek64", "fstat64", "stat64", "statx",
            "mmap64", "munmap", "syscall", "dl_iterate_phdr", "gettid", "pthread_key_create",
            "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
            "pthread_mutex_lock", "pthread_mutex_unlock", "sysconf", "dlsym", "dladdr", "sigaction",
            "sigaltstack", "mprotect", "getpid", "nanosleep", "clock_gettime", "poll", "pipe2",
            "environ", "__libc_start_main", "__tunable_get_val", "strerror_r", "malloc_usable_size",
        ];
        if !known.contains(&n.as_str()) {
            offenders.push(n);
        }
    }
    assert!(
        offenders.is_empty(),
        "Rust .so has undefined symbols that are not part of the platform runtime \
         (a sign of untranslated code): {offenders:#?}"
    );
}

#[test]
fn d04_libraries_under_test_match_the_current_profile() {
    let c = c_lib_path();
    let r = rust_lib_path();
    println!("C    .so under test: {}", c.display());
    println!("Rust .so under test: {}", r.display());

    if std::env::var("DRIVER_RUST_SO").is_ok() {
        return; // explicitly overridden (the LD_PRELOAD child does this)
    }
    // Guard against a debug run silently exercising the release artifact:
    // the cdylib must live in the same profile directory as this test binary.
    let expected_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap() // deps/
        .parent()
        .unwrap() // <target>/<profile>
        .to_path_buf();
    assert_eq!(
        r.parent().unwrap(),
        expected_dir.as_path(),
        "the Rust .so under test ({}) is not the one built for this profile ({})",
        r.display(),
        expected_dir.display()
    );
}

#[test]
fn d03_rust_exports_nothing_extra_in_the_c_namespace() {
    // Not a hard requirement, but a renamed/duplicated export would show up
    // here. Informational: list Rust exports that the C does not have.
    let c = exported(&c_lib_path());
    let r = exported(&rust_lib_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    println!("Rust-only exported symbols (informational): {extra:#?}");
    // The Rust cdylib legitimately exports the Rust allocator shims and
    // `rust_eh_personality`; assert none of them collide with a C API name.
    for e in extra {
        assert!(
            !e.starts_with("decode") && !e.starts_with("is_base64"),
            "unexpected Rust export shadowing the C API: {e}"
        );
    }
}
