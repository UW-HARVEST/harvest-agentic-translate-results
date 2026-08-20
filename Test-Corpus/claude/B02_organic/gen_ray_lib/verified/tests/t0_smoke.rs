//! Smoke test: both `.so`s load and their exported symbols are callable
//! through the C ABI, plus a diagnostic dump of the one undefined-behaviour
//! path in the library (`c2CastRay` with an out-of-range `C2_TYPE`).
//!
//! The UB probe deliberately makes NO assertion about the returned `int`: at
//! `-O0` the C reaches its epilogue without ever writing `%rax`, so the value
//! is whatever the caller left there (here: the callee's own address, because
//! the test's indirect call is `call *%rax`).  See `ERRORS.md` row E23; the
//! *defined* part of that behaviour is asserted in
//! `tests/t6_castray.rs::e23_e35_out_of_range_enum`.

mod common;
use common::*;

#[test]
fn probe_load_both() {
    let (c, r) = apis();
    eprintln!("loaded {} and {}", c.name, r.name);
    let a = (c.c2V)(1.5, -2.5);
    let b = (r.c2V)(1.5, -2.5);
    assert!(v_eq(a, b), "c2V mismatch {:?} {:?}", a, b);
    eprintln!("c2Dot C={} RUST={}", (c.c2Dot)(a, a), (r.c2Dot)(b, b));
}

#[test]
fn probe_invalid_type() {
    let (c, r) = apis();
    let ray = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let circle = c2Circle {
        p: c2v { x: 5.0, y: 0.0 },
        r: 1.0,
    };
    for ty in [-1i32, 3, 7, 99, i32::MIN, i32::MAX] {
        let mut oc = POISON;
        let mut orr = POISON;
        let rc = unsafe {
            (c.c2CastRay)(
                ray,
                &circle as *const _ as *const std::ffi::c_void,
                ty,
                &mut oc,
            )
        };
        let rr = unsafe {
            (r.c2CastRay)(
                ray,
                &circle as *const _ as *const std::ffi::c_void,
                ty,
                &mut orr,
            )
        };
        eprintln!(
            "ty={ty}: C ret={rc:#x} ({rc}) out={} | RUST ret={rr:#x} ({rr}) out={}",
            fmt_cast(oc),
            fmt_cast(orr)
        );
        eprintln!(
            "   ptr={:#x} c2CastRay_C={:p} c2CastRay_RUST={:p}",
            &circle as *const _ as usize, c.c2CastRay as *const (), r.c2CastRay as *const ()
        );
    }
}
