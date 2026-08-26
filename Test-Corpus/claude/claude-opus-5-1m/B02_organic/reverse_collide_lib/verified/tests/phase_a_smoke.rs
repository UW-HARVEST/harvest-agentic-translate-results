//! Phase A — harness smoke test + struct-layout / symbol-parity assertions.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::os::raw::c_int;
use std::process::Command;

#[test]
fn a01_struct_layouts_match_the_c_abi() {
    assert_eq!(std::mem::size_of::<c2v>(), 8);
    assert_eq!(std::mem::size_of::<c2r>(), 8);
    assert_eq!(std::mem::size_of::<c2x>(), 16);
    assert_eq!(std::mem::size_of::<c2Circle>(), 12);
    assert_eq!(std::mem::size_of::<c2AABB>(), 16);
    assert_eq!(std::mem::size_of::<c2Capsule>(), 20);
    assert_eq!(std::mem::size_of::<c2GJKCache>(), 36);
    assert_eq!(std::mem::size_of::<c2Proxy>(), 72);
    assert_eq!(std::mem::size_of::<c2sv>(), 36);
    // c2Simplex == 4 * c2sv + float + int
    assert_eq!(std::mem::size_of::<c2Simplex>(), 4 * 36 + 8);
}

#[test]
fn a02_both_libraries_load_and_resolve_every_symbol() {
    let (c, r) = libs();
    assert_eq!(c.tag, "C");
    assert_eq!(r.tag, "RUST");
    assert_eq!(Api::SYMBOLS.len(), 38, "harness must cover all 38 exports");
    // Actually calling one function from each proves the handles are distinct
    // and both really resolved.
    unsafe {
        eq_int("reverse_collide(0,0,0)", (c.reverse_collide)(0.0, 0.0, 0.0), (r.reverse_collide)(0.0, 0.0, 0.0));
    }
}

#[test]
fn a03_the_two_handles_are_distinct_libraries() {
    let (c, r) = libs();
    let a = (c.reverse_collide) as usize;
    let b = (r.reverse_collide) as usize;
    assert_ne!(
        a, b,
        "both handles resolved to the same address — the Rust .so was not really loaded"
    );
}

/// `nm -D` parity, run from inside the test suite so it is enforced on every
/// `cargo test` and for every feature combination.
#[test]
fn a04_nm_dynamic_symbol_parity() {
    fn defined(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(&c_so_path());
    let r = defined(&rust_so_path());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );

    assert_eq!(c.len(), 38, "C .so export count changed: {c:?}");

    // No unresolved non-libc references in the Rust .so.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno", "__gmon", "__tls_", "pthread_", "std::",
    ];
    let libc_names = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "sqrtf", "stat", "stat64", "statx", "strlen",
        "syscall", "write", "writev",
    ];
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let sym = match line.split_whitespace().last() {
            Some(s) => s,
            None => continue,
        };
        let base = sym.split('@').next().unwrap();
        let ok = allowed_prefixes.iter().any(|p| base.starts_with(p))
            || libc_names.contains(&base);
        assert!(ok, "unexpected undefined symbol in the Rust .so: {sym}");
    }
}

#[test]
fn a05_no_feature_gated_code_paths_exist() {
    // Documented in CONFIGS.md: neither the C nor the Rust build has any
    // conditional compilation, so there is exactly one configuration.
    let c = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/src/lib.c"),
    )
    .unwrap();
    assert!(
        !c.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with("#if") || t.starts_with("#ifdef") || t.starts_with("#ifndef")
        }),
        "the C source gained a preprocessor conditional — CONFIGS.md must be updated"
    );
    let toml =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml gained a [features] section — CONFIGS.md must be updated"
    );
}

/// Sanity: the C's `sqrtf` and Rust's `f32::sqrt` agree bit-exactly, including
/// on the negative domain (where the result is a NaN whose bit pattern must
/// also agree) — this underpins every `c2Len`/`c2Norm` comparison.
#[test]
fn a06_sqrtf_domain_parity() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xA06);
    unsafe {
        for v in GRID {
            let a = c2v { x: v, y: 0.0 };
            eq_f32(&format!("c2Len({v:?},0)"), (c.c2Len)(a), (r.c2Len)(a));
        }
        for _ in 0..4096 {
            let a = rng.wild_v();
            eq_f32(&format!("c2Len({a:?})"), (c.c2Len)(a), (r.c2Len)(a));
        }
        // Force the sqrt(negative) path: c2Dot(a,a) can only be negative when
        // it involves an inf*0 -> NaN or a -inf, so drive it directly through
        // the vectors that produce those.
        let cases = [
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: FLT_MAX, y: FLT_MAX },
            c2v { x: 1.0e30, y: -1.0e30 },
        ];
        for a in cases {
            eq_f32(&format!("c2Len({a:?})"), (c.c2Len)(a), (r.c2Len)(a));
            eq_v(&format!("c2Norm({a:?})"), (c.c2Norm)(a), (r.c2Norm)(a));
        }
        let _ = c2v::default();
        let _: c_int = 0;
    }
}
