// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Re-runs `nm -D` on both objects and asserts:
//   * every symbol the C object exports is exported by the Rust object with
//     the exact same name (the diff must be EMPTY);
//   * the C `static` internals (`multi_stage`, `y`) leak from neither;
//   * every symbol is actually resolvable through `dlsym`.

use crate::common::*;
use crate::Case;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("`nm` must be available on PATH");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep only globally-visible code/data definitions.
            match kind {
                "T" | "t" | "D" | "B" | "R" | "W" | "V" | "i" => Some(name.to_string()),
                _ => None,
            }
        })
        // Drop compiler/CRT bookkeeping symbols that are not API.
        .filter(|n| {
            !matches!(
                n.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .collect()
}

fn nm_undefined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("`nm` must be available on PATH");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

fn d1_every_c_symbol_is_exported_by_rust() {
    let c_syms = nm_defined(&c_so_path());
    let r_syms = nm_defined(&rust_so_path());

    assert!(
        c_syms.contains("driver"),
        "sanity: the C object must export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c_syms:?}\n\
         Rust: {r_syms:?}"
    );
}

fn d2_static_internals_are_not_exported() {
    for so in [c_so_path(), rust_so_path()] {
        let syms = nm_defined(&so);
        for internal in ["multi_stage", "y"] {
            assert!(
                !syms.contains(internal),
                "`{internal}` has internal linkage in the C source and must not \
                 be exported by {}",
                so.display()
            );
        }
    }
}

fn d3_no_dangling_non_libc_undefined_symbols_in_rust() {
    // Everything the Rust object imports must be a libc / libgcc runtime
    // symbol; nothing may reference an untranslated C module.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location", "statx",
        "gettid", "syscall",
    ];
    let libc_names: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "printf", "pthread_getspecific",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific", "puts", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "strlen", "write", "writev", "memrchr",
        "sysconf", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "poll", "sigaction",
        "sigaltstack", "mprotect", "getrandom", "__libc_start_main", "qsort", "fwrite", "fputs",
        "fflush", "exit", "strerror_r", "dlsym", "dladdr",
    ]
    .into_iter()
    .collect();

    let undef = nm_undefined(&rust_so_path());
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !libc_names.contains(base) && !allowed_prefixes.iter().any(|p| base.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbols (would indicate an untranslated C \
         module): {unexpected:?}"
    );
}

fn d4_symbols_resolve_through_dlsym() {
    // Not just present in the symbol table: actually callable via dlopen/dlsym
    // in both objects (this is what an external consumer does).
    for lib in [c_lib(), rust_lib()] {
        // SAFETY: signature matches `void driver(int, int, int)`.
        let f: libloading::Symbol<DriverFn> =
            unsafe { lib.get(b"driver\0") }.expect("`driver` must resolve via dlsym");
        let out = capture_stdout(|| unsafe { f(1, 2, 3) });
        assert_eq!(out, b"Ok!\nResult: 0\n");
    }
}

/// Registry of this module's cases, in execution order.
pub fn cases() -> Vec<Case> {
    vec![
        ("d1_every_c_symbol_is_exported_by_rust", d1_every_c_symbol_is_exported_by_rust as fn()),
        ("d2_static_internals_are_not_exported", d2_static_internals_are_not_exported as fn()),
        ("d3_no_dangling_non_libc_undefined_symbols_in_rust", d3_no_dangling_non_libc_undefined_symbols_in_rust as fn()),
        ("d4_symbols_resolve_through_dlsym", d4_symbols_resolve_through_dlsym as fn()),
    ]
}
