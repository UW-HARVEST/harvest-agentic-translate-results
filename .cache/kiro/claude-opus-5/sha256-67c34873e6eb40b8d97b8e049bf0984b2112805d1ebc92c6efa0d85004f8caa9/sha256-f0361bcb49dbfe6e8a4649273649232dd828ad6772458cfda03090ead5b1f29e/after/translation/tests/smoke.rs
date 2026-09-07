//! Smoke test: both `.so`s load and every one of the 20 symbols resolves.
mod harness;

use harness::*;

#[test]
fn both_libraries_load_all_20_symbols() {
    let p = load();
    // Resolving happens in `Lib::open`; a missing symbol panics there.
    // Do one trivial call per symbol to prove the ABI is wired up.
    for l in [&p.c, &p.r] {
        unsafe {
            let v = (l.c2V)(1.0, 2.0);
            assert_eq!((v.x, v.y), (1.0, 2.0), "{} c2V", l.name);
            let a = C2v { x: 1.0, y: 5.0 };
            let b = C2v { x: 3.0, y: 2.0 };
            assert_eq!(bits32((l.c2Maxv)(a, b).x), bits32(3.0), "{}", l.name);
            assert_eq!(bits32((l.c2Minv)(a, b).y), bits32(2.0), "{}", l.name);
            let _ = (l.c2Clampv)(a, b, b);
            let _ = (l.c2Sub)(a, b);
            assert_eq!(bits32((l.c2Dot)(a, b)), bits32(13.0), "{}", l.name);
            let ca = C2Circle {
                p: C2v { x: 0.0, y: 0.0 },
                r: 1.0,
            };
            let bb = C2Aabb {
                min: C2v { x: -1.0, y: -1.0 },
                max: C2v { x: 1.0, y: 1.0 },
            };
            assert_eq!((l.c2CircletoCircle)(ca, ca), 1, "{}", l.name);
            assert_eq!((l.c2CircletoAABB)(ca, bb), 1, "{}", l.name);
            assert_eq!((l.c2AABBtoAABB)(bb, bb), 1, "{}", l.name);
            let _ = (l.f2)(
                &ca as *const _ as *const _,
                C2_TYPE_CIRCLE,
                &bb as *const _ as *const _,
                C2_TYPE_AABB,
            );
            assert_eq!((l.f3)(7, 3), 2, "{}", l.name);
            let mut st = CnRnd { state: [1, 2] };
            let _ = (l.f4)(&mut st);
            assert_eq!((l.f5)(1), 0x8000, "{}", l.name);
            let _ = (l.f7)(4096, 2, 16);
            let z = LmVec2 { x: 0.0, y: 0.0 };
            let _ = (l.f9)(z, z, z, z);
            assert_eq!(bits32((l.f10)(0x3c00)), bits32(1.0), "{}", l.name);
            let _ = l.call_triple(Triple::F11, [30.0, 0.5, 0.5]);
            let _ = l.call_triple(Triple::F12, [30.0, 0.5, 0.5]);
            let _ = l.call_triple(Triple::F13, [0.75, 0.5, 0.25]);
            let _ = l.call_agglom(&AgglomArgs::default());
        }
    }
}
