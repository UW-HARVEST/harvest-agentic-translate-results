mod common;

use common::*;
use std::ffi::c_void;

fn sv(p: V) -> Sv {
    Sv {
        sA: V {
            x: p.x + 1.25,
            y: p.y - 2.5,
        },
        sB: V {
            x: p.x - 3.75,
            y: p.y + 4.5,
        },
        p,
        u: 0.0,
        iA: 1,
        iB: 2,
    }
}

fn simplex(points: &[V], count: i32) -> Simplex {
    let mut result = Simplex {
        div: 7.0,
        count,
        ..Simplex::default()
    };
    if let Some(&point) = points.first() {
        result.a = sv(point);
    }
    if let Some(&point) = points.get(1) {
        result.b = sv(point);
    }
    if let Some(&point) = points.get(2) {
        result.c = sv(point);
    }
    result.d = sv(V { x: 99.0, y: -77.0 });
    result
}

fn compare_simplex_mutation(
    c_fn: unsafe extern "C" fn(*mut Simplex),
    rust_fn: unsafe extern "C" fn(*mut Simplex),
    input: Simplex,
    context: &str,
) -> Simplex {
    let mut c_value = input;
    let mut rust_value = input;
    unsafe {
        c_fn(&mut c_value);
        rust_fn(&mut rust_value);
    }
    same(c_value, rust_value, context);
    c_value
}

#[test]
fn low_level_exports_match_byte_for_byte() {
    unsafe {
        let api = Pair::load();
        let mut rng = Rng(0x6d5a_56da_1b74_9f21);

        let special = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc1_2345),
        ];
        for &x in &special {
            for &y in &special {
                same((api.c.c2V)(x, y), (api.rust.c2V)(x, y), "c2V special");
            }
        }

        for iteration in 0..1024 {
            let a = rng.v();
            let b = rng.v();
            let scalar = rng.finite();
            same(
                (api.c.c2Mulvs)(a, scalar),
                (api.rust.c2Mulvs)(a, scalar),
                &format!("c2Mulvs random {iteration}"),
            );
            same(
                (api.c.c2Sub)(a, b),
                (api.rust.c2Sub)(a, b),
                &format!("c2Sub random {iteration}"),
            );
            same(
                (api.c.c2Add)(a, b),
                (api.rust.c2Add)(a, b),
                &format!("c2Add random {iteration}"),
            );
            same(
                (api.c.c2Dot)(a, b),
                (api.rust.c2Dot)(a, b),
                &format!("c2Dot random {iteration}"),
            );
            same(
                (api.c.c2Len)(a),
                (api.rust.c2Len)(a),
                &format!("c2Len random {iteration}"),
            );
            same(
                (api.c.c2Det2)(a, b),
                (api.rust.c2Det2)(a, b),
                &format!("c2Det2 random {iteration}"),
            );
            same(
                (api.c.c2Neg)(a),
                (api.rust.c2Neg)(a),
                &format!("c2Neg random {iteration}"),
            );
            same(
                (api.c.c2Skew)(a),
                (api.rust.c2Skew)(a),
                &format!("c2Skew random {iteration}"),
            );
            same(
                (api.c.c2CCW90)(a),
                (api.rust.c2CCW90)(a),
                &format!("c2CCW90 random {iteration}"),
            );
            if scalar != 0.0 {
                same(
                    (api.c.c2Div)(a, scalar),
                    (api.rust.c2Div)(a, scalar),
                    &format!("c2Div random {iteration}"),
                );
            }
            if a.x != 0.0 || a.y != 0.0 {
                same(
                    (api.c.c2Norm)(a),
                    (api.rust.c2Norm)(a),
                    &format!("c2Norm random {iteration}"),
                );
            }

            let r = R {
                c: rng.small(),
                s: rng.small(),
            };
            let x = X {
                p: rng.small_v(),
                r,
            };
            same(
                (api.c.c2Mulrv)(r, a),
                (api.rust.c2Mulrv)(r, a),
                &format!("c2Mulrv random {iteration}"),
            );
            same(
                (api.c.c2MulrvT)(r, a),
                (api.rust.c2MulrvT)(r, a),
                &format!("c2MulrvT random {iteration}"),
            );
            same(
                (api.c.c2Mulxv)(x, a),
                (api.rust.c2Mulxv)(x, a),
                &format!("c2Mulxv random {iteration}"),
            );
        }

        let selectors = [
            (V { x: 3.0, y: 4.0 }, V { x: 1.0, y: 2.0 }),
            (V { x: 3.0, y: 1.0 }, V { x: 1.0, y: 2.0 }),
            (V { x: 1.0, y: 4.0 }, V { x: 3.0, y: 2.0 }),
            (V { x: 1.0, y: 2.0 }, V { x: 3.0, y: 4.0 }),
            (
                V {
                    x: f32::NAN,
                    y: 2.0,
                },
                V { x: 7.0, y: 2.0 },
            ),
        ];
        for (index, &(a, b)) in selectors.iter().enumerate() {
            same(
                (api.c.c2Maxv)(a, b),
                (api.rust.c2Maxv)(a, b),
                &format!("c2Maxv selector {index}"),
            );
            same(
                (api.c.c2Minv)(a, b),
                (api.rust.c2Minv)(a, b),
                &format!("c2Minv selector {index}"),
            );
        }

        let lo = V { x: -2.0, y: -3.0 };
        let hi = V { x: 5.0, y: 7.0 };
        let x_classes = [-9.0, 1.0, 12.0];
        let y_classes = [-11.0, 2.0, 15.0];
        for (x_index, &x) in x_classes.iter().enumerate() {
            for (y_index, &y) in y_classes.iter().enumerate() {
                let value = V { x, y };
                same(
                    (api.c.c2Clampv)(value, lo, hi),
                    (api.rust.c2Clampv)(value, lo, hi),
                    &format!("c2Clampv shape {x_index}/{y_index}"),
                );
            }
        }

        same(
            (api.c.c2RotIdentity)(),
            (api.rust.c2RotIdentity)(),
            "c2RotIdentity",
        );
        same(
            (api.c.c2xIdentity)(),
            (api.rust.c2xIdentity)(),
            "c2xIdentity",
        );

        for iteration in 0..256 {
            let mut bb = Box2 {
                min: rng.v(),
                max: rng.v(),
            };
            let mut c_out = [V {
                x: -999.0,
                y: 888.0,
            }; 4];
            let mut rust_out = c_out;
            (api.c.c2BBVerts)(c_out.as_mut_ptr(), &mut bb);
            (api.rust.c2BBVerts)(rust_out.as_mut_ptr(), &mut bb);
            same_slice(&c_out, &rust_out, &format!("c2BBVerts {iteration}"));
        }

        for iteration in 0..256 {
            let circle = Circle {
                p: rng.v(),
                r: rng.finite(),
            };
            let bb = Box2 {
                min: rng.v(),
                max: rng.v(),
            };
            let capsule = Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rng.finite(),
            };
            for (tag, shape) in [
                (CIRCLE, ptr(&circle)),
                (AABB, ptr(&bb)),
                (CAPSULE, ptr(&capsule)),
            ] {
                let sentinel = Proxy {
                    radius: -123.0,
                    count: -17,
                    verts: [V { x: 44.0, y: -55.0 }; 8],
                };
                let mut c_proxy = sentinel;
                let mut rust_proxy = sentinel;
                (api.c.c2MakeProxy)(shape, tag, &mut c_proxy);
                (api.rust.c2MakeProxy)(shape, tag, &mut rust_proxy);
                same(
                    c_proxy,
                    rust_proxy,
                    &format!("c2MakeProxy tag {tag} iteration {iteration}"),
                );
            }
        }

        for count in [1, 2, 3] {
            for iteration in 0..256 {
                let mut c_simplex = simplex(&[rng.v(), rng.v(), rng.v()], count);
                let mut rust_simplex = c_simplex;
                same(
                    (api.c.c2GJKSimplexMetric)(&mut c_simplex),
                    (api.rust.c2GJKSimplexMetric)(&mut rust_simplex),
                    &format!("c2GJKSimplexMetric count {count} iteration {iteration}"),
                );
            }
        }

        let c22_cases = [
            simplex(&[V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }], 2),
            simplex(&[V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }], 2),
            simplex(&[V { x: -1.0, y: 1.0 }, V { x: 1.0, y: 1.0 }], 2),
        ];
        let expected_c22_counts = [1, 1, 2];
        for (index, input) in c22_cases.into_iter().enumerate() {
            let output =
                compare_simplex_mutation(api.c.c22, api.rust.c22, input, &format!("c22 {index}"));
            assert_eq!(output.count, expected_c22_counts[index]);
        }
        for iteration in 0..2048 {
            compare_simplex_mutation(
                api.c.c22,
                api.rust.c22,
                simplex(&[rng.small_v(), rng.small_v()], 2),
                &format!("c22 random {iteration}"),
            );
        }

        let c23_cases = [
            simplex(
                &[
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 1.0 },
                    V { x: 2.0, y: -1.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: 2.0, y: -1.0 },
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 1.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: 2.0, y: 1.0 },
                    V { x: 2.0, y: -1.0 },
                    V { x: 1.0, y: 0.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: -1.0, y: 1.0 },
                    V { x: 1.0, y: 1.0 },
                    V { x: 0.0, y: 3.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: 3.0, y: 0.0 },
                    V { x: 1.0, y: -1.0 },
                    V { x: 1.0, y: 1.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: -1.0, y: 1.0 },
                    V { x: -3.0, y: 0.0 },
                    V { x: -1.0, y: -1.0 },
                ],
                3,
            ),
            simplex(
                &[
                    V { x: -1.0, y: -1.0 },
                    V { x: 1.0, y: -1.0 },
                    V { x: 0.0, y: 1.0 },
                ],
                3,
            ),
        ];
        let expected_c23_counts = [1, 1, 1, 2, 2, 2, 3];
        for (index, input) in c23_cases.into_iter().enumerate() {
            let output =
                compare_simplex_mutation(api.c.c23, api.rust.c23, input, &format!("c23 {index}"));
            assert_eq!(
                output.count, expected_c23_counts[index],
                "c23 targeted case {index} did not hit intended branch"
            );
        }
        for iteration in 0..4096 {
            compare_simplex_mutation(
                api.c.c23,
                api.rust.c23,
                simplex(&[rng.small_v(), rng.small_v(), rng.small_v()], 3),
                &format!("c23 random {iteration}"),
            );
        }

        for count in [1, 2, 3] {
            for iteration in 0..512 {
                let input = simplex(&[rng.small_v(), rng.small_v(), rng.small_v()], count);
                let mut c_simplex = input;
                let mut rust_simplex = input;
                same(
                    (api.c.c2D)(&mut c_simplex),
                    (api.rust.c2D)(&mut rust_simplex),
                    &format!("c2D count {count} iteration {iteration}"),
                );
                same(
                    (api.c.c2L)(&mut c_simplex),
                    (api.rust.c2L)(&mut rust_simplex),
                    &format!("c2L count {count} iteration {iteration}"),
                );
            }
        }
        let d_positive = simplex(
            &[V { x: 1.0, y: 1.0 }, V { x: -1.0, y: 1.0 }],
            2,
        );
        let d_nonpositive = simplex(
            &[V { x: 1.0, y: -1.0 }, V { x: -1.0, y: -1.0 }],
            2,
        );
        for (index, input) in [d_positive, d_nonpositive].into_iter().enumerate() {
            let mut c_value = input;
            let mut rust_value = input;
            same(
                (api.c.c2D)(&mut c_value),
                (api.rust.c2D)(&mut rust_value),
                &format!("c2D orientation {index}"),
            );
        }

        let tied = [
            V { x: 3.0, y: 0.0 },
            V { x: 3.0, y: 7.0 },
            V { x: -4.0, y: 1.0 },
        ];
        same(
            (api.c.c2Support)(tied.as_ptr(), tied.len() as i32, V { x: 1.0, y: 0.0 }),
            (api.rust.c2Support)(
                tied.as_ptr(),
                tied.len() as i32,
                V { x: 1.0, y: 0.0 },
            ),
            "c2Support tied maximum",
        );
        for count in 1..=9 {
            for iteration in 0..256 {
                let mut verts = [V::default(); 9];
                for vertex in &mut verts {
                    *vertex = rng.small_v();
                }
                let direction = rng.small_v();
                same(
                    (api.c.c2Support)(verts.as_ptr(), count, direction),
                    (api.rust.c2Support)(verts.as_ptr(), count, direction),
                    &format!("c2Support count {count} iteration {iteration}"),
                );
            }
        }

        for count in [1, 2, 3] {
            for iteration in 0..512 {
                let mut input = simplex(&[rng.small_v(), rng.small_v(), rng.small_v()], count);
                input.a.u = rng.positive();
                input.b.u = rng.positive();
                input.c.u = rng.positive();
                input.div = input.a.u + input.b.u + input.c.u;
                let mut c_simplex = input;
                let mut rust_simplex = input;
                let mut c_a = V { x: 91.0, y: 92.0 };
                let mut c_b = V { x: 93.0, y: 94.0 };
                let mut rust_a = c_a;
                let mut rust_b = c_b;
                (api.c.c2Witness)(&mut c_simplex, &mut c_a, &mut c_b);
                (api.rust.c2Witness)(&mut rust_simplex, &mut rust_a, &mut rust_b);
                same(
                    c_a,
                    rust_a,
                    &format!("c2Witness A count {count} iteration {iteration}"),
                );
                same(
                    c_b,
                    rust_b,
                    &format!("c2Witness B count {count} iteration {iteration}"),
                );
            }
        }

        for divisor in [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_55aa),
        ] {
            let value = V { x: 3.0, y: -7.0 };
            same(
                (api.c.c2Div)(value, divisor),
                (api.rust.c2Div)(value, divisor),
                "c2Div special divisor",
            );
        }
        for value in [
            V { x: 0.0, y: 0.0 },
            V { x: -0.0, y: 0.0 },
            V {
                x: f32::INFINITY,
                y: 1.0,
            },
            V {
                x: f32::from_bits(0x7fc0_1234),
                y: -2.0,
            },
        ] {
            same(
                (api.c.c2Norm)(value),
                (api.rust.c2Norm)(value),
                "c2Norm special",
            );
        }

        let _type_check: *const c_void = std::ptr::null();
    }
}
