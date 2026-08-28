//! Diagnostic probe (not a correctness gate): what does the C `c2CastRay`
//! actually return when `typeB` is out of range?
//!
//! `c2CastRay` has no `default:` label and no `return` after the `switch`, so
//! falling off the end is UB. At `-O0` gcc emits `jmp <epilogue>` with `%eax`
//! never written, i.e. the return value is whatever the *caller* happened to
//! leave in `eax`. This test prints the observed values so the Rust side can be
//! judged against reality instead of a guess.

mod common;

use common::*;
use std::ffi::c_void;

#[test]
fn probe_castray_out_of_range() {
    let p = load();
    let a = c2Ray {
        p: v(-5.0, 0.0),
        d: v(1.0, 0.0),
        t: 100.0,
    };
    let ci = c2Circle { p: v(0.0, 0.0), r: 2.0 };
    for &ty in &[3i32, 4, 5, -1, -2, 100, i32::MAX, i32::MIN, 0x7fff_ffff] {
        let mut co = sentinel();
        let mut ro = sentinel();
        let cr = unsafe {
            (p.c.c2CastRay)(a, (&raw const ci) as *const c_void, ty, &mut co)
        };
        let rr = unsafe {
            (p.r.c2CastRay)(a, (&raw const ci) as *const c_void, ty, &mut ro)
        };
        println!(
            "typeB={:<12} C ret={:<12} ({:#x})  Rust ret={:<12} ({:#x})  Cout={} Rout={}  Bptr={:#x}",
            ty,
            cr,
            cr as u32,
            rr,
            rr as u32,
            rcs(&co),
            rcs(&ro),
            (&raw const ci) as usize
        );
    }
    // Repeat after a *known* preceding call so `eax` holds a known value.
    for &prev_ty in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let mut co = sentinel();
        let pre = unsafe {
            (p.c.c2CastRay)(a, (&raw const ci) as *const c_void, prev_ty, &mut co)
        };
        let mut co2 = sentinel();
        let post = unsafe {
            (p.c.c2CastRay)(a, (&raw const ci) as *const c_void, 7, &mut co2)
        };
        println!("C: after ret={pre} the out-of-range call returned {post}");
    }
}
