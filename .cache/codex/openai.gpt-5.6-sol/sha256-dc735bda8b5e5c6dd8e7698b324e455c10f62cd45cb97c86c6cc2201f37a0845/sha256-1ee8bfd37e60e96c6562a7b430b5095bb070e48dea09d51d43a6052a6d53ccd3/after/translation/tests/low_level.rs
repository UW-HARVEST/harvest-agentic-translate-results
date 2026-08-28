mod common;

use common::*;

fn simplex_with_points(a: V, b: V, c: V, count: i32) -> Simplex {
    let mut simplex = Simplex::default();
    simplex.a.p = a;
    simplex.a.sA = V {
        x: a.x + 1.0,
        y: a.y - 2.0,
    };
    simplex.a.sB = V {
        x: a.x - 3.0,
        y: a.y + 4.0,
    };
    simplex.a.iA = 3;
    simplex.a.iB = 4;
    simplex.b.p = b;
    simplex.b.sA = V {
        x: b.x + 1.0,
        y: b.y - 2.0,
    };
    simplex.b.sB = V {
        x: b.x - 3.0,
        y: b.y + 4.0,
    };
    simplex.b.iA = 5;
    simplex.b.iB = 6;
    simplex.c.p = c;
    simplex.c.sA = V {
        x: c.x + 1.0,
        y: c.y - 2.0,
    };
    simplex.c.sB = V {
        x: c.x - 3.0,
        y: c.y + 4.0,
    };
    simplex.c.iA = 7;
    simplex.c.iB = 8;
    simplex.div = 1.0;
    simplex.count = count;
    simplex
}

#[test]
fn all_dynamic_symbols_load() {
    let _ = apis();
}

#[test]
fn vector_transform_and_scalar_functions_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x2d35_8dcc_aa6c_78a5);

    unsafe {
        assert_same(
            "c2RotIdentity",
            &(c.c2RotIdentity)(),
            &(rust.c2RotIdentity)(),
        );
        assert_same("c2xIdentity", &(c.c2xIdentity)(), &(rust.c2xIdentity)());

        for case in 0..512 {
            let a = rng.v();
            let b = rng.v();
            let scalar = rng.f32();
            let divisor = if scalar == 0.0 { 1.0 } else { scalar };
            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            let value = match case % 3 {
                0 => V {
                    x: lo.x - rng.positive(),
                    y: hi.y + rng.positive(),
                },
                1 => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: (lo.y + hi.y) * 0.5,
                },
                _ => rng.v(),
            };
            let rotation = if case % 4 == 0 {
                R { c: 1.0, s: 0.0 }
            } else {
                R {
                    c: rng.f32() / 100.0,
                    s: rng.f32() / 100.0,
                }
            };
            let transform = X {
                p: rng.v(),
                r: rotation,
            };

            assert_same(
                &format!("c2V case {case}"),
                &(c.c2V)(a.x, a.y),
                &(rust.c2V)(a.x, a.y),
            );
            assert_same(
                &format!("c2Mulvs case {case}"),
                &(c.c2Mulvs)(a, scalar),
                &(rust.c2Mulvs)(a, scalar),
            );
            assert_same(
                &format!("c2Maxv case {case}"),
                &(c.c2Maxv)(a, b),
                &(rust.c2Maxv)(a, b),
            );
            assert_same(
                &format!("c2Maxv equality case {case}"),
                &(c.c2Maxv)(a, a),
                &(rust.c2Maxv)(a, a),
            );
            assert_same(
                &format!("c2Minv case {case}"),
                &(c.c2Minv)(a, b),
                &(rust.c2Minv)(a, b),
            );
            assert_same(
                &format!("c2Minv equality case {case}"),
                &(c.c2Minv)(a, a),
                &(rust.c2Minv)(a, a),
            );
            assert_same(
                &format!("c2Clampv case {case}"),
                &(c.c2Clampv)(value, lo, hi),
                &(rust.c2Clampv)(value, lo, hi),
            );
            assert_same(
                &format!("c2Sub case {case}"),
                &(c.c2Sub)(a, b),
                &(rust.c2Sub)(a, b),
            );
            assert_f32(
                &format!("c2Dot case {case}"),
                (c.c2Dot)(a, b),
                (rust.c2Dot)(a, b),
            );
            assert_f32(&format!("c2Len case {case}"), (c.c2Len)(a), (rust.c2Len)(a));
            assert_f32(
                &format!("c2Det2 case {case}"),
                (c.c2Det2)(a, b),
                (rust.c2Det2)(a, b),
            );
            assert_f32(
                &format!("c2Det2 collinear case {case}"),
                (c.c2Det2)(a, a),
                (rust.c2Det2)(a, a),
            );
            assert_same(
                &format!("c2Mulrv case {case}"),
                &(c.c2Mulrv)(rotation, a),
                &(rust.c2Mulrv)(rotation, a),
            );
            assert_same(
                &format!("c2Add case {case}"),
                &(c.c2Add)(a, b),
                &(rust.c2Add)(a, b),
            );
            assert_same(
                &format!("c2Mulxv case {case}"),
                &(c.c2Mulxv)(transform, a),
                &(rust.c2Mulxv)(transform, a),
            );
            let identity = X {
                p: V::default(),
                r: R { c: 1.0, s: 0.0 },
            };
            assert_same(
                &format!("c2Mulxv identity case {case}"),
                &(c.c2Mulxv)(identity, a),
                &(rust.c2Mulxv)(identity, a),
            );
            assert_same(
                &format!("c2Neg case {case}"),
                &(c.c2Neg)(a),
                &(rust.c2Neg)(a),
            );
            assert_same(
                &format!("c2Skew case {case}"),
                &(c.c2Skew)(a),
                &(rust.c2Skew)(a),
            );
            assert_same(
                &format!("c2CCW90 case {case}"),
                &(c.c2CCW90)(a),
                &(rust.c2CCW90)(a),
            );
            assert_same(
                &format!("c2Div case {case}"),
                &(c.c2Div)(a, divisor),
                &(rust.c2Div)(a, divisor),
            );
            if a.x != 0.0 || a.y != 0.0 {
                assert_same(
                    &format!("c2Norm case {case}"),
                    &(c.c2Norm)(a),
                    &(rust.c2Norm)(a),
                );
            }
            assert_same(
                &format!("c2MulrvT case {case}"),
                &(c.c2MulrvT)(rotation, a),
                &(rust.c2MulrvT)(rotation, a),
            );
        }

        let zero = V { x: 0.0, y: -0.0 };
        assert_f32("c2Len signed zero", (c.c2Len)(zero), (rust.c2Len)(zero));
    }
}

#[test]
fn ieee_special_float_values_match_bit_for_bit() {
    let (c, rust) = apis();
    let values = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];

    unsafe {
        for (i, &x) in values.iter().enumerate() {
            for (j, &y) in values.iter().enumerate() {
                let a = V { x, y };
                let b = V { x: y, y: x };
                let rotation = R { c: x, s: y };
                let transform = X { p: b, r: rotation };
                let label = format!("IEEE case {i}-{j}");

                assert_same(&label, &(c.c2V)(x, y), &(rust.c2V)(x, y));
                assert_same(
                    &format!("{label} c2Mulvs"),
                    &(c.c2Mulvs)(a, y),
                    &(rust.c2Mulvs)(a, y),
                );
                assert_same(
                    &format!("{label} c2Maxv"),
                    &(c.c2Maxv)(a, b),
                    &(rust.c2Maxv)(a, b),
                );
                assert_same(
                    &format!("{label} c2Minv"),
                    &(c.c2Minv)(a, b),
                    &(rust.c2Minv)(a, b),
                );
                assert_same(
                    &format!("{label} c2Clampv"),
                    &(c.c2Clampv)(a, b, a),
                    &(rust.c2Clampv)(a, b, a),
                );
                assert_same(
                    &format!("{label} c2Sub"),
                    &(c.c2Sub)(a, b),
                    &(rust.c2Sub)(a, b),
                );
                assert_f32(
                    &format!("{label} c2Dot"),
                    (c.c2Dot)(a, b),
                    (rust.c2Dot)(a, b),
                );
                assert_f32(&format!("{label} c2Len"), (c.c2Len)(a), (rust.c2Len)(a));
                assert_f32(
                    &format!("{label} c2Det2"),
                    (c.c2Det2)(a, b),
                    (rust.c2Det2)(a, b),
                );
                assert_same(
                    &format!("{label} c2Mulrv"),
                    &(c.c2Mulrv)(rotation, a),
                    &(rust.c2Mulrv)(rotation, a),
                );
                assert_same(
                    &format!("{label} c2Add"),
                    &(c.c2Add)(a, b),
                    &(rust.c2Add)(a, b),
                );
                assert_same(
                    &format!("{label} c2Mulxv"),
                    &(c.c2Mulxv)(transform, a),
                    &(rust.c2Mulxv)(transform, a),
                );
                assert_same(&format!("{label} c2Neg"), &(c.c2Neg)(a), &(rust.c2Neg)(a));
                assert_same(
                    &format!("{label} c2Skew"),
                    &(c.c2Skew)(a),
                    &(rust.c2Skew)(a),
                );
                assert_same(
                    &format!("{label} c2CCW90"),
                    &(c.c2CCW90)(a),
                    &(rust.c2CCW90)(a),
                );
                assert_same(
                    &format!("{label} c2Div"),
                    &(c.c2Div)(a, y),
                    &(rust.c2Div)(a, y),
                );
                assert_same(
                    &format!("{label} c2Norm"),
                    &(c.c2Norm)(a),
                    &(rust.c2Norm)(a),
                );
                assert_same(
                    &format!("{label} c2MulrvT"),
                    &(c.c2MulrvT)(rotation, a),
                    &(rust.c2MulrvT)(rotation, a),
                );
            }
        }
    }
}

#[test]
fn bounding_box_vertices_and_proxies_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xa4d1_529c_7e68_b309);

    unsafe {
        for case in 0..256 {
            let mut c_bb = Bb {
                min: rng.v(),
                max: rng.v(),
            };
            if case % 3 == 0 {
                c_bb.max = c_bb.min;
            }
            let mut rust_bb = c_bb;
            let sentinel = V {
                x: f32::from_bits(0x4f00_0001),
                y: f32::from_bits(0xcf00_0001),
            };
            let mut c_out = [sentinel; 4];
            let mut rust_out = [sentinel; 4];
            (c.c2BBVerts)(c_out.as_mut_ptr(), &mut c_bb);
            (rust.c2BBVerts)(rust_out.as_mut_ptr(), &mut rust_bb);
            assert_same(&format!("c2BBVerts case {case}"), &c_out, &rust_out);

            let circle = Circle {
                p: rng.v(),
                r: rng.f32(),
            };
            let capsule = Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rng.f32(),
            };
            for (kind, shape) in [
                (CIRCLE, (&circle as *const Circle).cast()),
                (AABB, (&c_bb as *const Bb).cast()),
                (CAPSULE, (&capsule as *const Capsule).cast()),
            ] {
                let initial = Proxy {
                    radius: f32::from_bits(0x7fc0_1234),
                    count: -99,
                    verts: [sentinel; 8],
                };
                let mut c_proxy = initial;
                let mut rust_proxy = initial;
                (c.c2MakeProxy)(shape, kind, &mut c_proxy);
                (rust.c2MakeProxy)(shape, kind, &mut rust_proxy);
                assert_same(
                    &format!("c2MakeProxy case {case} kind {kind}"),
                    &c_proxy,
                    &rust_proxy,
                );
            }
        }
    }
}

#[test]
fn simplex_metric_and_line_regions_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xf344_9725_4268_b18c);

    unsafe {
        for case in 0..256 {
            let points = [rng.v(), rng.v(), rng.v()];
            for count in 1..=3 {
                let mut c_simplex = simplex_with_points(points[0], points[1], points[2], count);
                let mut rust_simplex = c_simplex;
                assert_f32(
                    &format!("c2GJKSimplexMetric case {case} count {count}"),
                    (c.c2GJKSimplexMetric)(&mut c_simplex),
                    (rust.c2GJKSimplexMetric)(&mut rust_simplex),
                );
            }
        }

        let patterns = [
            (V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }, 1),
            (V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }, 1),
            (V { x: -1.0, y: 0.0 }, V { x: 1.0, y: 0.0 }, 2),
        ];
        for case in 0..256 {
            let (base_a, base_b, expected_count) = patterns[case % patterns.len()];
            let scale = rng.positive();
            let turns = rng.u32();
            let a = transformed(base_a, scale, turns);
            let b = transformed(base_b, scale, turns);
            let mut c_simplex = simplex_with_points(a, b, V::default(), 2);
            let mut rust_simplex = c_simplex;
            (c.c22)(&mut c_simplex);
            (rust.c22)(&mut rust_simplex);
            assert_eq!(c_simplex.count, expected_count, "c22 branch case {case}");
            assert_same(&format!("c22 case {case}"), &c_simplex, &rust_simplex);
        }
    }
}

#[test]
fn triangle_simplex_regions_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x12c8_6cb9_b4f2_a13d);
    let patterns = [
        (
            V { x: 1.0, y: 0.0 },
            V { x: 2.0, y: 1.0 },
            V { x: 2.0, y: -1.0 },
            1,
        ),
        (
            V { x: 2.0, y: -1.0 },
            V { x: 1.0, y: 0.0 },
            V { x: 2.0, y: 1.0 },
            1,
        ),
        (
            V { x: 2.0, y: 1.0 },
            V { x: 2.0, y: -1.0 },
            V { x: 1.0, y: 0.0 },
            1,
        ),
        (
            V { x: -1.0, y: 1.0 },
            V { x: 1.0, y: 1.0 },
            V { x: 0.0, y: 2.0 },
            2,
        ),
        (
            V { x: 0.0, y: 2.0 },
            V { x: -1.0, y: 1.0 },
            V { x: 1.0, y: 1.0 },
            2,
        ),
        (
            V { x: 1.0, y: 1.0 },
            V { x: 0.0, y: 2.0 },
            V { x: -1.0, y: 1.0 },
            2,
        ),
        (
            V { x: -1.0, y: -1.0 },
            V { x: 1.0, y: -1.0 },
            V { x: 0.0, y: 1.0 },
            3,
        ),
    ];

    unsafe {
        for case in 0..512 {
            let (base_a, base_b, base_c, expected_count) = patterns[case % patterns.len()];
            let scale = rng.positive();
            let turns = rng.u32();
            let mut c_simplex = simplex_with_points(
                transformed(base_a, scale, turns),
                transformed(base_b, scale, turns),
                transformed(base_c, scale, turns),
                3,
            );
            let mut rust_simplex = c_simplex;
            (c.c23)(&mut c_simplex);
            (rust.c23)(&mut rust_simplex);
            assert_eq!(c_simplex.count, expected_count, "c23 branch case {case}");
            assert_same(&format!("c23 case {case}"), &c_simplex, &rust_simplex);
        }
    }
}

#[test]
fn direction_support_witness_and_interpolation_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x8945_d2e1_31ab_3c78);

    unsafe {
        for case in 0..384 {
            let mut simplex = simplex_with_points(rng.v(), rng.v(), rng.v(), 1);
            let mut rust_simplex = simplex;
            assert_same(
                &format!("c2D count 1 case {case}"),
                &(c.c2D)(&mut simplex),
                &(rust.c2D)(&mut rust_simplex),
            );

            let a = V {
                x: -rng.positive(),
                y: if case % 2 == 0 { 1.0 } else { -1.0 },
            };
            let b = V {
                x: rng.positive(),
                y: if case % 2 == 0 { 1.0 } else { -1.0 },
            };
            simplex = simplex_with_points(a, b, V::default(), 2);
            rust_simplex = simplex;
            assert_same(
                &format!("c2D count 2 case {case}"),
                &(c.c2D)(&mut simplex),
                &(rust.c2D)(&mut rust_simplex),
            );

            simplex.count = 3;
            rust_simplex = simplex;
            assert_same(
                &format!("c2D count 3 case {case}"),
                &(c.c2D)(&mut simplex),
                &(rust.c2D)(&mut rust_simplex),
            );

            let mut verts = [V::default(); 64];
            for value in &mut verts[..8] {
                *value = rng.v();
            }
            let direction = rng.v();
            for count in 1..=8 {
                assert_eq!(
                    (c.c2Support)(verts.as_ptr(), count, direction),
                    (rust.c2Support)(verts.as_ptr(), count, direction),
                    "c2Support count {count} case {case}"
                );
            }

            let tied = [rng.v(); 8];
            assert_eq!(
                (c.c2Support)(tied.as_ptr(), 8, direction),
                (rust.c2Support)(tied.as_ptr(), 8, direction),
                "c2Support tie case {case}"
            );

            // The C function still reads verts[0] for count zero, then returns zero.
            assert_eq!(
                (c.c2Support)(verts.as_ptr(), 0, direction),
                (rust.c2Support)(verts.as_ptr(), 0, direction),
                "c2Support zero count case {case}"
            );
            for value in &mut verts {
                *value = rng.v();
            }
            assert_eq!(
                (c.c2Support)(verts.as_ptr(), 64, direction),
                (rust.c2Support)(verts.as_ptr(), 64, direction),
                "c2Support oversized count case {case}"
            );
        }

        for count in 1..=3 {
            for case in 0..256 {
                let mut c_simplex = simplex_with_points(rng.v(), rng.v(), rng.v(), count);
                c_simplex.a.u = rng.positive();
                c_simplex.b.u = rng.positive();
                c_simplex.c.u = rng.positive();
                c_simplex.div = c_simplex.a.u + c_simplex.b.u + c_simplex.c.u;
                let mut rust_simplex = c_simplex;
                let sentinel = V {
                    x: f32::from_bits(0x7fc0_1234),
                    y: f32::from_bits(0xffc0_5678),
                };
                let mut c_a = sentinel;
                let mut c_b = sentinel;
                let mut rust_a = sentinel;
                let mut rust_b = sentinel;
                (c.c2Witness)(&mut c_simplex, &mut c_a, &mut c_b);
                (rust.c2Witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
                assert_same(
                    &format!("c2Witness A count {count} case {case}"),
                    &c_a,
                    &rust_a,
                );
                assert_same(
                    &format!("c2Witness B count {count} case {case}"),
                    &c_b,
                    &rust_b,
                );
                assert_same(
                    &format!("c2L count {count} case {case}"),
                    &(c.c2L)(&mut c_simplex),
                    &(rust.c2L)(&mut rust_simplex),
                );
            }
        }
    }
}
