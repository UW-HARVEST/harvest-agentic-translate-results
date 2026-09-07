//! Phase D — symbol parity, enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. The diff must be empty.

mod common;

use common::{c_so_path, rust_so_path};

fn exported_symbols(so: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("run nm -u on {}: {e}", so.display()));
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    // The C .so must actually export something, or this test is vacuous.
    assert!(
        c.contains(&"stbds_hash_bytes".to_string()) && c.contains(&"siphash".to_string()),
        "C .so exports unexpected set: {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );

    // This library has exactly two public functions, so parity is also exact.
    assert_eq!(c, r, "symbol sets differ (extra Rust exports are also flagged)");
}

#[test]
fn d2_no_missing_non_libc_undefined_symbols() {
    // Every undefined symbol in the Rust .so must be satisfiable by libc /
    // libgcc / ld.so — i.e. none of the library's own code is missing.
    let allowed_prefixes = [
        "_Unwind_", "__cxa_", "__gmon_", "_ITM_", "__tls_get_addr", "__errno_location",
        "pthread_", "gettid", "statx", "syscall",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "printf", "puts", "read", "readlink", "realloc",
        "realpath", "stat64", "strlen", "write", "writev", "fflush", "putchar", "fwrite",
        "sprintf", "snprintf", "exit", "_exit", "qsort", "memcmp",
    ];

    let unresolved: Vec<String> = undefined_symbols(&rust_so_path())
        .into_iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !allowed_prefixes.iter().any(|p| base.starts_with(p))
                && !allowed_exact.contains(&base)
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so has non-libc undefined symbols (untranslated code?): {unresolved:?}"
    );
}

#[test]
fn d3_static_c_helper_is_not_exported() {
    // `stbds_siphash_bytes` is `static` in the C, so neither .so may export it.
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    assert!(
        !c.contains(&"stbds_siphash_bytes".to_string()),
        "C .so unexpectedly exports the static helper"
    );
    assert!(
        !r.contains(&"stbds_siphash_bytes".to_string()),
        "Rust .so exports `stbds_siphash_bytes`, which is `static` in the C"
    );
}
