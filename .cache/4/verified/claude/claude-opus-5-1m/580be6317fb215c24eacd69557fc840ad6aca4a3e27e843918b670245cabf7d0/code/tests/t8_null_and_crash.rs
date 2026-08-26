//! Phase C rows E25..E29: NULL-pointer behaviour.
//!
//! Some of these rows are *supposed* to crash (the C dereferences the pointer
//! unconditionally).  "Both crash the same way" is the property under test, so
//! those cases run in a re-executed child process and the delivered signal is
//! compared.  The non-crashing rows run in-process.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::ptr;

/// A ray/circle pair that definitely misses (`disc < 0`).
fn missing_circle() -> (c2Ray, c2Circle) {
    (
        c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        },
        c2Circle {
            p: c2v { x: 0.0, y: 100.0 },
            r: 1.0,
        },
    )
}

/// A ray/circle pair that definitely hits at t == 4.
fn hitting_circle() -> (c2Ray, c2Circle) {
    (
        c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 100.0,
        },
        c2Circle {
            p: c2v { x: 5.0, y: 0.0 },
            r: 1.0,
        },
    )
}

fn missing_aabb() -> (c2Ray, c2AABB) {
    (
        c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        },
        c2AABB {
            min: c2v { x: 0.0, y: 100.0 },
            max: c2v { x: 10.0, y: 110.0 },
        },
    )
}

fn hitting_aabb() -> (c2Ray, c2AABB) {
    (
        c2Ray {
            p: c2v { x: -5.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 100.0,
        },
        c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        },
    )
}

/// E25: `out == NULL` on a guaranteed miss - the C never writes, so this is
/// well defined and must return 0 in both libraries without faulting.
#[test]
fn e25_null_out_on_miss() {
    let (c, r) = apis();
    let mut d = Diff::new();

    let (ray, circ) = missing_circle();
    let rc = unsafe { (c.c2RaytoCircle)(ray, circ, ptr::null_mut()) };
    let rr = unsafe { (r.c2RaytoCircle)(ray, circ, ptr::null_mut()) };
    d.ints("E25/circle", || "null out, miss".into(), rc, rr);
    d.check(rc == 0, || format!("expected miss, C returned {rc}"));

    let (ray2, b2) = missing_aabb();
    let rc2 = unsafe { (c.c2RaytoAABB)(ray2, b2, ptr::null_mut()) };
    let rr2 = unsafe { (r.c2RaytoAABB)(ray2, b2, ptr::null_mut()) };
    d.ints("E25/aabb", || "null out, miss".into(), rc2, rr2);
    d.check(rc2 == 0, || format!("expected miss, C returned {rc2}"));

    // ... and through the dispatcher, plus every out-of-range typeB (which
    // never touches `out` at all).
    let buf = shape_bytes_circle(circ);
    for ty in [C2_TYPE_CIRCLE, -1, 3, 77, i32::MIN] {
        let rc3 = unsafe {
            (c.c2CastRay)(ray, buf.as_ptr() as *const c_void, ty, ptr::null_mut())
        };
        let rr3 = unsafe {
            (r.c2CastRay)(ray, buf.as_ptr() as *const c_void, ty, ptr::null_mut())
        };
        if (0..=2).contains(&ty) {
            d.ints("E25/castray", || format!("ty={ty}"), rc3, rr3);
        }
        // out-of-range typeB: the return value is UB (stale %rax); only the
        // absence of a fault and of any write is defined, and both survived.
    }

    // E29 (safe half): gen_ray with a NULL cast1 while the circle misses.
    let args: GenRayArgs = [
        0.0, 0.0, // mp
        -10.0, 0.0, // ray origin
        0.0, 1e6, 1.0, // circle far away -> miss -> cast1 not written
        0.0, 1e6, 1.0, 1e6, 1.0, // capsule far away
        0.0, 1e6, 1.0, 1e6 + 1.0, // aabb far away
    ];
    let mut c2 = POISON;
    let mut c3 = POISON;
    let g_c = unsafe {
        (c.gen_ray)(
            ptr::null_mut(),
            &mut c2,
            &mut c3,
            args[0],
            args[1],
            args[2],
            args[3],
            args[4],
            args[5],
            args[6],
            args[7],
            args[8],
            args[9],
            args[10],
            args[11],
            args[12],
            args[13],
            args[14],
            args[15],
        )
    };
    let mut r2 = POISON;
    let mut r3 = POISON;
    let g_r = unsafe {
        (r.gen_ray)(
            ptr::null_mut(),
            &mut r2,
            &mut r3,
            args[0],
            args[1],
            args[2],
            args[3],
            args[4],
            args[5],
            args[6],
            args[7],
            args[8],
            args[9],
            args[10],
            args[11],
            args[12],
            args[13],
            args[14],
            args[15],
        )
    };
    d.check(
        g_c == g_r && cast_eq(c2, r2) && cast_eq(c3, r3),
        || {
            format!(
                "E29/safe: C ret={} c2={} c3={} | RUST ret={} c2={} c3={}",
                g_c,
                fmt_cast(c2),
                fmt_cast(c3),
                g_r,
                fmt_cast(r2),
                fmt_cast(r3)
            )
        },
    );
    d.check(g_c & 1 == 0, || {
        format!("expected the circle to miss, gen_ray returned {g_c}")
    });
    d.finish("E25 null out on miss paths");
}

// ---------------------------------------------------------------------------
// Crash-parity harness: E26, E27, E28, E29
// ---------------------------------------------------------------------------

const CASES: &[&str] = &[
    "capsule_null_out",   // E26: capsule writes out->n/out->t unconditionally
    "circle_null_out_hit",// E27
    "aabb_null_out_hit",  // E27
    "castray_null_b_0",   // E28
    "castray_null_b_1",   // E28
    "castray_null_b_2",   // E28
    "gen_ray_null_cast2", // E29: capsule cast always writes
    "gen_ray_null_all",   // E29
];

/// The child half: performs one faulting call against one library.
/// A no-op unless `CRASH_CASE`/`CRASH_LIB` are set, so it is harmless when the
/// whole test file is run normally.
#[test]
fn crash_child() {
    let case = match std::env::var("CRASH_CASE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var("CRASH_LIB").unwrap_or_default();
    let (c, r) = apis();
    let api: &Api = if which == "C" { c } else { r };
    eprintln!("child: case={case} lib={}", api.name);

    unsafe {
        match case.as_str() {
            "capsule_null_out" => {
                let ray = c2Ray {
                    p: c2v { x: 0.0, y: 0.0 },
                    d: c2v { x: 1.0, y: 0.0 },
                    t: 10.0,
                };
                // far-away capsule: still writes out->n / out->t first
                let cap = c2Capsule {
                    a: c2v { x: 0.0, y: 1e6 },
                    b: c2v { x: 1.0, y: 1e6 },
                    r: 1.0,
                };
                let v = (api.c2RaytoCapsule)(ray, cap, ptr::null_mut());
                println!("no fault: {v}");
            }
            "circle_null_out_hit" => {
                let (ray, circ) = hitting_circle();
                let v = (api.c2RaytoCircle)(ray, circ, ptr::null_mut());
                println!("no fault: {v}");
            }
            "aabb_null_out_hit" => {
                let (ray, b) = hitting_aabb();
                let v = (api.c2RaytoAABB)(ray, b, ptr::null_mut());
                println!("no fault: {v}");
            }
            "castray_null_b_0" | "castray_null_b_1" | "castray_null_b_2" => {
                let ty: i32 = case.as_bytes()[case.len() - 1] as i32 - b'0' as i32;
                let ray = c2Ray {
                    p: c2v { x: 0.0, y: 0.0 },
                    d: c2v { x: 1.0, y: 0.0 },
                    t: 10.0,
                };
                let mut out = POISON;
                let v = (api.c2CastRay)(ray, ptr::null(), ty, &mut out);
                println!("no fault: {v}");
            }
            "gen_ray_null_cast2" | "gen_ray_null_all" => {
                let mut a = POISON;
                let mut b = POISON;
                let (p1, p3) = if case == "gen_ray_null_all" {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&mut a as *mut c2Raycast, &mut b as *mut c2Raycast)
                };
                let v = (api.gen_ray)(
                    p1,
                    ptr::null_mut(),
                    p3,
                    0.0,
                    0.0,
                    -10.0,
                    0.0,
                    0.0,
                    1e6,
                    1.0,
                    0.0,
                    1e6,
                    1.0,
                    1e6,
                    1.0,
                    0.0,
                    1e6,
                    1.0,
                    1e6 + 1.0,
                );
                println!("no fault: {v}");
            }
            other => panic!("unknown CRASH_CASE {other}"),
        }
    }
}

fn run_child(case: &str, lib: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", "crash_child", "--nocapture", "--test-threads=1"])
        .env("CRASH_CASE", case)
        .env("CRASH_LIB", lib)
        .output()
        .expect("spawn child");
    (st.status.code(), st.status.signal())
}

/// E26 + E27 + E28 + E29: every NULL-pointer dereference must fault
/// identically in both libraries.
#[test]
fn e26_e29_null_deref_crash_parity() {
    for case in CASES {
        let (c_code, c_sig) = run_child(case, "C");
        let (r_code, r_sig) = run_child(case, "RUST");
        eprintln!(
            "{case}: C exit={:?} signal={:?} | RUST exit={:?} signal={:?}",
            c_code, c_sig, r_code, r_sig
        );
        assert_eq!(
            c_sig, r_sig,
            "{case}: signal mismatch (C {:?} vs RUST {:?})",
            c_sig, r_sig
        );
        assert_eq!(
            c_code.is_some(),
            r_code.is_some(),
            "{case}: one library exited normally and the other did not \
             (C code={:?} sig={:?}, RUST code={:?} sig={:?})",
            c_code,
            c_sig,
            r_code,
            r_sig
        );
        // The C dereferences unconditionally on these paths, so a fault is the
        // expected outcome; assert we really exercised a crash.
        assert_eq!(
            c_sig,
            Some(11),
            "{case}: expected SIGSEGV from the C library, got code={:?} sig={:?}",
            c_code,
            c_sig
        );
    }
}
