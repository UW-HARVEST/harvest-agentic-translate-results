//! Phase D: symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_syms(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 3 && matches!(f[1], "T" | "D" | "B" | "R" | "W") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn c_so_symbols_are_all_exported_by_rust_so() {
    let c = defined_syms(&c_so_path());
    let r = defined_syms(&rust_so_path());
    assert!(!c.is_empty(), "no symbols read from the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    // Sanity: the 31 known functions must all be present on both sides.
    for name in [
        "c22",
        "c23",
        "c2Add",
        "c2BBVerts",
        "c2CCW90",
        "c2Clampv",
        "c2D",
        "c2Det2",
        "c2Div",
        "c2Dot",
        "c2GJK",
        "c2GJKSimplexMetric",
        "c2L",
        "c2Len",
        "c2MakeProxy",
        "c2Maxv",
        "c2Minv",
        "c2Mulrv",
        "c2MulrvT",
        "c2Mulvs",
        "c2Mulxv",
        "c2Neg",
        "c2Norm",
        "c2RotIdentity",
        "c2Skew",
        "c2Sub",
        "c2Support",
        "c2V",
        "c2Witness",
        "c2xIdentity",
        "gjk_cache",
    ] {
        assert!(c.contains(name), "C .so is missing {name}");
        assert!(r.contains(name), "Rust .so is missing {name}");
    }
    assert_eq!(c.len(), 31, "unexpected C symbol count: {c:?}");
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = line.split_whitespace().last().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let base = name.split('@').next().unwrap_or(name);
        let ok = base.starts_with('_')
            || base.starts_with("c2")
            || matches!(
                base,
                "abort"
                    | "bcmp"
                    | "calloc"
                    | "close"
                    | "dl_iterate_phdr"
                    | "free"
                    | "fstat64"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "lseek64"
                    | "malloc"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "mmap64"
                    | "munmap"
                    | "open64"
                    | "posix_memalign"
                    | "pthread_key_create"
                    | "pthread_key_delete"
                    | "pthread_setspecific"
                    | "read"
                    | "readlink"
                    | "realloc"
                    | "realpath"
                    | "stat64"
                    | "statx"
                    | "strlen"
                    | "syscall"
                    | "write"
                    | "writev"
                    | "sqrtf"
                    | "sqrt"
            );
        if !ok {
            suspicious.push(name.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined non-libc symbols: {suspicious:?}"
    );
}

#[test]
fn struct_layouts_match_the_c_source() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(size_of::<c2r>(), 8);
    assert_eq!(size_of::<c2x>(), 16);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2GJKCache>(), 36);
    assert_eq!(size_of::<c2Proxy>(), 72);
    assert_eq!(size_of::<c2sv>(), 36);
    assert_eq!(size_of::<c2Simplex>(), 152);
    assert_eq!(align_of::<c2Simplex>(), 4);
}

#[test]
fn both_libraries_load_and_resolve_every_symbol() {
    let p = load_pair();
    // c2RotIdentity / c2xIdentity take no inputs: pure constant parity.
    unsafe {
        let rc = (p.c.c2RotIdentity)();
        let rr = (p.r.c2RotIdentity)();
        assert!(req(rc, rr), "c2RotIdentity divergence");
        let xc = (p.c.c2xIdentity)();
        let xr = (p.r.c2xIdentity)();
        assert!(xeq(xc, xr), "c2xIdentity divergence");
        assert_eq!(rc.c, 1.0);
        assert_eq!(rc.s, 0.0);
    }
    println!("C   .so: {}", c_so_path().display());
    println!("Rust.so: {}", rust_so_path().display());
}
