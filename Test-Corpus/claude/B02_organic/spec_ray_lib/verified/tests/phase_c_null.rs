//! Phase C — rows 36, 37 and 43 of `ERRORS.md`: null-pointer behaviour.
//!
//! The C library never checks a pointer, so a null `out` / `B` is undefined
//! behaviour that manifests as a SIGSEGV *only on the code paths that actually
//! dereference it*.  `c2RaytoCircle` / `c2RaytoAABB` store into `*out` only on
//! the hit path, `c2RaytoCapsule` stores unconditionally, and `c2CastRay` loads
//! `*B` unconditionally (but not when `typeB` is out of range).
//!
//! Each case is therefore run in a forked child process (a re-exec of this test
//! binary) once against the C `.so` and once against the Rust `.so`, and the
//! two exit statuses — including the fatal signal number — must be identical.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const CASES: &[(&str, bool)] = &[
    // (case name, expected to be killed by a signal)
    ("circle_out_null_miss", false),
    ("circle_out_null_hit", true),
    ("aabb_out_null_miss", false),
    ("aabb_out_null_hit", true),
    ("capsule_out_null", true),
    ("castray_out_null_miss", false),
    ("castray_out_null_hit", true),
    ("castray_b_null", true),
    ("castray_b_and_out_null_invalid_type", false),
    ("spec_ray_null_miss", false),
    ("spec_ray_null_hit", true),
];

fn run_case(api: &Api, case: &str) {
    let null_out = std::ptr::null_mut::<C2Raycast>();
    match case {
        "circle_out_null_miss" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let circ = C2Circle {
                p: v(0.0, 100.0),
                r: 1.0,
            };
            let rc = unsafe { (api.c2RaytoCircle)(ray, circ, null_out) };
            println!("rc={rc}");
        }
        "circle_out_null_hit" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let circ = C2Circle {
                p: v(5.0, 0.0),
                r: 1.0,
            };
            let rc = unsafe { (api.c2RaytoCircle)(ray, circ, null_out) };
            println!("rc={rc}");
        }
        "aabb_out_null_miss" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let b = C2AABB {
                min: v(0.0, 100.0),
                max: v(1.0, 101.0),
            };
            let rc = unsafe { (api.c2RaytoAABB)(ray, b, null_out) };
            println!("rc={rc}");
        }
        "aabb_out_null_hit" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let b = C2AABB {
                min: v(4.0, -1.0),
                max: v(6.0, 1.0),
            };
            let rc = unsafe { (api.c2RaytoAABB)(ray, b, null_out) };
            println!("rc={rc}");
        }
        "capsule_out_null" => {
            // no hit anywhere near, but c2RaytoCapsule writes *out first
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let cap = C2Capsule {
                a: v(0.0, 100.0),
                b: v(0.0, 110.0),
                r: 1.0,
            };
            let rc = unsafe { (api.c2RaytoCapsule)(ray, cap, null_out) };
            println!("rc={rc}");
        }
        "castray_out_null_miss" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let circ = C2Circle {
                p: v(0.0, 100.0),
                r: 1.0,
            };
            let rc = unsafe {
                (api.c2CastRay)(
                    ray,
                    &circ as *const C2Circle as *const c_void,
                    C2_TYPE_CIRCLE,
                    null_out,
                )
            };
            println!("rc={rc}");
        }
        "castray_out_null_hit" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let circ = C2Circle {
                p: v(5.0, 0.0),
                r: 1.0,
            };
            let rc = unsafe {
                (api.c2CastRay)(
                    ray,
                    &circ as *const C2Circle as *const c_void,
                    C2_TYPE_CIRCLE,
                    null_out,
                )
            };
            println!("rc={rc}");
        }
        "castray_b_null" => {
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let mut out = sentinel();
            let rc = unsafe {
                (api.c2CastRay)(ray, std::ptr::null(), C2_TYPE_CIRCLE, &mut out)
            };
            println!("rc={rc}");
        }
        "castray_b_and_out_null_invalid_type" => {
            // typeB outside {0,1,2}: nothing is dereferenced at all
            let ray = C2Ray {
                p: v(0.0, 0.0),
                d: v(1.0, 0.0),
                t: 10.0,
            };
            let rc = unsafe { (api.c2CastRay)(ray, std::ptr::null(), 99, null_out) };
            println!("rc={rc}");
        }
        "spec_ray_null_miss" => {
            // mouse point and ray origin on the far side of the circle
            let rc = unsafe {
                (api.spec_ray)(null_out, 1.0, 100.0, 0.0, 0.0, 1.0, 0.0, 100.0)
            };
            println!("rc={rc}");
        }
        "spec_ray_null_hit" => {
            let rc = unsafe {
                (api.spec_ray)(null_out, 20.0, 0.0, 5.0, 0.0, 1.0, -5.0, 0.0)
            };
            println!("rc={rc}");
        }
        other => panic!("unknown null-pointer case `{other}`"),
    }
}

/// Child entry point: runs one case in this process and exits 0 if it survives.
#[test]
fn null_child_runner() {
    let Ok(spec) = std::env::var("DIFF_NULL_CASE") else {
        // normal test run: nothing to do
        return;
    };
    let (which, case) = spec.split_once(':').expect("DIFF_NULL_CASE=<c|r>:<case>");
    let api = match which {
        "c" => c_api(),
        "r" => rust_api(),
        _ => panic!("bad library selector {which}"),
    };
    run_case(api, case);
    println!("SURVIVED");
    // exit(0) explicitly so that the harness cannot report anything else
    std::process::exit(0);
}

fn child_outcome(which: &str, case: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", "null_child_runner", "--nocapture", "--test-threads=1"])
        .env("DIFF_NULL_CASE", format!("{which}:{case}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn child");
    (status.code(), status.signal())
}

#[test]
fn rows36_37_43_null_pointer_behaviour_matches() {
    // Only meaningful in an unoptimised build: with optimisations LLVM is
    // allowed to turn a null dereference into an arbitrary trap.
    for (case, expect_signal) in CASES {
        let c = child_outcome("c", case);
        let r = child_outcome("r", case);
        assert_eq!(
            c, r,
            "case `{case}`: C exited {c:?} but RUST exited {r:?} (code, signal)"
        );
        if *expect_signal {
            assert_eq!(
                c.1,
                Some(11),
                "case `{case}`: expected SIGSEGV from the C library, got {c:?}"
            );
        } else {
            assert_eq!(
                c,
                (Some(0), None),
                "case `{case}`: expected a clean exit from the C library, got {c:?}"
            );
        }
        println!("null case `{case}`: C {c:?} == RUST {r:?}");
    }
}
