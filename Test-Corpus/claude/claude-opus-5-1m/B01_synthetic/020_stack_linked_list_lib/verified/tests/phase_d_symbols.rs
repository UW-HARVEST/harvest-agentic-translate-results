//! Phase D -- symbol parity enforced as a test.
//!
//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name, and both must be reachable via `dlsym`.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::process::Command;

fn exported_symbols(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so])
        .output()
        .expect("run nm -D --defined-only");
    assert!(
        out.status.success(),
        "nm failed on {so}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        // Ignore linker/CRT bookkeeping that is not part of either public ABI.
        .filter(|s| {
            !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__bss_start")
                && !s.starts_with("_edata")
                && !s.starts_with("_end")
        })
        .collect()
}

fn so_path(kind: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    let candidates: Vec<String> = match kind {
        "c" => vec![
            std::env::var("SIMPLELIST_C_SO").unwrap_or_default(),
            format!("{root}/c_src/build/libSimpleList.so"),
            format!("{root}/target/c-build/libSimpleList.so"),
        ],
        _ => vec![
            std::env::var("SIMPLELIST_RUST_SO").unwrap_or_default(),
            format!("{root}/target/release/libSimpleList.so"),
            format!("{root}/target/diff-so/release/libSimpleList.so"),
            format!("{root}/target/debug/libSimpleList.so"),
        ],
    };
    candidates
        .into_iter()
        .find(|p| !p.is_empty() && std::path::Path::new(p).is_file())
        .unwrap_or_else(|| panic!("no {kind} .so found; run tests via ./run_verification.sh"))
}

#[test]
fn symbols_rust_exports_every_c_symbol() {
    // Force the harness to build/locate both artifacts first.
    let _ = impls();

    let c_syms = exported_symbols(&so_path("c"));
    let r_syms = exported_symbols(&so_path("rust"));

    assert!(
        c_syms.contains("smallestValue"),
        "sanity: the C .so must export smallestValue, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {r_syms:?}",
        missing.len()
    );
}

#[test]
fn symbols_no_undefined_non_libc_in_rust() {
    let _ = impls();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", &so_path("rust")])
        .output()
        .expect("run nm -D --undefined-only");
    assert!(out.status.success());

    // libc / libgcc / CRT imports pulled in by linking `std`. Anything outside
    // this set would indicate an untranslated dependency.
    const ALLOWED_PREFIXES: [&str; 6] =
        ["_ITM_", "__cxa_", "__gmon_", "_Unwind_", "pthread_", "__libc_"];
    const ALLOWED: [&str; 33] = [
        "__errno_location", "__tls_get_addr", "abort", "bcmp", "calloc", "close",
        "dl_iterate_phdr", "free", "fstat64", "fstat", "getcwd", "getenv", "gettid",
        "lseek64", "lseek", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap64",
        "mmap", "munmap", "open64", "open", "posix_memalign", "read", "readlink",
        "realloc", "realpath", "stat64", "statx", "syscall",
    ];
    const ALLOWED2: [&str; 3] = ["strlen", "write", "writev"];

    let text = String::from_utf8_lossy(&out.stdout);
    let mut unexpected = Vec::new();
    for line in text.lines() {
        let Some(sym) = line.split_whitespace().last() else { continue };
        let bare = sym.split('@').next().unwrap_or(sym);
        let ok = ALLOWED_PREFIXES.iter().any(|p| bare.starts_with(p))
            || ALLOWED.contains(&bare)
            || ALLOWED2.contains(&bare);
        if !ok {
            unexpected.push(bare.to_string());
        }
    }
    assert!(
        unexpected.is_empty(),
        "the Rust .so imports non-libc symbols (possible untranslated dependency): {unexpected:?}"
    );
}

/// `dlsym` must succeed on both -- the strongest form of "the export exists".
#[test]
fn symbols_dlsym_reachable_in_both() {
    let im = impls();
    // Loading already performed dlsym("smallestValue") on both; prove they are
    // live by calling through each with the documented error input.
    let empty: Vec<i32> = Vec::new();
    assert_eq!(im.c.smallest_value(std::ptr::null_mut()), -1);
    assert_eq!(im.rust.smallest_value(std::ptr::null_mut()), -1);
    assert_same("D/dlsym-live", std::ptr::null_mut(), &empty);
}
