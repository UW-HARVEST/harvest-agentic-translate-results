mod common;

use common::*;
use std::ptr::{null, null_mut};

unsafe fn compare_gjk(
    label: &str,
    c: &Api,
    rust: &Api,
    a: &Shape,
    b: &Shape,
    ax: Option<&X>,
    bx: Option<&X>,
    use_radius: i32,
    with_outputs: bool,
    c_cache: Option<&mut Cache>,
    rust_cache: Option<&mut Cache>,
) {
    let mut c_out_a = V {
        x: f32::from_bits(0x7fc0_1234),
        y: f32::from_bits(0xffc0_5678),
    };
    let mut c_out_b = c_out_a;
    let mut rust_out_a = c_out_a;
    let mut rust_out_b = c_out_a;
    let mut c_iterations = -777;
    let mut rust_iterations = -777;
    let ax_ptr = ax.map_or(null(), |value| value as *const X);
    let bx_ptr = bx.map_or(null(), |value| value as *const X);
    let c_cache_ptr = c_cache.map_or(null_mut(), |value| value as *mut Cache);
    let rust_cache_ptr = rust_cache.map_or(null_mut(), |value| value as *mut Cache);
    let c_out_a_ptr = if with_outputs {
        &mut c_out_a
    } else {
        null_mut()
    };
    let c_out_b_ptr = if with_outputs {
        &mut c_out_b
    } else {
        null_mut()
    };
    let c_iterations_ptr = if with_outputs {
        &mut c_iterations
    } else {
        null_mut()
    };
    let rust_out_a_ptr = if with_outputs {
        &mut rust_out_a
    } else {
        null_mut()
    };
    let rust_out_b_ptr = if with_outputs {
        &mut rust_out_b
    } else {
        null_mut()
    };
    let rust_iterations_ptr = if with_outputs {
        &mut rust_iterations
    } else {
        null_mut()
    };

    let c_distance = unsafe {
        (c.c2GJK)(
            a.ptr(),
            a.kind(),
            ax_ptr,
            b.ptr(),
            b.kind(),
            bx_ptr,
            c_out_a_ptr,
            c_out_b_ptr,
            use_radius,
            c_iterations_ptr,
            c_cache_ptr,
        )
    };
    let rust_distance = unsafe {
        (rust.c2GJK)(
            a.ptr(),
            a.kind(),
            ax_ptr,
            b.ptr(),
            b.kind(),
            bx_ptr,
            rust_out_a_ptr,
            rust_out_b_ptr,
            use_radius,
            rust_iterations_ptr,
            rust_cache_ptr,
        )
    };
    assert_f32(&format!("{label} distance"), c_distance, rust_distance);
    if with_outputs {
        assert_same(&format!("{label} outA"), &c_out_a, &rust_out_a);
        assert_same(&format!("{label} outB"), &c_out_b, &rust_out_b);
        assert_eq!(
            c_iterations, rust_iterations,
            "{label} iteration count differs"
        );
    }
}

fn random_transform(rng: &mut Rng, identity: bool) -> X {
    if identity {
        X {
            p: V::default(),
            r: R { c: 1.0, s: 0.0 },
        }
    } else {
        let rotation = match rng.u32() % 4 {
            0 => R { c: 1.0, s: 0.0 },
            1 => R { c: 0.0, s: 1.0 },
            2 => R { c: -1.0, s: 0.0 },
            _ => R { c: 0.0, s: -1.0 },
        };
        X {
            p: rng.v(),
            r: rotation,
        }
    }
}

fn centered_shape(kind: u32, center_x: f32) -> Shape {
    match kind {
        CIRCLE => Shape::Circle(Circle {
            p: V {
                x: center_x,
                y: 0.0,
            },
            r: 2.0,
        }),
        AABB => Shape::Bb(Bb {
            min: V {
                x: center_x - 1.0,
                y: -1.0,
            },
            max: V {
                x: center_x + 1.0,
                y: 1.0,
            },
        }),
        CAPSULE => Shape::Capsule(Capsule {
            a: V {
                x: center_x,
                y: -1.0,
            },
            b: V {
                x: center_x,
                y: 1.0,
            },
            r: 1.0,
        }),
        _ => unreachable!(),
    }
}

fn x_extent(kind: u32) -> f32 {
    match kind {
        CIRCLE => 2.0,
        AABB | CAPSULE => 1.0,
        _ => unreachable!(),
    }
}

#[test]
fn gjk_all_ordered_shape_pairs_without_radius_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xf31d_2587_8a44_60bc);

    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                for case in 0..128 {
                    let a = random_shape(&mut rng, type_a);
                    let b = random_shape(&mut rng, type_b);
                    compare_gjk(
                        &format!("pair {type_a}-{type_b} case {case}"),
                        &c,
                        &rust,
                        &a,
                        &b,
                        None,
                        None,
                        0,
                        true,
                        None,
                        None,
                    );
                }
            }
        }
    }
}

#[test]
fn gjk_transforms_radius_cold_and_warm_caches_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x43a8_99b2_124f_d576);

    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                for case in 0..128 {
                    let a = random_shape(&mut rng, type_a);
                    let b = random_shape(&mut rng, type_b);
                    let ax = random_transform(&mut rng, false);
                    let bx = random_transform(&mut rng, false);
                    let initial = Cache {
                        metric: f32::from_bits(0x7fc0_1234),
                        count: 0,
                        iA: [-91, -92, -93],
                        iB: [-81, -82, -83],
                        div: f32::from_bits(0xffc0_5678),
                    };
                    let mut c_cache = initial;
                    let mut rust_cache = initial;
                    compare_gjk(
                        &format!("cold pair {type_a}-{type_b} case {case}"),
                        &c,
                        &rust,
                        &a,
                        &b,
                        Some(&ax),
                        Some(&bx),
                        1,
                        true,
                        Some(&mut c_cache),
                        Some(&mut rust_cache),
                    );
                    assert_same(
                        &format!("cold cache pair {type_a}-{type_b} case {case}"),
                        &c_cache,
                        &rust_cache,
                    );

                    compare_gjk(
                        &format!("warm pair {type_a}-{type_b} case {case}"),
                        &c,
                        &rust,
                        &a,
                        &b,
                        Some(&ax),
                        Some(&bx),
                        1,
                        true,
                        Some(&mut c_cache),
                        Some(&mut rust_cache),
                    );
                    assert_same(
                        &format!("warm cache pair {type_a}-{type_b} case {case}"),
                        &c_cache,
                        &rust_cache,
                    );
                }
            }
        }
    }
}

#[test]
fn gjk_optional_outputs_and_radius_boundaries_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xcb76_0d31_8e59_2af4);

    unsafe {
        for case in 0..256 {
            let a = random_shape(&mut rng, [CIRCLE, AABB, CAPSULE][case % 3]);
            let b = random_shape(&mut rng, [CAPSULE, CIRCLE, AABB][case % 3]);
            compare_gjk(
                &format!("all optional pointers null case {case}"),
                &c,
                &rust,
                &a,
                &b,
                None,
                None,
                case as i32 & 1,
                false,
                None,
                None,
            );
        }

        for case in 0..256 {
            let radius_a = rng.positive();
            let radius_b = rng.positive();
            let gap = rng.positive();
            let separated_a = Shape::Circle(Circle {
                p: V { x: 0.0, y: 0.0 },
                r: radius_a,
            });
            let separated_b = Shape::Circle(Circle {
                p: V {
                    x: radius_a + radius_b + gap,
                    y: 0.0,
                },
                r: radius_b,
            });
            compare_gjk(
                &format!("radius separated case {case}"),
                &c,
                &rust,
                &separated_a,
                &separated_b,
                None,
                None,
                1,
                true,
                None,
                None,
            );

            let touching_b = Shape::Circle(Circle {
                p: V {
                    x: radius_a + radius_b,
                    y: 0.0,
                },
                r: radius_b,
            });
            compare_gjk(
                &format!("radius touching case {case}"),
                &c,
                &rust,
                &separated_a,
                &touching_b,
                None,
                None,
                1,
                true,
                None,
                None,
            );

            let overlap_b = Shape::Circle(Circle {
                p: V {
                    x: (radius_a + radius_b) * 0.5,
                    y: 0.0,
                },
                r: radius_b,
            });
            compare_gjk(
                &format!("radius overlap case {case}"),
                &c,
                &rust,
                &separated_a,
                &overlap_b,
                None,
                None,
                1,
                true,
                None,
                None,
            );
        }
    }
}

#[test]
fn direct_collision_entry_points_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x79c0_4ad8_e312_6bf5);

    unsafe {
        for case in 0..1024 {
            let center_a = rng.v();
            let half_a = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            let center_b = rng.v();
            let half_b = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            let bb_a = Bb {
                min: V {
                    x: center_a.x - half_a.x,
                    y: center_a.y - half_a.y,
                },
                max: V {
                    x: center_a.x + half_a.x,
                    y: center_a.y + half_a.y,
                },
            };
            let bb_b = Bb {
                min: V {
                    x: center_b.x - half_b.x,
                    y: center_b.y - half_b.y,
                },
                max: V {
                    x: center_b.x + half_b.x,
                    y: center_b.y + half_b.y,
                },
            };
            let circle_a = Circle {
                p: center_a,
                r: rng.positive(),
            };
            let circle_b = Circle {
                p: center_b,
                r: rng.positive(),
            };
            let capsule_a = Capsule {
                a: center_a,
                b: rng.v(),
                r: rng.positive(),
            };
            let capsule_b = Capsule {
                a: center_b,
                b: rng.v(),
                r: rng.positive(),
            };

            assert_eq!(
                (c.c2AABBtoAABB)(bb_a, bb_b),
                (rust.c2AABBtoAABB)(bb_a, bb_b),
                "c2AABBtoAABB case {case}"
            );
            assert_eq!(
                (c.c2AABBtoCapsule)(bb_a, capsule_b),
                (rust.c2AABBtoCapsule)(bb_a, capsule_b),
                "c2AABBtoCapsule case {case}"
            );
            assert_eq!(
                (c.c2CapsuletoCapsule)(capsule_a, capsule_b),
                (rust.c2CapsuletoCapsule)(capsule_a, capsule_b),
                "c2CapsuletoCapsule case {case}"
            );
            assert_eq!(
                (c.c2CircletoCircle)(circle_a, circle_b),
                (rust.c2CircletoCircle)(circle_a, circle_b),
                "c2CircletoCircle case {case}"
            );
            assert_eq!(
                (c.c2CircletoAABB)(circle_a, bb_b),
                (rust.c2CircletoAABB)(circle_a, bb_b),
                "c2CircletoAABB case {case}"
            );
            assert_eq!(
                (c.c2CircletoCapsule)(circle_a, capsule_b),
                (rust.c2CircletoCapsule)(circle_a, capsule_b),
                "c2CircletoCapsule case {case}"
            );
        }
    }
}

#[test]
fn collision_tangent_overlap_and_separation_boundaries_match() {
    let (c, rust) = apis();
    let circle = Circle {
        p: V { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    let circles = [
        Circle {
            p: V { x: 3.0, y: 0.0 },
            r: 2.0,
        },
        Circle {
            p: V { x: 4.0, y: 0.0 },
            r: 2.0,
        },
        Circle {
            p: V { x: 5.0, y: 0.0 },
            r: 2.0,
        },
    ];
    let boxes = [
        Bb {
            min: V { x: 1.0, y: -1.0 },
            max: V { x: 3.0, y: 1.0 },
        },
        Bb {
            min: V { x: 2.0, y: -1.0 },
            max: V { x: 4.0, y: 1.0 },
        },
        Bb {
            min: V { x: 3.0, y: -1.0 },
            max: V { x: 5.0, y: 1.0 },
        },
    ];
    let capsules = [
        Capsule {
            a: V { x: 1.0, y: -2.0 },
            b: V { x: 1.0, y: 2.0 },
            r: 0.5,
        },
        Capsule {
            a: V { x: 2.5, y: -2.0 },
            b: V { x: 2.5, y: 2.0 },
            r: 0.5,
        },
        Capsule {
            a: V { x: 4.0, y: -2.0 },
            b: V { x: 4.0, y: 2.0 },
            r: 0.5,
        },
    ];

    unsafe {
        for (case, other) in circles.into_iter().enumerate() {
            assert_eq!(
                (c.c2CircletoCircle)(circle, other),
                (rust.c2CircletoCircle)(circle, other),
                "circle relation {case}"
            );
        }
        for (case, bb) in boxes.into_iter().enumerate() {
            assert_eq!(
                (c.c2CircletoAABB)(circle, bb),
                (rust.c2CircletoAABB)(circle, bb),
                "circle-AABB relation {case}"
            );
            let shifted = Bb {
                min: V {
                    x: bb.min.x + 2.0,
                    y: bb.min.y,
                },
                max: V {
                    x: bb.max.x + 2.0,
                    y: bb.max.y,
                },
            };
            assert_eq!(
                (c.c2AABBtoAABB)(bb, shifted),
                (rust.c2AABBtoAABB)(bb, shifted),
                "AABB relation {case}"
            );
        }
        for (case, capsule) in capsules.into_iter().enumerate() {
            assert_eq!(
                (c.c2CircletoCapsule)(circle, capsule),
                (rust.c2CircletoCapsule)(circle, capsule),
                "circle-capsule relation {case}"
            );
            assert_eq!(
                (c.c2AABBtoCapsule)(boxes[0], capsule),
                (rust.c2AABBtoCapsule)(boxes[0], capsule),
                "AABB-capsule relation {case}"
            );
            assert_eq!(
                (c.c2CapsuletoCapsule)(capsules[0], capsule),
                (rust.c2CapsuletoCapsule)(capsules[0], capsule),
                "capsule relation {case}"
            );
        }

        let aabb_relations = [
            (
                Bb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 1.0, y: 1.0 },
                },
                Bb {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 2.0, y: 2.0 },
                },
            ),
            (
                Bb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 1.0, y: 1.0 },
                },
                Bb {
                    min: V { x: 1.0, y: -1.0 },
                    max: V { x: 3.0, y: 1.0 },
                },
            ),
            (
                Bb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 1.0, y: 1.0 },
                },
                Bb {
                    min: V { x: 1.0, y: 1.0 },
                    max: V { x: 3.0, y: 3.0 },
                },
            ),
            (
                Bb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 1.0, y: 1.0 },
                },
                Bb {
                    min: V { x: 2.0, y: -1.0 },
                    max: V { x: 4.0, y: 1.0 },
                },
            ),
            (
                Bb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 1.0, y: 1.0 },
                },
                Bb {
                    min: V { x: -1.0, y: 2.0 },
                    max: V { x: 1.0, y: 4.0 },
                },
            ),
        ];
        for (case, (a, b)) in aabb_relations.into_iter().enumerate() {
            assert_eq!(
                (c.c2AABBtoAABB)(a, b),
                (rust.c2AABBtoAABB)(a, b),
                "explicit AABB relation {case}"
            );
        }

        let corner_tangent_circle = Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 5.0,
        };
        let corner_tangent_box = Bb {
            min: V { x: 3.0, y: 4.0 },
            max: V { x: 6.0, y: 7.0 },
        };
        assert_eq!(
            (c.c2CircletoAABB)(corner_tangent_circle, corner_tangent_box),
            (rust.c2CircletoAABB)(corner_tangent_circle, corner_tangent_box),
            "circle-AABB corner tangent"
        );

        let endpoint_and_segment_capsules = [
            Capsule {
                a: V { x: 3.0, y: 0.0 },
                b: V { x: 5.0, y: 0.0 },
                r: 1.0,
            },
            Capsule {
                a: V { x: -2.0, y: 3.0 },
                b: V { x: 2.0, y: 3.0 },
                r: 1.0,
            },
            Capsule {
                a: V { x: -5.0, y: 0.0 },
                b: V { x: -3.0, y: 0.0 },
                r: 1.0,
            },
        ];
        for (region, capsule) in endpoint_and_segment_capsules.into_iter().enumerate() {
            for radius in [1.5, 2.0, 2.5] {
                let region_circle = Circle {
                    p: V::default(),
                    r: radius,
                };
                assert_eq!(
                    (c.c2CircletoCapsule)(region_circle, capsule),
                    (rust.c2CircletoCapsule)(region_circle, capsule),
                    "circle-capsule region {region} radius {radius}"
                );
            }
        }
    }
}

#[test]
fn collided_dispatch_all_ordered_pairs_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xe275_58c3_91ab_460d);

    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                let touching_distance = x_extent(type_a) + x_extent(type_b);
                for (relation, distance) in [
                    ("overlap", 0.0),
                    ("touch", touching_distance),
                    ("separate", touching_distance + 1.0),
                ] {
                    let a = centered_shape(type_a, 0.0);
                    let b = centered_shape(type_b, distance);
                    assert_eq!(
                        (c.c2Collided)(a.ptr(), type_a, b.ptr(), type_b),
                        (rust.c2Collided)(a.ptr(), type_a, b.ptr(), type_b),
                        "c2Collided explicit {relation} pair {type_a}-{type_b}"
                    );
                }
                for case in 0..512 {
                    let a = random_shape(&mut rng, type_a);
                    let b = random_shape(&mut rng, type_b);
                    assert_eq!(
                        (c.c2Collided)(a.ptr(), type_a, b.ptr(), type_b),
                        (rust.c2Collided)(a.ptr(), type_a, b.ptr(), type_b),
                        "c2Collided pair {type_a}-{type_b} case {case}"
                    );
                }
            }
        }
    }
}

#[test]
fn aabb_wrapper_normal_degenerate_and_inverted_inputs_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6c18_a4be_d239_70f5);

    unsafe {
        for case in 0..4096 {
            let first = rng.v();
            let second = rng.v();
            let (min_x, min_y, max_x, max_y) = match case % 3 {
                0 => (
                    first.x.min(second.x),
                    first.y.min(second.y),
                    first.x.max(second.x),
                    first.y.max(second.y),
                ),
                1 => (first.x, first.y, first.x, second.y),
                _ => (first.x, first.y, second.x, second.y),
            };
            assert_eq!(
                (c.aabb)(min_x, min_y, max_x, max_y),
                (rust.aabb)(min_x, min_y, max_x, max_y),
                "aabb case {case}: ({min_x}, {min_y})-({max_x}, {max_y})"
            );
        }
    }
}

#[test]
fn aabb_wrapper_ieee_special_values_match() {
    let (c, rust) = apis();
    let values = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::from_bits(1),
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];

    unsafe {
        for case in 0..2048 {
            let min_x = values[case % values.len()];
            let min_y = values[(case * 3 + 1) % values.len()];
            let max_x = values[(case * 5 + 2) % values.len()];
            let max_y = values[(case * 7 + 3) % values.len()];
            assert_eq!(
                (c.aabb)(min_x, min_y, max_x, max_y),
                (rust.aabb)(min_x, min_y, max_x, max_y),
                "aabb IEEE case {case}"
            );
        }
    }
}
