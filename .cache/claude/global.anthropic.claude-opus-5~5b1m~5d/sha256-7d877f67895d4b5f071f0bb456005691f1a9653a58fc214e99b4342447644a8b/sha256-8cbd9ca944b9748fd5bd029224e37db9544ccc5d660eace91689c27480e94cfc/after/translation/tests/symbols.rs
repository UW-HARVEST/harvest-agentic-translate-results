//! Phase D — symbol parity, plus sanity checks that the harness really is
//! comparing two DIFFERENT libraries (a harness that accidentally loaded the
//! same `.so` twice would pass every differential test vacuously).

#![allow(non_snake_case)]

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

/// Every external-linkage function in c_src/src/lib.c.
const EXPECTED: [&str; 38] = [
    "c22",
    "c23",
    "c2AABBtoAABB",
    "c2AABBtoCapsule",
    "c2Add",
    "c2BBVerts",
    "c2CCW90",
    "c2CapsuletoCapsule",
    "c2CircletoAABB",
    "c2CircletoCapsule",
    "c2CircletoCircle",
    "c2Clampv",
    "c2Collided",
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
    "reverse_collide",
];

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: PathBuf, exact: Option<&str>) -> PathBuf {
    let mut found: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .flatten()
    {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            match exact {
                Some(n) if p.file_name().unwrap() != n => {}
                _ => found.push(p),
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| panic!("no .so in {}", dir.display()))
}

fn defined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind == "T" || kind == "t" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn phaseD_symbol_diff_is_empty() {
    let c_so = std::env::var("C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_so(crate_root().join("../c_src/build"), None));
    let rust_so = std::env::var("RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
        for prof in ["release", "debug"] {
            let p = crate_root().join("target").join(prof).join("libreverse_collide_lib.so");
            if p.exists() {
                return p;
            }
        }
        panic!("no Rust .so found")
    });

    let cs = defined_symbols(&c_so);
    let rs = defined_symbols(&rust_so);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // The full C surface must be present, matching the hand-derived list.
    for name in EXPECTED.iter() {
        assert!(cs.contains(*name), "C .so unexpectedly lacks `{name}`");
        assert!(rs.contains(*name), "Rust .so lacks `{name}`");
    }
    assert_eq!(
        cs.len(),
        EXPECTED.len(),
        "C .so exports {} symbols, SYMBOLS.md lists {}: {:?}",
        cs.len(),
        EXPECTED.len(),
        cs
    );
    assert_eq!(rs.len(), EXPECTED.len(), "Rust .so exports {:?}", rs);
}

#[test]
fn phaseD_no_unresolved_non_libc_symbols() {
    let rust_so = std::env::var("RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
        for prof in ["release", "debug"] {
            let p = crate_root().join("target").join(prof).join("libreverse_collide_lib.so");
            if p.exists() {
                return p;
            }
        }
        panic!("no Rust .so found")
    });
    let undef = undefined_symbols(&rust_so);
    // Everything the Rust .so imports must be libc / the language runtime.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno_location", "__gmon_start__", "__tls_get_addr",
        "__libc_", "__stack_chk", "__memcpy", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr",
        "free", "fstat", "getcwd", "getenv", "gettid", "lseek", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "munmap", "open", "posix_memalign", "pthread_", "read",
        "readlink", "realloc", "realpath", "sqrtf", "stat", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getrandom", "poll", "sigaltstack", "sigaction", "mprotect",
        "pipe2", "signal", "raise", "nanosleep", "clock_gettime", "environ", "_edata", "_end",
        "__bss_start", "dlsym", "dladdr", "getpid",
    ];
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !allowed_prefixes.iter().any(|p| base.starts_with(p))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbol(s): {bad:?}"
    );
}

/// The harness must be loading two genuinely distinct shared objects.
#[test]
fn phaseD_harness_loads_two_distinct_libraries() {
    for name in EXPECTED.iter() {
        let (c, r) = sym::<*const ()>(name);
        let (ca, ra) = unsafe {
            (
                c.into_raw().as_raw_ptr() as usize,
                r.into_raw().as_raw_ptr() as usize,
            )
        };
        assert_ne!(
            ca, ra,
            "`{name}` resolved to the same address in both libraries — the harness \
             is comparing one library against itself"
        );
    }
}

/// Negative control: the comparison machinery must actually be able to FAIL.
#[test]
fn phaseD_bit_comparison_detects_divergence() {
    // f32 bit comparison
    assert!(!0.0f32.bit_eq(&-0.0f32), "+0.0 must not compare equal to -0.0");
    assert!(1.0f32.bit_eq(&1.0f32));
    assert!(f32::NAN.bit_eq(&f32::NAN));
    // struct comparison
    let a = c2v { x: 1.0, y: 2.0 };
    let b = c2v { x: 1.0, y: 2.000_000_2 };
    assert!(!a.bit_eq(&b), "near-equal vectors must not compare equal");
    // and `same` must panic on divergence
    let res = std::panic::catch_unwind(|| same("negative control", "x", 1.0f32, 2.0f32));
    assert!(res.is_err(), "`same` failed to report a divergence");
}
