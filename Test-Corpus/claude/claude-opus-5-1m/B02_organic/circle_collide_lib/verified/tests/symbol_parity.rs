//! Phase D — symbol parity between the C `.so` and the Rust `cdylib`.

mod common;

use common::*;
use std::process::Command;

/// `nm -D --defined-only` on a `.so`, keeping only exported text symbols and
/// dropping the toolchain-internal ones that start with `_` (`_init`, `_fini`,
/// `__bss_start`, …).
fn exported_text_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind == "T" && !name.starts_with('_') {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_syms = exported_text_symbols(&c_lib_path());
    let r_syms = exported_text_symbols(&rust_lib_path());

    assert_eq!(
        c_syms.len(),
        EXPECTED_SYMBOLS.len(),
        "the C .so exports {} text symbols, but EXPECTED_SYMBOLS lists {}. \
         Update SYMBOLS.md / EXPECTED_SYMBOLS and add tests for the new ones. C: {:?}",
        c_syms.len(),
        EXPECTED_SYMBOLS.len(),
        c_syms
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c_syms:?}\nRUST ({}): {r_syms:?}",
        c_lib_path().display(),
        rust_lib_path().display()
    );

    // Documented set must match reality in both directions.
    let mut expected: Vec<String> = EXPECTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(c_syms, expected, "C .so symbol set changed vs SYMBOLS.md");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args([
            "-D",
            "--undefined-only",
            rust_lib_path().to_str().unwrap(),
        ])
        .output()
        .expect("nm");
    assert!(out.status.success());
    let allowed_prefixes = [
        "_", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "getcwd",
        "getenv", "gettid", "lseek", "malloc", "memcpy", "memmove", "memset", "mmap", "munmap",
        "open", "posix_memalign", "pthread_", "read", "realloc", "realpath", "stat", "statx",
        "strlen", "syscall", "write", "writev",
    ];
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|name| {
            let base = name.split('@').next().unwrap_or(name);
            !allowed_prefixes.iter().any(|p| base.starts_with(p))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}"
    );
}

/// C30 — every documented symbol resolves through `dlsym` in *both* objects and
/// is callable back-to-back with both libraries mapped at once.
#[test]
fn c30_all_symbols_reachable() {
    let (c, r) = libs();
    for name in EXPECTED_SYMBOLS {
        // `libs()` already resolved every symbol eagerly; re-assert explicitly
        // so a failure names the symbol.
        assert!(
            !name.is_empty(),
            "symbol {name} unresolved in {} / {}",
            c.name,
            r.name
        );
    }

    let a = C2Circle {
        p: C2v::new(1.0, 2.0),
        r: 3.0,
    };
    let b = C2Circle {
        p: C2v::new(2.0, 3.0),
        r: 4.0,
    };
    let bb = C2Aabb {
        min: C2v::new(-1.0, -1.0),
        max: C2v::new(1.0, 1.0),
    };
    let cap = C2Capsule {
        a: C2v::new(-5.0, 0.0),
        b: C2v::new(5.0, 0.0),
        r: 2.0,
    };

    assert_v_bits((c.c2V)(1.5, -2.5), (r.c2V)(1.5, -2.5), "c2V");
    assert_v_bits(
        (c.c2Mulvs)(C2v::new(1.5, -2.5), 3.0),
        (r.c2Mulvs)(C2v::new(1.5, -2.5), 3.0),
        "c2Mulvs",
    );
    assert_v_bits((c.c2Maxv)(a.p, b.p), (r.c2Maxv)(a.p, b.p), "c2Maxv");
    assert_v_bits((c.c2Minv)(a.p, b.p), (r.c2Minv)(a.p, b.p), "c2Minv");
    assert_v_bits(
        (c.c2Clampv)(a.p, bb.min, bb.max),
        (r.c2Clampv)(a.p, bb.min, bb.max),
        "c2Clampv",
    );
    assert_v_bits((c.c2Sub)(a.p, b.p), (r.c2Sub)(a.p, b.p), "c2Sub");
    assert_f32_bits((c.c2Dot)(a.p, b.p), (r.c2Dot)(a.p, b.p), "c2Dot");
    assert_int(
        (c.c2CircletoCircle)(a, b),
        (r.c2CircletoCircle)(a, b),
        "c2CircletoCircle",
    );
    assert_int(
        (c.c2CircletoAABB)(a, bb),
        (r.c2CircletoAABB)(a, bb),
        "c2CircletoAABB",
    );
    assert_int(
        (c.c2CircletoCapsule)(a, cap),
        (r.c2CircletoCapsule)(a, cap),
        "c2CircletoCapsule",
    );
    unsafe {
        let pa = &a as *const C2Circle as *const std::os::raw::c_void;
        let pb = &b as *const C2Circle as *const std::os::raw::c_void;
        assert_int(
            (c.c2Collided)(pa, pb, C2_TYPE_CIRCLE),
            (r.c2Collided)(pa, pb, C2_TYPE_CIRCLE),
            "c2Collided",
        );
    }
    assert_int(
        (c.circle_collide)(-70.0, 0.0, 5.0),
        (r.circle_collide)(-70.0, 0.0, 5.0),
        "circle_collide",
    );
}
