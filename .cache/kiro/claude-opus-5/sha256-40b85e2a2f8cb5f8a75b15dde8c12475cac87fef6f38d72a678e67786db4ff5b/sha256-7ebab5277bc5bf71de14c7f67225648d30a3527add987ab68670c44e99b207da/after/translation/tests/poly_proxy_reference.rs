//! Reference check for the `C2_TYPE_POLY` proxy path.
//!
//! `c2MakeProxy` has no `C2_TYPE_POLY` case, so `c2GJK`'s `c2Proxy pA/pB`
//! (at `rbp-0x100` / `rbp-0x150` in the reference build) stay uninitialised for a
//! polygon operand. The translation models the only well-defined state: a stack
//! page that has never been written, i.e. all zeros. These tests confirm the C
//! agrees when it is measured on a brand new thread stack, and that
//! `common::scrub_stack` reproduces that state cheaply.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

type GjkFn = unsafe extern "C" fn(
    *const c_void,
    C2_TYPE,
    *const c2x,
    *const c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;

type OmniFn = unsafe extern "C" fn(
    *mut c2Manifold,
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
);

/// The C, run on an untouched thread stack, must agree with the Rust for a
/// polygon operand.
#[test]
fn gjk_poly_operand_on_fresh_stack() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let cf: GjkFn = *l.pair::<GjkFn>("c2GJK").0;
    let rf: GjkFn = *l.pair::<GjkFn>("c2GJK").1;

    let circle = c2Circle {
        p: c2v { x: 3.0, y: 4.0 },
        r: 1.0,
    };
    let mut poly = c2Poly::default();
    poly.count = 4;
    poly.verts[0] = c2v { x: -1.0, y: -1.0 };
    poly.verts[1] = c2v { x: 1.0, y: -1.0 };
    poly.verts[2] = c2v { x: 1.0, y: 1.0 };
    poly.verts[3] = c2v { x: -1.0, y: 1.0 };

    let call = move |f: GjkFn| {
        let mut a = c2v::default();
        let mut b = c2v::default();
        let mut it: c_int = 0;
        let d = unsafe {
            f(
                &circle as *const _ as *const c_void,
                C2_TYPE_CIRCLE,
                std::ptr::null(),
                &poly as *const _ as *const c_void,
                C2_TYPE_POLY,
                std::ptr::null(),
                &mut a,
                &mut b,
                0,
                &mut it,
                std::ptr::null_mut(),
            )
        };
        (d.to_bits(), a, b, it)
    };

    // C on a brand new (zero-filled) stack.
    let c_fresh = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || call(cf))
        .unwrap()
        .join()
        .unwrap();
    // C after `scrub_stack`, on this thread's already dirty stack.
    scrub_stack();
    let c_scrubbed = call(cf);
    let rust = call(rf);

    assert_eq!(
        (c_fresh.0, c_fresh.3),
        (rust.0, rust.3),
        "C on a fresh stack should match Rust: C={c_fresh:?} Rust={rust:?}"
    );
    assert_same("fresh-stack outA", &c_fresh.1, &rust.1);
    assert_same("fresh-stack outB", &c_fresh.2, &rust.2);
    assert_eq!(
        (c_scrubbed.0, c_scrubbed.3),
        (c_fresh.0, c_fresh.3),
        "scrub_stack should reproduce the fresh-stack state"
    );
    assert_same("scrubbed outA", &c_scrubbed.1, &c_fresh.1);
    assert_same("scrubbed outB", &c_scrubbed.2, &c_fresh.2);
}

/// Long-run stability: once the allocator is warm the C agrees on every call.
///
/// `ptr_from_parts` never frees, so the first `omni_manifold` calls in a process
/// make glibc create and grow its arena. `sysmalloc`/`brk` use far more stack
/// than the steady state and write into the region `c2GJK` later reads its
/// uninitialised `c2Proxy pB` from, so those very first calls can disagree with
/// any deterministic implementation. This asserts that after `warm_up` there is
/// no divergence left across a long random run over every type pair.
#[test]
fn long_run_stability() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<OmniFn>("omni_manifold");
    let types = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY];
    let mut rng = Rng::new(31337);
    for i in 0..200_000usize {
        let ta = types[rng.below(4) as usize];
        let tb = types[rng.below(4) as usize];
        let mut v = [0f32; 10];
        for x in v.iter_mut() {
            *x = rng.tame();
        }
        let mut mc = c2Manifold::default();
        let mut mr = c2Manifold::default();
        scrub_stack();
        unsafe { cf(&mut mc, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9]) };
        unsafe { rf(&mut mr, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9]) };
        assert_same_lazy(&mc, &mr, || {
            format!(
                "divergence at call {i}: ta={ta} tb={tb} args={:?}",
                v.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            )
        });
    }
}

/// The same check through `omni_manifold` for the capsule/AABB pairing, which is
/// the only way an ordinary caller reaches the polygon proxy path.
#[test]
fn omni_capsule_aabb_on_fresh_stack() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let cf: OmniFn = *l.pair::<OmniFn>("omni_manifold").0;
    let rf: OmniFn = *l.pair::<OmniFn>("omni_manifold").1;

    let cases: [([f32; 5], [f32; 5]); 4] = [
        (
            [0.0, -0.5, 1.0, 0.5, 0.75],
            [-0.25, 0.25, 1.25, 1.5, 0.5],
        ),
        ([-1.0, -0.5, 1.0, 0.5, 0.75], [-0.25, 0.25, 1.25, 1.5, 0.5]),
        ([0.0, 0.0, 0.0, 0.0, 1.0], [-1.0, -1.0, 1.0, 1.0, 0.0]),
        ([3.0, 3.0, 4.0, 4.0, 0.25], [-1.0, -1.0, 1.0, 1.0, 0.0]),
    ];

    for (i, (a, b)) in cases.into_iter().enumerate() {
        let seed = c2Manifold {
            count: 0x5555_5555,
            depths: [-1.5, 2.5],
            contact_points: [c2v { x: 3.0, y: 4.0 }, c2v { x: 5.0, y: 6.0 }],
            n: c2v { x: 7.0, y: 8.0 },
        };
        let run = move |f: OmniFn| {
            let mut m = seed;
            unsafe {
                f(
                    &mut m,
                    C2_TYPE_CAPSULE,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    a[4],
                    C2_TYPE_AABB,
                    b[0],
                    b[1],
                    b[2],
                    b[3],
                    b[4],
                )
            };
            m
        };
        let c_fresh = std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(move || run(cf))
            .unwrap()
            .join()
            .unwrap();
        scrub_stack();
        let c_scrubbed = run(cf);
        let rust = run(rf);
        assert_same_lazy(&c_fresh, &rust, || format!("case {i}: fresh-stack C vs Rust"));
        assert_same_lazy(&c_fresh, &c_scrubbed, || format!("case {i}: scrub_stack vs fresh stack"));
    }
}
