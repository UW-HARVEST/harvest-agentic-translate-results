mod common;

use common::*;
use std::ffi::{c_int, c_void};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Box(Box2),
    Capsule(Capsule),
}

impl Shape {
    fn tag(&self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Box(_) => AABB,
            Self::Capsule(_) => CAPSULE,
        }
    }

    fn raw(&self) -> *const c_void {
        match self {
            Self::Circle(value) => ptr(value),
            Self::Box(value) => ptr(value),
            Self::Capsule(value) => ptr(value),
        }
    }
}

#[derive(Clone, Copy)]
struct GjkOptions {
    ax: Option<X>,
    bx: Option<X>,
    use_radius: c_int,
    out_a: bool,
    out_b: bool,
    iterations: bool,
    cache: Option<Cache>,
}

#[derive(Clone, Copy, Debug)]
struct GjkResult {
    distance: f32,
    out_a: V,
    out_b: V,
    iterations: c_int,
    cache: Cache,
}

unsafe fn invoke_gjk(
    function: unsafe extern "C" fn(
        *const c_void,
        c_int,
        *const X,
        *const c_void,
        c_int,
        *const X,
        *mut V,
        *mut V,
        c_int,
        *mut c_int,
        *mut Cache,
    ) -> f32,
    a: &Shape,
    b: &Shape,
    options: GjkOptions,
) -> GjkResult {
    let mut out_a = V {
        x: 12345.0,
        y: -54321.0,
    };
    let mut out_b = V {
        x: -23456.0,
        y: 65432.0,
    };
    let mut iterations = -99;
    let mut cache = options.cache.unwrap_or(Cache {
        metric: 777.0,
        count: -7,
        iA: [31, 32, 33],
        iB: [41, 42, 43],
        div: 888.0,
    });
    let distance = unsafe {
        function(
            a.raw(),
            a.tag(),
            options
                .ax
                .as_ref()
                .map_or(std::ptr::null(), |value| value as *const X),
            b.raw(),
            b.tag(),
            options
                .bx
                .as_ref()
                .map_or(std::ptr::null(), |value| value as *const X),
            if options.out_a {
                &mut out_a
            } else {
                std::ptr::null_mut()
            },
            if options.out_b {
                &mut out_b
            } else {
                std::ptr::null_mut()
            },
            options.use_radius,
            if options.iterations {
                &mut iterations
            } else {
                std::ptr::null_mut()
            },
            if options.cache.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkResult {
        distance,
        out_a,
        out_b,
        iterations,
        cache,
    }
}

unsafe fn compare_gjk(
    api: &Pair,
    a: &Shape,
    b: &Shape,
    options: GjkOptions,
    context: &str,
) -> GjkResult {
    let c = unsafe { invoke_gjk(api.c.c2GJK, a, b, options) };
    let rust = unsafe { invoke_gjk(api.rust.c2GJK, a, b, options) };
    same(c.distance, rust.distance, &format!("{context} distance"));
    if options.out_a {
        same(c.out_a, rust.out_a, &format!("{context} outA"));
    }
    if options.out_b {
        same(c.out_b, rust.out_b, &format!("{context} outB"));
    }
    if options.iterations {
        same(
            c.iterations,
            rust.iterations,
            &format!("{context} iterations"),
        );
        assert!(
            (0..=20).contains(&c.iterations),
            "{context}: C iteration cap violated: {}",
            c.iterations
        );
    }
    if options.cache.is_some() {
        same(c.cache, rust.cache, &format!("{context} cache"));
    }
    c
}

fn random_shape(rng: &mut Rng, tag: c_int) -> Shape {
    match tag {
        CIRCLE => Shape::Circle(Circle {
            p: rng.small_v(),
            r: rng.positive(),
        }),
        AABB => {
            let center = rng.small_v();
            let extent = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Box(Box2 {
                min: V {
                    x: center.x - extent.x,
                    y: center.y - extent.y,
                },
                max: V {
                    x: center.x + extent.x,
                    y: center.y + extent.y,
                },
            })
        }
        CAPSULE => Shape::Capsule(Capsule {
            a: rng.small_v(),
            b: rng.small_v(),
            r: rng.positive(),
        }),
        _ => unreachable!(),
    }
}

fn random_transform(rng: &mut Rng) -> X {
    X {
        p: rng.small_v(),
        r: R {
            c: rng.small(),
            s: rng.small(),
        },
    }
}

#[test]
fn gjk_all_shape_pairs_options_and_cache_states_match() {
    unsafe {
        let api = Pair::load();
        let mut rng = Rng(0xa4f3_93c7_52e1_04bd);
        let tags = [CIRCLE, AABB, CAPSULE];

        for &tag_a in &tags {
            for &tag_b in &tags {
                for iteration in 0..384 {
                    let a = random_shape(&mut rng, tag_a);
                    let b = random_shape(&mut rng, tag_b);
                    let transform_mode = iteration % 4;
                    let output_mode = iteration % 5;
                    let options = GjkOptions {
                        ax: matches!(transform_mode, 1 | 3)
                            .then(|| random_transform(&mut rng)),
                        bx: matches!(transform_mode, 2 | 3)
                            .then(|| random_transform(&mut rng)),
                        use_radius: (iteration % 2) as c_int,
                        out_a: !matches!(output_mode, 1 | 4),
                        out_b: !matches!(output_mode, 2 | 4),
                        iterations: !matches!(output_mode, 3 | 4),
                        cache: if iteration % 3 == 0 {
                            Some(Cache::default())
                        } else {
                            None
                        },
                    };
                    let first = compare_gjk(
                        &api,
                        &a,
                        &b,
                        options,
                        &format!("pair {tag_a}/{tag_b} random {iteration}"),
                    );

                    if options.cache.is_some() && first.cache.count > 0 {
                        let warm_options = GjkOptions {
                            cache: Some(first.cache),
                            ..options
                        };
                        compare_gjk(
                            &api,
                            &a,
                            &b,
                            warm_options,
                            &format!("pair {tag_a}/{tag_b} warm {iteration}"),
                        );
                    }
                }
            }
        }

        let point = Shape::Circle(Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 0.0,
        });
        let same_point = Shape::Circle(Circle {
            p: V { x: 0.0, y: 0.0 },
            r: 0.0,
        });
        compare_gjk(
            &api,
            &point,
            &same_point,
            GjkOptions {
                ax: None,
                bx: None,
                use_radius: 1,
                out_a: true,
                out_b: true,
                iterations: true,
                cache: None,
            },
            "zero direction and midpoint collapse",
        );

        let far_a = Shape::Circle(Circle {
            p: V { x: -100.0, y: 0.0 },
            r: 3.0,
        });
        let far_b = Shape::Circle(Circle {
            p: V { x: 100.0, y: 0.0 },
            r: 5.0,
        });
        for use_radius in [0, 1] {
            compare_gjk(
                &api,
                &far_a,
                &far_b,
                GjkOptions {
                    ax: None,
                    bx: None,
                    use_radius,
                    out_a: true,
                    out_b: true,
                    iterations: true,
                    cache: None,
                },
                &format!("separated circles radius={use_radius}"),
            );
        }
        let rounded_a = Shape::Circle(Circle {
            p: V {
                x: 100_000_000.0,
                y: 0.0,
            },
            r: 3.0,
        });
        let rounded_b = Shape::Circle(Circle {
            p: V {
                x: 100_000_008.0,
                y: 0.0,
            },
            r: 4.0,
        });
        let rounded = compare_gjk(
            &api,
            &rounded_a,
            &rounded_b,
            GjkOptions {
                ax: None,
                bx: None,
                use_radius: 1,
                out_a: true,
                out_b: true,
                iterations: true,
                cache: None,
            },
            "radius adjustment rounds witnesses equal",
        );
        assert_eq!(rounded.distance.to_bits(), 0.0f32.to_bits());
        same(rounded.out_a, rounded.out_b, "rounded witnesses equal");

        let aabb_a = Shape::Box(Box2 {
            min: V { x: -2.0, y: -2.0 },
            max: V { x: 2.0, y: 2.0 },
        });
        let aabb_b = Shape::Box(Box2 {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 3.0, y: 3.0 },
        });
        let cache_templates = [
            Cache {
                metric: 1.0,
                count: 1,
                iA: [0, 0, 0],
                iB: [0, 0, 0],
                div: 1.0,
            },
            Cache {
                metric: 2.0,
                count: 2,
                iA: [0, 2, 0],
                iB: [2, 0, 0],
                div: 2.0,
            },
            Cache {
                metric: 4.0,
                count: 3,
                iA: [0, 1, 2],
                iB: [2, 3, 0],
                div: 4.0,
            },
        ];
        for cache in cache_templates {
            compare_gjk(
                &api,
                &aabb_a,
                &aabb_b,
                GjkOptions {
                    ax: None,
                    bx: None,
                    use_radius: 1,
                    out_a: true,
                    out_b: true,
                    iterations: true,
                    cache: Some(cache),
                },
                &format!("manual warm cache count {}", cache.count),
            );
        }

        let degenerate_a = Shape::Box(Box2 {
            min: V { x: 0.0, y: 0.0 },
            max: V { x: 0.0, y: 0.0 },
        });
        let huge_b = Shape::Box(Box2 {
            min: V {
                x: -20_000.0,
                y: -20_000.0,
            },
            max: V {
                x: 20_000.0,
                y: 20_000.0,
            },
        });
        let rejected_cache = Cache {
            metric: 0.0,
            count: 3,
            iA: [0, 0, 0],
            iB: [0, 2, 1],
            div: 1.0,
        };
        compare_gjk(
            &api,
            &degenerate_a,
            &huge_b,
            GjkOptions {
                ax: None,
                bx: None,
                use_radius: 0,
                out_a: true,
                out_b: true,
                iterations: true,
                cache: Some(rejected_cache),
            },
            "negative huge cache metric rejection",
        );
    }
}

#[test]
fn collision_wrappers_dispatch_and_aabb_entry_match() {
    unsafe {
        let api = Pair::load();
        let targeted_boxes = [
            (
                Box2 {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 4.0, y: 4.0 },
                },
                Box2 {
                    min: V { x: 2.0, y: 2.0 },
                    max: V { x: 6.0, y: 6.0 },
                },
            ),
            (
                Box2 {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 4.0, y: 4.0 },
                },
                Box2 {
                    min: V { x: 4.0, y: 1.0 },
                    max: V { x: 8.0, y: 3.0 },
                },
            ),
            (
                Box2 {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 4.0, y: 4.0 },
                },
                Box2 {
                    min: V { x: 5.0, y: 1.0 },
                    max: V { x: 8.0, y: 3.0 },
                },
            ),
            (
                Box2 {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 4.0, y: 4.0 },
                },
                Box2 {
                    min: V { x: 1.0, y: 5.0 },
                    max: V { x: 3.0, y: 8.0 },
                },
            ),
        ];
        for (index, (a, b)) in targeted_boxes.into_iter().enumerate() {
            same(
                (api.c.c2AABBtoAABB)(a, b),
                (api.rust.c2AABBtoAABB)(a, b),
                &format!("targeted AABB/AABB {index}"),
            );
        }

        let circle_cases = [
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                Circle {
                    p: V { x: 8.0, y: 0.0 },
                    r: 2.0,
                },
            ),
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                Circle {
                    p: V { x: 4.0, y: 0.0 },
                    r: 2.0,
                },
            ),
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 3.0,
                },
                Circle {
                    p: V { x: 4.0, y: 0.0 },
                    r: 2.0,
                },
            ),
        ];
        for (index, (a, b)) in circle_cases.into_iter().enumerate() {
            same(
                (api.c.c2CircletoCircle)(a, b),
                (api.rust.c2CircletoCircle)(a, b),
                &format!("targeted circle/circle {index}"),
            );
        }

        let box0 = Box2 {
            min: V { x: -2.0, y: -2.0 },
            max: V { x: 2.0, y: 2.0 },
        };
        let circle_box_cases = [
            Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 3.0, y: 0.0 },
                r: 2.0,
            },
            Circle {
                p: V { x: 3.0, y: 3.0 },
                r: 2.0,
            },
            Circle {
                p: V { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 8.0, y: 8.0 },
                r: 1.0,
            },
        ];
        for (index, circle) in circle_box_cases.into_iter().enumerate() {
            same(
                (api.c.c2CircletoAABB)(circle, box0),
                (api.rust.c2CircletoAABB)(circle, box0),
                &format!("targeted circle/AABB {index}"),
            );
        }

        let segment = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        let circle_capsule_cases = [
            Circle {
                p: V { x: -5.0, y: 4.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: -1.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 5.0, y: 5.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 5.0, y: 1.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 15.0, y: 4.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 11.0, y: 0.0 },
                r: 1.0,
            },
        ];
        for (index, circle) in circle_capsule_cases.into_iter().enumerate() {
            same(
                (api.c.c2CircletoCapsule)(circle, segment),
                (api.rust.c2CircletoCapsule)(circle, segment),
                &format!("targeted circle/capsule {index}"),
            );
        }

        let far_capsule = Capsule {
            a: V { x: 20.0, y: 20.0 },
            b: V { x: 30.0, y: 20.0 },
            r: 1.0,
        };
        let crossing_capsule = Capsule {
            a: V { x: -5.0, y: 0.0 },
            b: V { x: 5.0, y: 0.0 },
            r: 1.0,
        };
        for (index, capsule) in [far_capsule, crossing_capsule].into_iter().enumerate() {
            same(
                (api.c.c2AABBtoCapsule)(box0, capsule),
                (api.rust.c2AABBtoCapsule)(box0, capsule),
                &format!("targeted AABB/capsule {index}"),
            );
        }
        for (index, capsule) in [far_capsule, crossing_capsule].into_iter().enumerate() {
            same(
                (api.c.c2CapsuletoCapsule)(segment, capsule),
                (api.rust.c2CapsuletoCapsule)(segment, capsule),
                &format!("targeted capsule/capsule {index}"),
            );
        }

        let aabb_bit_cases = [
            (-75.0, -5.0, -65.0, 5.0, 1),
            (-35.0, -35.0, -20.0, -20.0, 2),
            (-35.0, 50.0, -25.0, 60.0, 4),
            (200.0, 200.0, 210.0, 210.0, 0),
        ];
        for (min_x, min_y, max_x, max_y, expected) in aabb_bit_cases {
            let c = (api.c.aabb)(min_x, min_y, max_x, max_y);
            let rust = (api.rust.aabb)(min_x, min_y, max_x, max_y);
            same(c, rust, "targeted aabb result bit");
            assert_eq!(c, expected);
        }

        let mut rng = Rng(0x19e2_786b_023f_7a51);
        for iteration in 0..4096 {
            let circle_a = match random_shape(&mut rng, CIRCLE) {
                Shape::Circle(value) => value,
                _ => unreachable!(),
            };
            let circle_b = match random_shape(&mut rng, CIRCLE) {
                Shape::Circle(value) => value,
                _ => unreachable!(),
            };
            let box_a = match random_shape(&mut rng, AABB) {
                Shape::Box(value) => value,
                _ => unreachable!(),
            };
            let box_b = match random_shape(&mut rng, AABB) {
                Shape::Box(value) => value,
                _ => unreachable!(),
            };
            let capsule_a = match random_shape(&mut rng, CAPSULE) {
                Shape::Capsule(value) => value,
                _ => unreachable!(),
            };
            let capsule_b = match random_shape(&mut rng, CAPSULE) {
                Shape::Capsule(value) => value,
                _ => unreachable!(),
            };

            same(
                (api.c.c2AABBtoAABB)(box_a, box_b),
                (api.rust.c2AABBtoAABB)(box_a, box_b),
                &format!("random AABB/AABB {iteration}"),
            );
            same(
                (api.c.c2AABBtoCapsule)(box_a, capsule_a),
                (api.rust.c2AABBtoCapsule)(box_a, capsule_a),
                &format!("random AABB/capsule {iteration}"),
            );
            same(
                (api.c.c2CapsuletoCapsule)(capsule_a, capsule_b),
                (api.rust.c2CapsuletoCapsule)(capsule_a, capsule_b),
                &format!("random capsule/capsule {iteration}"),
            );
            same(
                (api.c.c2CircletoCircle)(circle_a, circle_b),
                (api.rust.c2CircletoCircle)(circle_a, circle_b),
                &format!("random circle/circle {iteration}"),
            );
            same(
                (api.c.c2CircletoAABB)(circle_a, box_a),
                (api.rust.c2CircletoAABB)(circle_a, box_a),
                &format!("random circle/AABB {iteration}"),
            );
            same(
                (api.c.c2CircletoCapsule)(circle_a, capsule_a),
                (api.rust.c2CircletoCapsule)(circle_a, capsule_a),
                &format!("random circle/capsule {iteration}"),
            );

            let shapes = [
                Shape::Circle(circle_a),
                Shape::Box(box_a),
                Shape::Capsule(capsule_a),
            ];
            let other_shapes = [
                Shape::Circle(circle_b),
                Shape::Box(box_b),
                Shape::Capsule(capsule_b),
            ];
            for left in &shapes {
                for right in &other_shapes {
                    same(
                        (api.c.c2Collided)(left.raw(), left.tag(), right.raw(), right.tag()),
                        (api.rust.c2Collided)(
                            left.raw(),
                            left.tag(),
                            right.raw(),
                            right.tag(),
                        ),
                        &format!(
                            "random c2Collided {}/{} iteration {iteration}",
                            left.tag(),
                            right.tag()
                        ),
                    );
                }
            }

            let x0 = rng.finite();
            let y0 = rng.finite();
            let x1 = rng.finite();
            let y1 = rng.finite();
            same(
                (api.c.aabb)(x0, y0, x1, y1),
                (api.rust.aabb)(x0, y0, x1, y1),
                &format!("aabb random {iteration}"),
            );
        }
    }
}
