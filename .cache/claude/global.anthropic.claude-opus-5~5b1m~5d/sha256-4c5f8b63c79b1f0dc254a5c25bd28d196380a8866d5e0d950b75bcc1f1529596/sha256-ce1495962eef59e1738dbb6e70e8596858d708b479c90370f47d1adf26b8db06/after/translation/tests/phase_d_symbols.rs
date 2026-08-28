//! Phase D — symbol parity and harness self-checks, executed as tests so they
//! run under every profile / feature combination.

mod common;

use common::*;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every symbol exported by the C `.so` must also be exported by the Rust one.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = c_lib_path();
    let r = rust_lib_path();
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());

    let cs = nm_defined(&c);
    let rs = nm_defined(&r);
    assert!(!cs.is_empty(), "C .so exports nothing?");
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {cs:?}\nRust: {rs:?}"
    );
    // The three known entry points must really be there (guards against an nm
    // parsing mishap silently making the check vacuous).
    for want in ["tflac_pack_u64le", "tflac_md5_addsample", "update_md5"] {
        assert!(cs.iter().any(|s| s == want), "C .so lacks {want}");
        assert!(rs.iter().any(|s| s == want), "Rust .so lacks {want}");
    }
}

/// The Rust `.so` must not depend on anything beyond libc / the unwinder.
#[test]
fn rust_so_has_only_libc_undefined_symbols() {
    let undef = nm_undefined(&rust_lib_path());
    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "abort@",
        "bcmp@",
        "calloc@",
        "close@",
        "dl_iterate_phdr@",
        "free@",
        "fstat",
        "getcwd@",
        "getenv@",
        "gettid@",
        "lseek",
        "malloc@",
        "memcpy@",
        "memmove@",
        "memset@",
        "mmap",
        "munmap@",
        "open",
        "posix_memalign@",
        "pthread_",
        "read@",
        "readlink@",
        "realloc@",
        "realpath@",
        "stat",
        "statx@",
        "strlen@",
        "syscall@",
        "write@",
        "writev@",
    ];
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(bad.is_empty(), "unexpected undefined symbols: {bad:?}");
}

/// Harness self-check: the two libraries must be *different* files, otherwise
/// every differential assertion would be trivially true.
#[test]
fn harness_loads_two_distinct_libraries() {
    let c = c_lib_path().canonicalize().unwrap();
    let r = rust_lib_path().canonicalize().unwrap();
    assert_ne!(c, r, "C and Rust .so paths are identical");
    assert!(
        c.to_string_lossy().contains("c_src"),
        "C library should come from c_src/build, got {}",
        c.display()
    );
    assert!(
        r.to_string_lossy().contains("target"),
        "Rust library should come from target/, got {}",
        r.display()
    );

    // ...and they must be *distinguishable*: a deliberately different input to
    // each must produce different memory, proving both are really invoked.
    let api = both();
    let mut a = Arena::zeroed();
    let mut b = Arena::zeroed();
    unsafe {
        (api.c.pack_u64le)(a.as_ptr(), 0x1122_3344_5566_7788);
        (api.rust.pack_u64le)(b.as_ptr(), 0x8877_6655_4433_2211);
    }
    assert_ne!(
        a.bytes(),
        b.bytes(),
        "the two libraries appear to be the same code path"
    );
}

/// Guard against "stub" translations: the Rust `.so` must not contain the
/// panic strings that `unimplemented!()` / `todo!()` emit.
#[test]
fn rust_so_contains_no_stub_panics() {
    let bytes = std::fs::read(rust_lib_path()).expect("read rust .so");
    for needle in [
        b"not implemented".as_slice(),
        b"not yet implemented".as_slice(),
    ] {
        let found = bytes.windows(needle.len()).any(|w| w == needle);
        assert!(
            !found,
            "Rust .so contains the stub panic string {:?}",
            String::from_utf8_lossy(needle)
        );
    }
}
