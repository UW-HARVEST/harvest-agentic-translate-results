//! Harness sanity check: both libraries load, every Phase-A symbol resolves in
//! BOTH, and the struct-passing ABI round-trips.

mod common;
use common::*;

#[test]
fn smoke_00_both_libs_load() {
    let l = libs();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.r_path.display());
}

/// Every symbol from SYMBOLS.md must resolve in BOTH libraries.
#[test]
fn smoke_01_all_symbols_resolve() {
    const NAMES: [&str; 28] = [
        "c2V",
        "c2Dot",
        "c2Len",
        "c2Add",
        "c2Sub",
        "c2Mulvs",
        "c2Div",
        "c2Norm",
        "c2Minv",
        "c2Maxv",
        "c2Skew",
        "c2Absv",
        "c2RaytoCircle",
        "c2AABBtoAABB",
        "c2RaytoAABB",
        "c2CCW90",
        "c2MulmvT",
        "c2AABBtoPoint",
        "c2CircleToPoint",
        "c2RaytoCapsule",
        "c2RotIdentity",
        "c2xIdentity",
        "c2Mulrv",
        "c2MulrvT",
        "c2MulxvT",
        "c2RaytoPoly",
        "c2CastRay",
        "poly_ray",
    ];
    let l = libs();
    for n in NAMES {
        // Type is irrelevant for resolution; `*mut ()` just proves the symbol exists.
        let _ = l.pair::<*mut ()>(n);
    }
    eprintln!("all {} symbols present in both .so files", NAMES.len());
}

/// The `static inline` helpers must NOT be exported by either library.
#[test]
fn smoke_02_static_helpers_not_exported() {
    let l = libs();
    for n in [
        b"c2SignedDistPointToPlane_OneDimensional\0".as_slice(),
        b"c2RayToPlane_OneDimensional\0".as_slice(),
    ] {
        let in_c = unsafe { l.c.get::<*mut ()>(n) }.is_ok();
        let in_r = unsafe { l.r.get::<*mut ()>(n) }.is_ok();
        assert_eq!(
            in_c,
            in_r,
            "export parity mismatch for {}: C={in_c} Rust={in_r}",
            String::from_utf8_lossy(n)
        );
    }
}

/// Basic ABI round-trip: struct-by-value in, struct-by-value out.
#[test]
fn smoke_03_abi_roundtrip() {
    let (cv_c, cv_r) = sym!("c2V", FnVff);
    assert_eq!(vb(cv_c(1.5, -2.5)), vb(cv_r(1.5, -2.5)));
    assert_eq!(vb(cv_c(1.5, -2.5)), vb(c2v { x: 1.5, y: -2.5 }));

    let (dot_c, dot_r) = sym!("c2Dot", FnFvv);
    let a = c2v { x: 3.0, y: 4.0 };
    assert_eq!(fb(dot_c(a, a)), fb(25.0f32));
    assert_eq!(fb(dot_c(a, a)), fb(dot_r(a, a)));

    let (id_c, id_r) = sym!("c2xIdentity", FnX);
    assert_eq!(xb(id_c()), xb(id_r()));
    assert_eq!(xb(id_c()), xb(identity_x()));
}

/// The documented `poly_ray` result must agree, out-params included.
#[test]
fn smoke_04_poly_ray() {
    let (cf, rf) = sym!("poly_ray", FnPolyRay);
    let (mut c1, mut c2) = (dirty(), dirty());
    let (mut r1, mut r2) = (dirty(), dirty());
    let cret = unsafe { cf(&mut c1, &mut c2) };
    let rret = unsafe { rf(&mut r1, &mut r2) };
    eprintln!("C   -> {cret} {c1:?} {c2:?}");
    eprintln!("RS  -> {rret} {r1:?} {r2:?}");
    assert_eq!(cret, rret, "poly_ray return value");
    assert_eq!(cb(c1), cb(r1), "poly_ray cast1");
    assert_eq!(cb(c2), cb(r2), "poly_ray cast2");
}
