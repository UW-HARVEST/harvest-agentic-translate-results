// Phase D — exported-symbol parity between the C and the Rust shared object,
// plus self-checks proving the differential harness really compares output.

mod common;

use common::{c_so, driver_lines, libs, main_lines, rust_so, Side};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name.
fn c_and_rust_export_the_same_symbols() {
    let c = defined(&c_so());
    let r = defined(&rust_so());

    assert!(
        c.contains("driver") && c.contains("main"),
        "unexpected C symbol set: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );

    // The Rust side must not smuggle in extra public surface either.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

/// Nothing in the Rust `.so` may be left as an unresolved reference to
/// non-libc/runtime code (which would mean part of the translation is missing).
fn rust_so_has_no_foreign_undefined_symbols() {
    let undef = nm(&rust_so(), "--undefined-only");
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
        "__libc_", "__isoc99_",
    ];
    for sym in &undef {
        let bare = sym.split('@').next().unwrap_or(sym);
        if allowed_prefixes.iter().any(|p| bare.starts_with(p)) {
            continue;
        }
        // Everything else must be resolvable in libc, i.e. a plain libc call.
        assert!(
            LIBC_SYMBOLS.contains(&bare),
            "Rust .so has an unexpected undefined symbol `{sym}` - is part of the \
             translation still referring to foreign code?"
        );
    }
    assert!(
        undef.iter().any(|s| s.starts_with("read")),
        "expected the Rust .so to import read(2); got {undef:?}"
    );
}

/// The libc/loader imports the Rust `.so` is allowed to have.
const LIBC_SYMBOLS: &[&str] = &[
    "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64", "getcwd",
    "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
    "mmap", "mmap64", "munmap", "open", "open64", "poll", "posix_memalign", "pthread_key_create",
    "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "pthread_mutex_lock",
    "pthread_mutex_trylock", "pthread_mutex_unlock", "pthread_self", "read", "readlink",
    "realloc", "realpath", "sigaction", "sigaltstack", "stat", "stat64", "statx", "strlen",
    "syscall", "sysconf", "write", "writev", "memrchr", "getrandom", "__errno_location",
];

/// Both symbols really are callable through `dlsym` in both objects.
fn both_symbols_are_callable_via_dlsym() {
    let l = libs();
    for side in [Side::C, Side::Rust] {
        let d = l.driver(side);
        assert!(!(d as usize == 0), "{} driver symbol is null", side.name());
        let m = l.main(side);
        assert!(!(m as usize == 0), "{} main symbol is null", side.name());
    }
}

/// Harness self-check: the capture machinery must return the real output, so that
/// a comparison of two empty strings cannot masquerade as a pass.
fn harness_capture_self_check() {
    let bits = [
        0x3ff8_0000_0000_0000u64, // 1.5
        0x0000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
    ];
    let expected: [&[u8]; 3] = [
        b"3ff8000000000000 0x1.8p+0 1.5000",
        b"0 0x0p+0 0.0000",
        b"7ff0000000000000 inf inf",
    ];
    for side in [Side::C, Side::Rust] {
        let lines = driver_lines(side, &bits);
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(
                lines[i].as_slice(),
                *exp,
                "{} driver() line {i} was {:?}",
                side.name(),
                String::from_utf8_lossy(&lines[i])
            );
        }
    }

    let inputs: Vec<Vec<u8>> = vec![b"1.5".to_vec(), b"".to_vec(), b" -inf".to_vec()];
    let expected: [&[u8]; 3] = [
        b"3ff8000000000000 0x1.8p+0 1.5000",
        b"0 0x0p+0 0.0000",
        b"fff0000000000000 -inf -inf",
    ];
    for side in [Side::C, Side::Rust] {
        let lines = main_lines(side, &inputs);
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(
                lines[i].as_slice(),
                *exp,
                "{} main() line {i} was {:?}",
                side.name(),
                String::from_utf8_lossy(&lines[i])
            );
        }
    }
}

fn main() {
    common::run_suite(
        "symbols",
        &[
            (
                "c_and_rust_export_the_same_symbols",
                c_and_rust_export_the_same_symbols,
            ),
            (
                "rust_so_has_no_foreign_undefined_symbols",
                rust_so_has_no_foreign_undefined_symbols,
            ),
            (
                "both_symbols_are_callable_via_dlsym",
                both_symbols_are_callable_via_dlsym,
            ),
            ("harness_capture_self_check", harness_capture_self_check),
        ],
    );
}
