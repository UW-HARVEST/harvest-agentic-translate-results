//! Isolated probe: what does the C do when `c2GJK` is handed a `C2_TYPE`
//! outside {0,1,2}?  `c2MakeProxy` is then a no-op, so `c2Proxy pA;` stays
//! **uninitialised** and the C reads indeterminate stack memory
//! (`pA.count`, `pA.verts[0]`).  Run in its own test binary so that a crash
//! cannot take the real suites down.

#![allow(non_snake_case)]
mod common;
use common::*;

use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct Buf([u8; 32]);

fn put<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0xA5; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

#[test]
#[ignore = "reads indeterminate stack memory in the C (see ERRORS.md); \
            informational only, run with --ignored"]
fn cub1_gjk_out_of_range_type_is_indeterminate_in_c() {
    let (c, r) = libs();
    let a = put(&c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    });
    let b = put(&c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    });
    for &bad in [3i32, -1].iter() {
        for api in [c, r] {
            let mut oa = c2v::default();
            let mut ob = c2v::default();
            let mut it: c_int = 0;
            let d = unsafe {
                (api.c2GJK)(
                    a.0.as_ptr() as *const c_void,
                    bad,
                    std::ptr::null(),
                    b.0.as_ptr() as *const c_void,
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &mut oa,
                    &mut ob,
                    1,
                    &mut it,
                    std::ptr::null_mut(),
                )
            };
            println!(
                "{} typeA={bad}: dist={d:?} outA={oa:?} outB={ob:?} iters={it}",
                api.tag
            );
        }
    }

    // Demonstrate that the C's answer is *indeterminate*, not merely different:
    // dirtying the stack with an unrelated valid call changes it.
    let big = put(&c2Capsule {
        a: c2v { x: 1.0e20, y: -2.0e20 },
        b: c2v { x: -3.0e20, y: 4.0e20 },
        r: 5.0e20,
    });
    for round in 0..2 {
        if round == 1 {
            let mut oa = c2v::default();
            let mut ob = c2v::default();
            unsafe {
                (c.c2GJK)(
                    big.0.as_ptr() as *const c_void,
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    b.0.as_ptr() as *const c_void,
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &mut oa,
                    &mut ob,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
        }
        let mut oa = c2v::default();
        let mut ob = c2v::default();
        let d = unsafe {
            (c.c2GJK)(
                a.0.as_ptr() as *const c_void,
                7,
                std::ptr::null(),
                b.0.as_ptr() as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &mut oa,
                &mut ob,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        println!("C typeA=7 round={round} (stack dirtied={}): dist={d:?} outA={oa:?} outB={ob:?}", round == 1);
    }
}
