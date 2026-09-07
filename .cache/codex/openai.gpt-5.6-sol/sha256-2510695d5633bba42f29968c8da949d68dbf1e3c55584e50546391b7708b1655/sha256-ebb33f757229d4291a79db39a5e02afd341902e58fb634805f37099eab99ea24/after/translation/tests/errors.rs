mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::ptr;

#[test]
fn error_e01_invalid_enum_returns_zero_without_dereference() {
    let pair = Pair::load();
    let circle = C2Circle {
        p: v(1.0, -2.0),
        r: 3.0,
    };
    let valid = (&circle as *const C2Circle).cast::<c_void>();
    let mut rng = Rng::new(0xe220_a839_7b1d_cdaf);
    let boundaries = [-1, 3, c_int::MIN, c_int::MAX];

    unsafe {
        for case in 0..1024 {
            let type_b = if case < boundaries.len() {
                boundaries[case]
            } else {
                loop {
                    let candidate = rng.next_u32() as c_int;
                    if !matches!(candidate, 0..=2) {
                        break candidate;
                    }
                }
            };
            let (a, b) = match case % 4 {
                0 => (ptr::null(), ptr::null()),
                1 => (valid, ptr::null()),
                2 => (ptr::null(), valid),
                _ => (valid, valid),
            };
            let c = (pair.c.collided)(a, b, type_b);
            let rust = (pair.rust.collided)(a, b, type_b);
            assert_eq!(c, 0, "E01 case {case}: C returned {c} for type {type_b}");
            assert_i32("E01", case, c, rust);
        }
    }
}

#[test]
fn generic_null_pointer_boundary_matches_observed_process_termination() {
    let executable = std::env::current_exe().expect("current test executable");

    for selector in 0..=2 {
        for null_argument in ["A", "B"] {
            let run = |library: &str| {
                Command::new(&executable)
                    .args(["--exact", "null_probe_child", "--nocapture"])
                    .env("C2_NULL_PROBE_LIBRARY", library)
                    .env("C2_NULL_PROBE_SELECTOR", selector.to_string())
                    .env("C2_NULL_PROBE_ARGUMENT", null_argument)
                    .output()
                    .unwrap_or_else(|error| panic!("failed to run null probe: {error}"))
                    .status
            };

            let c_status = run("c");
            let rust_status = run("rust");
            assert!(
                !c_status.success() && !rust_status.success(),
                "selector {selector}, null {null_argument}: expected both calls to terminate; \
                 C={c_status:?}, Rust={rust_status:?}"
            );
            assert_eq!(
                c_status.signal(),
                rust_status.signal(),
                "selector {selector}, null {null_argument}: termination signal differs; \
                 C={c_status:?}, Rust={rust_status:?}"
            );
        }
    }
}

#[test]
fn null_probe_child() {
    let Ok(library) = std::env::var("C2_NULL_PROBE_LIBRARY") else {
        return;
    };
    let selector: c_int = std::env::var("C2_NULL_PROBE_SELECTOR")
        .expect("selector")
        .parse()
        .expect("integer selector");
    let null_argument = std::env::var("C2_NULL_PROBE_ARGUMENT").expect("argument");
    let path = match library.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown probe library {other}"),
    };

    unsafe {
        let api = Api::load(&path);
        let circle = C2Circle {
            p: v(1.0, 2.0),
            r: 3.0,
        };
        let other_circle = C2Circle {
            p: v(4.0, 5.0),
            r: 6.0,
        };
        let aabb = C2Aabb {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        let capsule = C2Capsule {
            a: v(-1.0, 0.0),
            b: v(1.0, 0.0),
            r: 1.0,
        };
        let a_valid = (&circle as *const C2Circle).cast::<c_void>();
        let b_valid = match selector {
            0 => (&other_circle as *const C2Circle).cast::<c_void>(),
            1 => (&aabb as *const C2Aabb).cast::<c_void>(),
            2 => (&capsule as *const C2Capsule).cast::<c_void>(),
            _ => unreachable!(),
        };
        let (a, b) = match null_argument.as_str() {
            "A" => (ptr::null(), b_valid),
            "B" => (a_valid, ptr::null()),
            other => panic!("unknown null argument {other}"),
        };
        let _ = (api.collided)(a, b, selector);
    }
}
