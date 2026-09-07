//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Symbols that belong to the loader / libc / Rust runtime rather than to
/// `c_src/src/driver.c`, and are therefore not part of the parity requirement.
fn is_boilerplate(name: &str) -> bool {
    matches!(
        name,
        "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "__libc_csu_init"
            | "__libc_csu_fini"
    ) || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
}

fn dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run `nm` — is binutils installed?");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .filter(|s| !is_boilerplate(s))
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();

    let c_syms = dynamic_symbols(&c_path);
    let r_syms = dynamic_symbols(&r_path);

    eprintln!("C   .so {} -> {:?}", c_path.display(), c_syms);
    eprintln!("Rust.so {} -> {:?}", r_path.display(), r_syms);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // The C source defines exactly these five externally-linked functions;
    // `goodG2B` / `goodB2G` are `static` and must not appear in either table.
    for expected in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert!(c_syms.contains(expected), "C .so lost {expected}");
        assert!(r_syms.contains(expected), "Rust .so lost {expected}");
    }
    for internal in ["goodG2B", "goodB2G"] {
        assert!(
            !c_syms.contains(internal),
            "{internal} is static in C but appeared in the C dynamic table"
        );
        assert!(
            !r_syms.contains(internal),
            "{internal} is static in C so the Rust .so must not export it"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r_path = common::rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&r_path)
        .output()
        .expect("nm");
    assert!(out.status.success());

    // Everything the Rust cdylib imports must be resolvable from libc / the
    // loader; there must be no dangling reference to an untranslated module.
    let known_prefixes = [
        "printf", "puts", "putchar", "fwrite", "memcpy", "memmove", "memset", "memcmp",
        "__libc", "__cxa", "_ITM_", "__gmon_start__", "__tls_get_addr", "abort", "malloc",
        "free", "realloc", "calloc", "posix_memalign", "write", "strlen", "bcmp",
        "__assert_fail", "dl_iterate_phdr", "_Unwind", "pthread", "sigaltstack",
        "sigaction", "sysconf", "mmap", "munmap", "mprotect", "getenv", "syscall",
        "gnu_get_libc_version", "__errno_location", "__stack_chk_fail", "stat", "open",
        "close", "read", "readlink", "getcwd", "environ", "sigemptyset", "raise",
        "__register_atfork", "_exit", "exit", "fstat", "lseek", "gettid", "realpath",
        "poll", "futex", "clock_gettime", "nanosleep", "sched_yield", "getrandom",
        "abs", "fabs", "memrchr", "strchr", "dlsym", "dladdr",
    ];
    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let name = match line.split_whitespace().next() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap_or(name);
        if known_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        unexpected.push(base.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined symbols (possible untranslated module): {unexpected:?}"
    );
}

#[test]
fn both_libraries_load_and_expose_all_five_entry_points() {
    // Loading through libloading is itself the check: `Api::load` resolves all
    // five symbols and panics if any is absent.
    let c = common::c_api();
    let r = common::rust_api();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // Distinct code: the two libraries must not have collapsed onto one mapping.
    assert_ne!(
        c.driver as usize, r.driver as usize,
        "C and Rust `driver` resolved to the same address — the two .so files were not both loaded"
    );
}

// ---------------------------------------------------------------------------
// Harness self-checks (negative controls)
//
// A differential harness that silently captures nothing would make every test
// pass vacuously. These tests prove the capture actually observes bytes and that
// `diff` really fails when the two sides differ.
// ---------------------------------------------------------------------------

#[test]
fn harness_capture_observes_real_bytes() {
    for api in [common::c_api(), common::rust_api()] {
        let out = common::capture(|| unsafe { (api.print_int_line)(1234567) });
        assert_eq!(
            out, b"1234567\n",
            "{} capture returned {:?} — fd-1 redirection is not working",
            api.name,
            common::show(&out)
        );
        let out2 = common::capture(|| unsafe { (api.driver)(2.0, 0.0) });
        assert_eq!(
            String::from_utf8_lossy(&out2),
            "Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n-2147483648\nFinished bad()\n",
            "{} driver output unexpected",
            api.name
        );
    }
}

#[test]
fn harness_diff_detects_divergence() {
    // Deliberately compare two *different* operations; `diff` must panic.
    let flip = std::sync::atomic::AtomicBool::new(false);
    let result = std::panic::catch_unwind(|| {
        common::diff("negative control", |api| unsafe {
            // Same API on both sides, but the payload changes between the two
            // captures, so the byte streams must differ.
            let n = if flip.fetch_xor(true, std::sync::atomic::Ordering::Relaxed) {
                1
            } else {
                2
            };
            (api.print_int_line)(n);
        });
    });
    assert!(
        result.is_err(),
        "negative control PASSED — the differential harness cannot detect divergence"
    );
}
