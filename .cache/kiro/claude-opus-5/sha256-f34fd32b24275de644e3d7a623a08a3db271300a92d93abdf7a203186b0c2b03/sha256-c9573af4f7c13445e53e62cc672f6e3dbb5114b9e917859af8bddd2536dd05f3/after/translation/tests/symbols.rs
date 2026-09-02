//! Phase D — symbol parity gate.
//!
//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name (including macro-generated ones), and the
//! Rust `.so` must not import anything that is not libc / the language runtime.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let dir = root().join("c_src/build");
    let mut best = None;
    for e in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
        .flatten()
    {
        let p = e.path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            best = Some(p);
        }
    }
    best.expect("no C .so; build c_src first")
}

fn rust_so() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libstr_put_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so; run `cargo build --release`");
}

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm {extra} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn defined_symbol_diff_is_empty() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so(), "--defined-only");
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         Per the Phase A rule these must be exported (or the missing C module \
         translated), never stubbed.",
        missing.len(),
        missing
    );
    assert_eq!(
        c.len(),
        16,
        "the C .so is expected to export exactly 16 symbols, found {}: {:?}",
        c.len(),
        c
    );
    for s in [
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_string",
        "stbds_hash_bytes",
        "stbds_hmfree_func",
        "stbds_hmget_key_ts",
        "stbds_hmget_key",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_shmode_func",
        "stbds_hmdel_key",
        "stbds_stralloc",
        "stbds_strreset",
        "strkey",
        "str_put",
    ] {
        assert!(c.contains(s), "C .so lost {s}");
        assert!(r.contains(s), "Rust .so does not export {s}");
    }
}

#[test]
fn rust_so_imports_only_runtime_symbols() {
    let r = nm(&rust_so(), "--undefined-only");
    // Anything the Rust .so imports must come from libc / libgcc / the Rust
    // runtime — never from a C translation unit that was left untranslated.
    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
        "stat",
        "fstat",
        "lseek",
        "open",
        "mmap",
        "munmap",
    ];
    let allowed_exact = [
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "getcwd",
        "getenv",
        "gettid",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "posix_memalign",
        "printf",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sprintf",
        "strcmp",
        "strlen",
        "syscall",
        "write",
        "writev",
    ];
    for s in &r {
        let bare = s.split('@').next().unwrap();
        let ok = allowed_prefixes.iter().any(|p| bare.starts_with(p))
            || allowed_exact.contains(&bare);
        assert!(ok, "unexpected undefined symbol in the Rust .so: {s}");
        assert!(
            !bare.starts_with("stbds_"),
            "the Rust .so references an untranslated stb_ds symbol: {s}"
        );
    }
}
