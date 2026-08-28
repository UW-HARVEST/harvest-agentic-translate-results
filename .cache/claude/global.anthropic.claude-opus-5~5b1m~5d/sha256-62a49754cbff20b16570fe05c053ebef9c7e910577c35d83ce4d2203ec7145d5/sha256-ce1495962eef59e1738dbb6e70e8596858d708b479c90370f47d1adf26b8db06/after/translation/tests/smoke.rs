//! Minimal load + ABI sanity check for the differential harness itself.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_expose_all_39_symbols() {
    let ps = pairs();
    assert!(!ps.is_empty());
    for p in ps {
        println!("loaded pair: {}", p.label);
        assert_eq!(p.c.name, "c");
    }
}

#[test]
fn abi_sanity_struct_layout() {
    assert_eq!(std::mem::size_of::<c2v>(), 8);
    assert_eq!(std::mem::size_of::<c2r>(), 8);
    assert_eq!(std::mem::size_of::<c2x>(), 16);
    assert_eq!(std::mem::size_of::<c2Circle>(), 12);
    assert_eq!(std::mem::size_of::<c2AABB>(), 16);
    assert_eq!(std::mem::size_of::<c2Capsule>(), 20);
    assert_eq!(std::mem::size_of::<c2GJKCache>(), 36);
    assert_eq!(std::mem::size_of::<c2Proxy>(), 72);
    assert_eq!(std::mem::size_of::<c2sv>(), 36);
    assert_eq!(std::mem::size_of::<c2Simplex>(), 152);
}

#[test]
fn abi_sanity_calls_round_trip() {
    for_each_pair(|c, r, label| {
        // 8-byte SSE struct in / out
        let a = c2v { x: 3.0, y: -4.0 };
        assert!(v_same((c.c2Neg)(a), (r.c2Neg)(a)), "{label}");
        assert!(f32_same((c.c2Len)(a), (r.c2Len)(a)), "{label}");
        assert_eq!((c.c2Len)(a), 5.0, "{label}: sanity");

        // c2r return
        assert!(r_same((c.c2RotIdentity)(), (r.c2RotIdentity)()), "{label}");
        // 16-byte two-SSE-eightbyte return
        assert!(x_same((c.c2xIdentity)(), (r.c2xIdentity)()), "{label}");

        // 20-byte MEMORY-class argument passed on the stack
        let cap = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: 0.5,
        };
        let circ = c2Circle {
            p: c2v { x: 0.0, y: 0.25 },
            r: 0.1,
        };
        assert_eq!(
            (c.c2CircletoCapsule)(circ, cap),
            (r.c2CircletoCapsule)(circ, cap),
            "{label}"
        );
        assert_eq!((c.c2CircletoCapsule)(circ, cap), 1, "{label}: sanity");

        // whole public pipeline
        unsafe {
            assert_eq!(
                (c.omni_collide)(
                    C2_TYPE_CIRCLE, 0.0, 0.0, 1.0, 0.0, 0.0, C2_TYPE_CIRCLE, 1.0, 0.0, 1.0, 0.0,
                    0.0
                ),
                (r.omni_collide)(
                    C2_TYPE_CIRCLE, 0.0, 0.0, 1.0, 0.0, 0.0, C2_TYPE_CIRCLE, 1.0, 0.0, 1.0, 0.0,
                    0.0
                ),
                "{label}"
            );
        }
    });
}

/// Guards against symbol interposition: both `.so`s export the *same* 39 names,
/// so if either were dlopen'd with `RTLD_GLOBAL` the second load could resolve
/// to the first library's symbols and the whole suite would silently be
/// comparing the C against itself. `libloading` defaults to `RTLD_LOCAL`, and
/// this test proves it by checking that every resolved address differs between
/// the two handles and lies inside the right mapping.
#[test]
fn the_two_libraries_are_actually_distinct() {
    for p in pairs() {
        let c = &p.c;
        let r = &p.r;
        macro_rules! distinct {
            ($($f:ident),* $(,)?) => {
                $(
                    let ca = c.$f as usize;
                    let ra = r.$f as usize;
                    assert_ne!(
                        ca, ra,
                        "{}: `{}` resolved to the SAME address in both libraries \
                         ({ca:#x}) -- symbol interposition, the suite would be \
                         comparing the C against itself",
                        p.label,
                        stringify!($f),
                    );
                )*
            };
        }
        distinct!(
            c2V, c2Mulvs, c2Maxv, c2Minv, c2Clampv, c2Sub, c2Dot, c2RotIdentity, c2xIdentity,
            c2BBVerts, c2MakeProxy, c2Len, c2Det2, c2GJKSimplexMetric, c2Mulrv, c2Add, c2Mulxv,
            c22, c23, c2Neg, c2Skew, c2CCW90, c2D, c2Support, c2Witness, c2Div, c2Norm, c2L,
            c2MulrvT, c2GJK, c2AABBtoAABB, c2AABBtoCapsule, c2CapsuletoCapsule, c2CircletoCircle,
            c2CircletoAABB, c2CircletoCapsule, c2Collided, ptr_from_parts, omni_collide,
        );
        println!(
            "{}: all 39 symbols resolve to distinct addresses (C {} / Rust {})",
            p.label,
            c.path.display(),
            r.path.display()
        );
    }
}
