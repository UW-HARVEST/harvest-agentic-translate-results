//! Harness smoke test: both `.so`s load and every one of the 20 exported
//! symbols is resolvable and callable through the FFI boundary.

mod common;

use common::*;

#[test]
fn both_libraries_load_all_20_symbols() {
    let p = pair();
    // Loading resolves every symbol eagerly, so reaching here already proves
    // symbol parity at the dynamic-linker level.
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}

#[test]
fn smoke_all_entry_points() {
    let p = pair();

    same("c2V", (1.5f32, -2.5f32), unsafe { (p.c.c2V)(1.5, -2.5) }, unsafe {
        (p.rs.c2V)(1.5, -2.5)
    });

    let a = C2v { x: 1.0, y: 2.0 };
    let b = C2v { x: 3.0, y: -1.0 };
    same("c2Maxv", (), unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
        (p.rs.c2Maxv)(a, b)
    });
    same("c2Minv", (), unsafe { (p.c.c2Minv)(a, b) }, unsafe {
        (p.rs.c2Minv)(a, b)
    });
    same("c2Sub", (), unsafe { (p.c.c2Sub)(a, b) }, unsafe {
        (p.rs.c2Sub)(a, b)
    });
    same("c2Dot", (), unsafe { (p.c.c2Dot)(a, b) }, unsafe {
        (p.rs.c2Dot)(a, b)
    });
    let lo = C2v { x: 0.0, y: 0.0 };
    let hi = C2v { x: 1.0, y: 1.0 };
    same("c2Clampv", (), unsafe { (p.c.c2Clampv)(b, lo, hi) }, unsafe {
        (p.rs.c2Clampv)(b, lo, hi)
    });

    let ca = C2Circle { p: a, r: 1.0 };
    let cb = C2Circle { p: b, r: 1.0 };
    let bx = C2Aabb { min: lo, max: hi };
    same(
        "c2CircletoCircle",
        (),
        unsafe { (p.c.c2CircletoCircle)(ca, cb) },
        unsafe { (p.rs.c2CircletoCircle)(ca, cb) },
    );
    same(
        "c2CircletoAABB",
        (),
        unsafe { (p.c.c2CircletoAABB)(ca, bx) },
        unsafe { (p.rs.c2CircletoAABB)(ca, bx) },
    );
    same(
        "c2AABBtoAABB",
        (),
        unsafe { (p.c.c2AABBtoAABB)(bx, bx) },
        unsafe { (p.rs.c2AABBtoAABB)(bx, bx) },
    );

    let cptr = &ca as *const C2Circle as *const std::ffi::c_void;
    let aptr = &bx as *const C2Aabb as *const std::ffi::c_void;
    same(
        "f2",
        (),
        unsafe { (p.c.f2)(cptr, C2_TYPE_CIRCLE, aptr, C2_TYPE_AABB) },
        unsafe { (p.rs.f2)(cptr, C2_TYPE_CIRCLE, aptr, C2_TYPE_AABB) },
    );

    same("f3", (7, -2), unsafe { (p.c.f3)(7, -2) }, unsafe {
        (p.rs.f3)(7, -2)
    });

    let mut sc = CnRnd { state: [1, 2] };
    let mut sr = CnRnd { state: [1, 2] };
    same(
        "f4",
        (),
        unsafe { (p.c.f4)(&mut sc) },
        unsafe { (p.rs.f4)(&mut sr) },
    );
    same("f4/state", (), sc.state, sr.state);

    same("f5", 0xABCDu32, unsafe { (p.c.f5)(0xABCD) }, unsafe {
        (p.rs.f5)(0xABCD)
    });
    same("f7", (), unsafe { (p.c.f7)(4096, 2, 16) }, unsafe {
        (p.rs.f7)(4096, 2, 16)
    });

    let v = LmVec2 { x: 0.25, y: 0.75 };
    let p1 = LmVec2 { x: 0.0, y: 0.0 };
    let p2 = LmVec2 { x: 1.0, y: 0.0 };
    let p3 = LmVec2 { x: 0.0, y: 1.0 };
    same("f9", (), unsafe { (p.c.f9)(p1, p2, p3, v) }, unsafe {
        (p.rs.f9)(p1, p2, p3, v)
    });

    same("f10", 0x3C00u16, unsafe { (p.c.f10)(0x3C00) }, unsafe {
        (p.rs.f10)(0x3C00)
    });

    let src = [30.0f32, 0.5, 0.5];
    same("f11", src, call_f1x(p.c.f11, src), call_f1x(p.rs.f11, src));
    same("f12", src, call_f1x(p.c.f12, src), call_f1x(p.rs.f12, src));
    let rgb = [0.2f32, 0.6, 0.4];
    same("f13", rgb, call_f1x(p.c.f13, rgb), call_f1x(p.rs.f13, rgb));

    #[rustfmt::skip]
    let args = (
        1.0f32, 2.0f32, 3.0f32, 0.0f32, 0.0f32, 4.0f32, 4.0f32,
        17i32, 5i32,
        123u64, 456u64,
        0xBEEFu32,
        4096u32, 2u32, 16u32,
        0.0f32, 0.0f32, 1.0f32, 0.0f32, 0.0f32, 1.0f32, 0.3f32, 0.3f32,
        0x3C00u16,
        30.0f32, 0.5f32, 0.5f32,
        200.0f32, 0.5f32, 0.5f32,
        0.2f32, 0.6f32, 0.4f32,
    );
    let cv = unsafe {
        (p.c.agglom)(
            args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
            args.10, args.11, args.12, args.13, args.14, args.15, args.16, args.17, args.18,
            args.19, args.20, args.21, args.22, args.23, args.24, args.25, args.26, args.27,
            args.28, args.29, args.30, args.31, args.32,
        )
    };
    let rv = unsafe {
        (p.rs.agglom)(
            args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
            args.10, args.11, args.12, args.13, args.14, args.15, args.16, args.17, args.18,
            args.19, args.20, args.21, args.22, args.23, args.24, args.25, args.26, args.27,
            args.28, args.29, args.30, args.31, args.32,
        )
    };
    same("agglom", (), cv, rv);
}
