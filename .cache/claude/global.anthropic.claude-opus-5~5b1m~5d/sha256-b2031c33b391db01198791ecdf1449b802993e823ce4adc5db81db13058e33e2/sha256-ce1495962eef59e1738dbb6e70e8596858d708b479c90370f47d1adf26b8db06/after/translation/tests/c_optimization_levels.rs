//! Robustness check: `lib.c` contains constructs whose behaviour is formally
//! undefined (union type-punning of `int`/`float`, `(int *)` reinterpretation of
//! an `unsigned char[4]`, signed overflow). If the Rust translation only agreed
//! with the *unoptimized* C build, the match would be accidental.
//!
//! This test compiles the untouched `c_src/src/lib.c` at several optimization
//! levels, loads each resulting `.so` with `libloading`, and compares all of
//! them against the Rust `.so` on the same inputs.

mod common;

use common::{pair, rust_so_path, Rng};
use libloading::{Library, Symbol};
use std::path::PathBuf;

type Memchra2 = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

fn build_variant(flag: &str, tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let src = root.join("c_src/src/lib.c");
    assert!(src.exists(), "missing {}", src.display());
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/harness");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join(format!("libc_{}_{}.so", tag, std::process::id()));
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let st = std::process::Command::new(&cc)
        .args(["-shared", "-fPIC", flag, "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .unwrap_or_else(|e| panic!("cc failed to start: {e}"));
    assert!(st.success(), "cc {flag} failed");
    out
}

#[test]
fn rust_matches_c_at_every_optimization_level() {
    let variants: Vec<(&str, &str)> = vec![
        ("-O0", "O0"),
        ("-O1", "O1"),
        ("-O2", "O2"),
        ("-O3", "O3"),
        ("-Os", "Os"),
    ];

    let rust_lib =
        unsafe { Library::new(rust_so_path()).expect("dlopen rust .so") };
    let rust_fn: Memchra2 = unsafe {
        let s: Symbol<Memchra2> = rust_lib.get(b"memchra2\0").expect("rust memchra2");
        *s
    };

    // The CMake-built reference, for cross-checking the freshly built variants.
    let reference = pair().c_memchra2;

    let mut libs = Vec::new();
    for (flag, tag) in &variants {
        let p = build_variant(flag, tag);
        let lib = unsafe { Library::new(&p).unwrap_or_else(|e| panic!("dlopen {p:?}: {e}")) };
        let f: Memchra2 = unsafe {
            let s: Symbol<Memchra2> = lib.get(b"memchra2\0").expect("memchra2");
            *s
        };
        libs.push((*flag, lib, f));
    }

    let mut rng = Rng::new(0xC0FFEE);
    let mut cases: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (0x3F80_0000u32 as i32, -1, 255, 256),
        (0x4479_FFFFu32 as i32, 0, 0, 0),
        (0x447A_0000u32 as i32, 0, 0, 0),
        (0x7F80_0000u32 as i32, 0, 0, 0),
        (0x7FC0_0000u32 as i32, 0, 0, 0),
        (0xFF80_0000u32 as i32, 0, 0, 0),
    ];
    for _ in 0..50_000 {
        cases.push((
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ));
    }

    for (a, b, c, d) in cases {
        let rv = unsafe { rust_fn(a, b, c, d) };
        let refv = unsafe { reference(a, b, c, d) };
        assert_eq!(
            refv, rv,
            "CMake C build vs Rust diverge on ({a},{b},{c},{d})"
        );
        for (flag, _lib, f) in &libs {
            let cv = unsafe { f(a, b, c, d) };
            assert_eq!(
                cv, rv,
                "C built with {flag} vs Rust diverge on ({a},{b},{c},{d}): C={cv} Rust={rv}"
            );
        }
    }
}
