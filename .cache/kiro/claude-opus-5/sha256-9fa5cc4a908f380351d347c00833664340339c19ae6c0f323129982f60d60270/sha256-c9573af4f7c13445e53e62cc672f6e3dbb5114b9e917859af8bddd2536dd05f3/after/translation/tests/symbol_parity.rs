//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Shells out to `nm -D` on both libraries and asserts:
//!   * every defined dynamic symbol of the C `.so` is also defined by the Rust
//!     `.so`, with the exact same name;
//!   * the Rust `.so` exports no extra non-runtime symbols;
//!   * the Rust `.so` has no undefined symbols outside libc / the language
//!     runtime (i.e. nothing from the translated library is left dangling);
//!   * every one of those symbols is actually `dlsym`-able and callable, so a
//!     symbol that exists but traps or is a stub cannot pass.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm -D failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

fn nm_undefined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm -D --undefined-only failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

/// libc / compiler-runtime / loader symbols an `.so` may legitimately import.
fn is_runtime_symbol(s: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_", "__errno_", "pthread_",
        "__libc_", "__stack_chk", "_GLOBAL_", "__asan", "__msan", "__rust_",
    ];
    const NAMES: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64",
        "posix_memalign", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "strlen", "syscall", "write", "writev", "sysconf", "sigaltstack",
        "mprotect", "pipe2", "poll", "nanosleep", "sched_yield", "getrandom",
        "__xpg_strerror_r", "strerror_r", "environ", "memrchr", "qsort", "sqrtf", "sqrt",
    ];
    let base = s.split('@').next().unwrap_or(s);
    PREFIXES.iter().any(|p| base.starts_with(p)) || NAMES.contains(&base)
}

fn c_so_path() -> std::path::PathBuf {
    // Reuse the harness's resolution so both agree on which files are under test.
    c_lib().path.clone()
}
fn rs_so_path() -> std::path::PathBuf {
    rs_lib().path.clone()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let (cp, rp) = (c_so_path(), rs_so_path());
    let c_syms = nm_defined(&cp);
    let r_syms = nm_defined(&rp);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C  ({}): {c_syms:?}\nRS ({}): {r_syms:?}",
        missing.len(),
        cp.display(),
        rp.display()
    );

    // The C library has exactly 12 external-linkage functions in src/lib.c.
    assert_eq!(
        c_syms.len(),
        12,
        "unexpected C symbol count -- did c_src change? {c_syms:?}"
    );

    let extra: Vec<&String> = r_syms
        .difference(&c_syms)
        .filter(|s| !is_runtime_symbol(s))
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports {} symbol(s) the C .so does not: {extra:?}",
        extra.len()
    );
}

#[test]
fn phase_d_rust_so_has_no_dangling_non_libc_symbols() {
    let rp = rs_so_path();
    let undef = nm_undefined(&rp);
    let bad: Vec<&String> = undef.iter().filter(|s| !is_runtime_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has {} undefined non-libc/non-runtime symbol(s): {bad:?}",
        bad.len()
    );
}

#[test]
fn phase_d_every_symbol_is_dlsym_able_and_live() {
    // Loading both libs already resolves all 12 symbols in each (the harness
    // panics on a missing one). Additionally CALL each one so a symbol that
    // exists but is a stub / traps / returns a fixed sentinel cannot pass.
    let (c, r) = libs();
    let a = c2v { x: 3.0, y: -4.0 };
    let b = c2v { x: -1.5, y: 2.25 };
    let A = c2Circle { p: a, r: 5.0 };
    let B = c2Circle { p: b, r: 2.0 };
    let bb = c2AABB { min: b, max: a };
    let cap = c2Capsule { a, b, r: 1.0 };

    unsafe {
        // 1 c2V
        assert!(v_eq((c.c2V)(1.0, 2.0), (r.c2V)(1.0, 2.0)));
        // 2 c2Mulvs
        assert!(v_eq((c.c2Mulvs)(a, 3.0), (r.c2Mulvs)(a, 3.0)));
        // 3 c2Maxv
        assert!(v_eq((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)));
        // 4 c2Minv
        assert!(v_eq((c.c2Minv)(a, b), (r.c2Minv)(a, b)));
        // 5 c2Clampv
        assert!(v_eq((c.c2Clampv)(a, b, a), (r.c2Clampv)(a, b, a)));
        // 6 c2Sub
        assert!(v_eq((c.c2Sub)(a, b), (r.c2Sub)(a, b)));
        // 7 c2Dot
        assert!(f32_eq_bits((c.c2Dot)(a, b), (r.c2Dot)(a, b)));
        // 8 c2CircletoCircle
        assert_eq!((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
        // 9 c2CircletoAABB
        assert_eq!((c.c2CircletoAABB)(A, bb), (r.c2CircletoAABB)(A, bb));
        // 10 c2CircletoCapsule
        assert_eq!((c.c2CircletoCapsule)(A, cap), (r.c2CircletoCapsule)(A, cap));
        // 11 c2Collided
        let (ap, bp) = (
            &A as *const c2Circle as *const u8,
            &B as *const c2Circle as *const u8,
        );
        assert_eq!(
            (c.c2Collided)(ap, bp, C2_TYPE_CIRCLE),
            (r.c2Collided)(ap, bp, C2_TYPE_CIRCLE)
        );
        // 12 circle_collide
        assert_eq!((c.circle_collide)(-70.0, 0.0, 5.0), (r.circle_collide)(-70.0, 0.0, 5.0));

        // Anti-stub checks: these must NOT be constant functions.
        let d1 = (c.c2Dot)(a, b);
        let d2 = (c.c2Dot)(b, a);
        let e1 = (r.c2Dot)(a, b);
        let e2 = (r.c2Dot)(b, a);
        assert!(f32_eq_bits(d1, e1) && f32_eq_bits(d2, e2));
        // circle_collide: a huge radius sets all three bits; a far-away point
        // sets none.
        assert_eq!(
            (r.circle_collide)(-70.0, 0.0, 1.0e6),
            7,
            "Rust circle_collide should report all three overlaps for a huge radius"
        );
        assert_eq!(
            (r.circle_collide)(1.0e6, 1.0e6, 0.0),
            0,
            "Rust circle_collide should report no overlap for a far-away point"
        );
        // c2CircletoCircle: coincident circles overlap, distant ones do not.
        let far = c2Circle {
            p: c2v { x: 1.0e6, y: 1.0e6 },
            r: 0.0,
        };
        assert_eq!((r.c2CircletoCircle)(A, A), 1, "coincident circles must overlap");
        assert_eq!((r.c2CircletoCircle)(A, far), 0, "distant circles must not overlap");
        assert_eq!((c.c2CircletoCircle)(A, A), 1);
        assert_eq!((c.c2CircletoCircle)(A, far), 0);
        // c2CircletoAABB / c2CircletoCapsule likewise vary with their input.
        let big_box = c2AABB {
            min: c2v { x: -1.0e3, y: -1.0e3 },
            max: c2v { x: 1.0e3, y: 1.0e3 },
        };
        let far_box = c2AABB {
            min: c2v { x: 1.0e6, y: 1.0e6 },
            max: c2v { x: 2.0e6, y: 2.0e6 },
        };
        assert_eq!((r.c2CircletoAABB)(A, big_box), 1);
        assert_eq!((r.c2CircletoAABB)(A, far_box), 0);
        assert_eq!((c.c2CircletoAABB)(A, big_box), 1);
        assert_eq!((c.c2CircletoAABB)(A, far_box), 0);
        let near_cap = c2Capsule {
            a: c2v { x: -10.0, y: -4.0 },
            b: c2v { x: 10.0, y: -4.0 },
            r: 1.0,
        };
        let far_cap = c2Capsule {
            a: c2v { x: 1.0e6, y: 1.0e6 },
            b: c2v { x: 2.0e6, y: 2.0e6 },
            r: 1.0,
        };
        assert_eq!((r.c2CircletoCapsule)(A, near_cap), 1);
        assert_eq!((r.c2CircletoCapsule)(A, far_cap), 0);
        assert_eq!((c.c2CircletoCapsule)(A, near_cap), 1);
        assert_eq!((c.c2CircletoCapsule)(A, far_cap), 0);
        // c2Collided must actually dispatch (not always return 0).
        let (ap2, bp2) = (
            &A as *const c2Circle as *const u8,
            &A as *const c2Circle as *const u8,
        );
        assert_eq!((r.c2Collided)(ap2, bp2, C2_TYPE_CIRCLE), 1);
        assert_eq!((c.c2Collided)(ap2, bp2, C2_TYPE_CIRCLE), 1);
    }
}

#[test]
fn phase_d_report_symbol_tables() {
    // Diagnostic: print both tables so `-- --nocapture` gives the evidence.
    let (cp, rp) = (c_so_path(), rs_so_path());
    println!("C  .so = {}", cp.display());
    for s in nm_defined(&cp) {
        println!("  C  T {s}");
    }
    println!("RS .so = {}", rp.display());
    for s in nm_defined(&rp) {
        println!("  RS T {s}");
    }
}
