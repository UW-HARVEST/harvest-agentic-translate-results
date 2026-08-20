//! Phase D — symbol parity: every symbol the C `.so` exports must also be
//! exported by the Rust `.so`, under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

fn nm_undefined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The 16 symbols `nm -D --defined-only` reports for the C shared object.
const EXPECTED_C_SYMBOLS: &[&str] = &[
    "buffer_conditional_copy",
    "buffer_copy",
    "buffer_copy_strided",
    "buffer_interleave",
    "buffer_merge",
    "buffer_reverse",
    "buffer_rotate",
    "buffer_split",
    "calculate_checksum",
    "free_buffer_array",
    "init_buffer_array",
    "main",
    "process_buffer_array",
    "read_buffer",
    "validate_buffer",
    "write_buffer",
];

#[test]
fn c_symbol_set_is_the_documented_one() {
    let c = nm_defined(common::c_so_path());
    let expected: BTreeSet<String> = EXPECTED_C_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C shared object's exported symbol set changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(common::c_so_path());
    let r = nm_defined(common::rust_so_path());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {:?}",
        missing
    );
    assert_eq!(c.len(), 16, "expected 16 C symbols, got {}", c.len());
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_both() {
    // `Api::load` panics if any of the 16 symbols cannot be resolved.
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    assert!(
        r.reset_stdin.is_some(),
        "the Rust .so should also export the test-support reset hook"
    );
    assert!(
        c.reset_stdin.is_none(),
        "the C .so must not have a reset hook (it uses freopen instead)"
    );
}

#[test]
fn rust_has_no_undefined_non_libc_symbols() {
    let u = nm_undefined(common::rust_so_path());
    // Everything a `cdylib` may legitimately import: libc, libm, libgcc/unwind
    // and the dynamic loader.
    let allowed_prefix = [
        "_", "__", "abort", "accept", "bcmp", "calloc", "close", "closedir", "connect", "dl",
        "environ", "exit", "fcntl", "fdopendir", "free", "ftruncate", "get", "gmtime_r", "isatty",
        "lseek", "malloc", "mem", "mkdir", "mmap", "mprotect", "munmap", "nanosleep", "open",
        "opendir", "pipe", "poll", "posix_", "pread", "pthread_", "pwrite", "read", "readdir",
        "readlink", "realloc", "realpath", "recv", "rename", "rmdir", "sched_", "send", "set",
        "shutdown", "sigaction", "sigaltstack", "signal", "socket", "stat", "strerror", "strlen",
        "sym", "sync", "syscall", "sysconf", "unlink", "write",
    ];
    let bad: Vec<&String> = u
        .iter()
        .filter(|s| {
            let s = s.trim_start_matches('_');
            !allowed_prefix
                .iter()
                .any(|p| s.starts_with(p.trim_start_matches('_')))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined symbols that are not libc/runtime: {:?}",
        bad
    );
}

#[test]
fn struct_layouts_match_the_c_abi() {
    // Sanity: if these ever differ from the C compiler's view, every other
    // differential test would be meaningless.
    assert_eq!(core::mem::size_of::<common::BufferT>(), 272);
    assert_eq!(core::mem::align_of::<common::BufferT>(), 8);
    assert_eq!(core::mem::size_of::<common::BufferArrayT>(), 16);
    let b = common::BufferT::zeroed();
    let base = &b as *const _ as usize;
    assert_eq!(&b.data as *const _ as usize - base, 0);
    assert_eq!(&b.length as *const _ as usize - base, 256);
    assert_eq!(&b.checksum as *const _ as usize - base, 264);
}
