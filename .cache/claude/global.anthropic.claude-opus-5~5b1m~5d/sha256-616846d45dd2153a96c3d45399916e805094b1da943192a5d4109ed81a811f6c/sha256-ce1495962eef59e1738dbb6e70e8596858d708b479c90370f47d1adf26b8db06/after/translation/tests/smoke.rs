//! Harness smoke test: both `.so`s load and every symbol is callable.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_all_symbols_resolve() {
    let a = apis();
    eprintln!("C   : {}", a.c.path.display());
    for r in &a.rust {
        eprintln!("Rust: {} -> {}", r.name, r.path.display());
    }
    assert!(!a.rust.is_empty());

    // one call through every one of the 12 exported symbols
    for api in std::iter::once(&a.c).chain(a.rust.iter()) {
        let v = (api.c2V)(1.0, 2.0);
        assert_eq!(vbits(v), (1.0f32.to_bits(), 2.0f32.to_bits()));
        let _ = (api.c2Mulvs)(v, 3.0);
        let _ = (api.c2Maxv)(v, v);
        let _ = (api.c2Minv)(v, v);
        let _ = (api.c2Clampv)(v, v, v);
        let _ = (api.c2Sub)(v, v);
        let _ = (api.c2Dot)(v, v);
        let ci = C2Circle { p: v, r: 1.0 };
        let _ = (api.c2CircletoCircle)(ci, ci);
        let _ = (api.c2CircletoAABB)(ci, C2Aabb { min: v, max: v });
        let _ = (api.c2CircletoCapsule)(
            ci,
            C2Capsule {
                a: v,
                b: C2v { x: 3.0, y: 4.0 },
                r: 1.0,
            },
        );
        let _ = unsafe {
            (api.c2Collided)(
                (&raw const ci).cast(),
                (&raw const ci).cast(),
                C2_TYPE_CIRCLE,
            )
        };
        let _ = (api.circle_collide)(0.0, 0.0, 1.0);
    }
}

#[test]
fn circle_collide_reference_values() {
    // Hard-coded shapes in circle_collide:
    //   circle  : centre (-70, 0)     r = 20
    //   aabb    : (-40,-40) .. (-15,-15)
    //   capsule : (-40,40)..(-20,100) r = 10
    let cases: &[(f32, f32, f32)] = &[
        (0.0, 0.0, 1.0),
        (-70.0, 0.0, 1.0),      // bit 0
        (-27.0, -27.0, 1.0),    // bit 1
        (-30.0, 60.0, 1.0),     // bit 2
        (-50.0, 0.0, 40.0),     // bits 0+1
        (-40.0, 20.0, 30.0),    // bits 1+2 maybe
        (-45.0, 20.0, 60.0),    // all
    ];
    for &(x, y, r) in cases {
        diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
    }
}
