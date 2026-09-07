mod common;

use common::*;
use std::ffi::c_void;

const CASES: usize = 512;

fn random_circle(rng: &mut Rng) -> C2Circle {
    C2Circle {
        p: v(rng.range(-500.0, 500.0), rng.range(-500.0, 500.0)),
        r: rng.range(-50.0, 100.0),
    }
}

fn random_aabb(rng: &mut Rng) -> C2Aabb {
    let min = v(rng.range(-500.0, 0.0), rng.range(-500.0, 0.0));
    C2Aabb {
        min,
        max: v(min.x + rng.range(0.0, 500.0), min.y + rng.range(0.0, 500.0)),
    }
}

fn random_capsule(rng: &mut Rng) -> C2Capsule {
    C2Capsule {
        a: v(rng.range(-500.0, 500.0), rng.range(-500.0, 500.0)),
        b: v(rng.range(-500.0, 500.0), rng.range(-500.0, 500.0)),
        r: rng.range(-50.0, 100.0),
    }
}

fn compare_circle_circle(pair: &Pair, row: &str, case: usize, a: C2Circle, b: C2Circle) {
    unsafe {
        assert_i32(
            row,
            case,
            (pair.c.circle_circle)(a, b),
            (pair.rust.circle_circle)(a, b),
        );
    }
}

fn compare_circle_aabb(pair: &Pair, row: &str, case: usize, a: C2Circle, b: C2Aabb) {
    unsafe {
        assert_i32(
            row,
            case,
            (pair.c.circle_aabb)(a, b),
            (pair.rust.circle_aabb)(a, b),
        );
    }
}

fn compare_circle_capsule(pair: &Pair, row: &str, case: usize, a: C2Circle, b: C2Capsule) {
    unsafe {
        assert_i32(
            row,
            case,
            (pair.c.circle_capsule)(a, b),
            (pair.rust.circle_capsule)(a, b),
        );
    }
}

fn run(row: u8) {
    let pair = Pair::load();
    let mut rng = Rng::new(0xd1ff_3a11_5eed_0000 ^ u64::from(row));

    match row {
        21 => {
            for case in 0..CASES {
                let a = random_circle(&mut rng);
                let ar = rng.range(0.1, 100.0);
                let br = rng.range(0.1, 100.0);
                let distance = (ar + br) * rng.range(0.0, 0.9);
                let b = C2Circle {
                    p: if rng.bool() {
                        v(a.p.x + distance, a.p.y)
                    } else {
                        v(a.p.x, a.p.y - distance)
                    },
                    r: br,
                };
                compare_circle_circle(&pair, "C21", case, C2Circle { r: ar, ..a }, b);
            }
        }
        22 => {
            for case in 0..CASES {
                let x = (case % 31) as f32 - 15.0;
                let y = (case % 17) as f32 - 8.0;
                let ar = (case % 8 + 1) as f32;
                let br = (case % 11 + 1) as f32;
                let a = C2Circle { p: v(x, y), r: ar };
                let b = C2Circle {
                    p: v(x + ar + br, y),
                    r: br,
                };
                compare_circle_circle(&pair, "C22", case, a, b);
            }
        }
        23 => {
            for case in 0..CASES {
                let x = rng.range(-100.0, 100.0);
                let y = rng.range(-100.0, 100.0);
                let ar = rng.range(0.0, 50.0);
                let br = rng.range(0.0, 50.0);
                let separation = ar + br + rng.range(0.1, 100.0);
                let a = C2Circle { p: v(x, y), r: ar };
                let b = C2Circle {
                    p: v(x + separation, y),
                    r: br,
                };
                compare_circle_circle(&pair, "C23", case, a, b);
            }
        }
        24 => {
            for case in 0..CASES {
                let s0 = SPECIAL[case % SPECIAL.len()];
                let s1 = SPECIAL[(case * 3 + 1) % SPECIAL.len()];
                let s2 = SPECIAL[(case * 5 + 2) % SPECIAL.len()];
                let (ar, br) = match case % 4 {
                    0 => (-rng.range(0.0, 100.0), -rng.range(0.0, 100.0)),
                    1 => (0.0, -0.0),
                    2 => (s0, s1),
                    _ => (s1, s2),
                };
                compare_circle_circle(
                    &pair,
                    "C24",
                    case,
                    C2Circle {
                        p: v(s0, s1),
                        r: ar,
                    },
                    C2Circle {
                        p: v(s2, s0),
                        r: br,
                    },
                );
            }
        }
        25 => {
            for case in 0..CASES {
                let b = random_aabb(&mut rng);
                let a = C2Circle {
                    p: v(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y)),
                    r: rng.range(0.0, 100.0),
                };
                compare_circle_aabb(&pair, "C25", case, a, b);
            }
        }
        26 => {
            for case in 0..CASES {
                let b = random_aabb(&mut rng);
                let outside = rng.range(0.01, 200.0);
                let p = match case % 4 {
                    0 => v(b.min.x - outside, rng.range(b.min.y, b.max.y)),
                    1 => v(b.max.x + outside, rng.range(b.min.y, b.max.y)),
                    2 => v(rng.range(b.min.x, b.max.x), b.min.y - outside),
                    _ => v(rng.range(b.min.x, b.max.x), b.max.y + outside),
                };
                compare_circle_aabb(
                    &pair,
                    "C26",
                    case,
                    C2Circle {
                        p,
                        r: rng.range(0.0, 250.0),
                    },
                    b,
                );
            }
        }
        27 => {
            for case in 0..CASES {
                let b = random_aabb(&mut rng);
                let dx = rng.range(0.01, 200.0);
                let dy = rng.range(0.01, 200.0);
                let p = match case % 4 {
                    0 => v(b.min.x - dx, b.min.y - dy),
                    1 => v(b.min.x - dx, b.max.y + dy),
                    2 => v(b.max.x + dx, b.min.y - dy),
                    _ => v(b.max.x + dx, b.max.y + dy),
                };
                compare_circle_aabb(
                    &pair,
                    "C27",
                    case,
                    C2Circle {
                        p,
                        r: rng.range(0.0, 300.0),
                    },
                    b,
                );
            }
        }
        28 => {
            for case in 0..CASES {
                let (a, b) = match case % 4 {
                    0 => {
                        let r = (case % 23 + 1) as f32;
                        (
                            C2Circle {
                                p: v(10.0 + r, 5.0),
                                r,
                            },
                            C2Aabb {
                                min: v(0.0, 0.0),
                                max: v(10.0, 10.0),
                            },
                        )
                    }
                    1 => (
                        C2Circle {
                            p: v(rng.finite(), rng.finite()),
                            r: if rng.bool() {
                                0.0
                            } else {
                                -rng.range(0.0, 100.0)
                            },
                        },
                        random_aabb(&mut rng),
                    ),
                    2 => {
                        let point = v(rng.finite(), rng.finite());
                        (
                            random_circle(&mut rng),
                            C2Aabb {
                                min: point,
                                max: point,
                            },
                        )
                    }
                    _ => (
                        random_circle(&mut rng),
                        C2Aabb {
                            min: v(20.0, 30.0),
                            max: v(-20.0, -30.0),
                        },
                    ),
                };
                compare_circle_aabb(&pair, "C28", case, a, b);
            }
        }
        29 => {
            for case in 0..CASES {
                let s0 = SPECIAL[case % SPECIAL.len()];
                let s1 = SPECIAL[(case * 3 + 1) % SPECIAL.len()];
                let s2 = SPECIAL[(case * 5 + 2) % SPECIAL.len()];
                let s3 = SPECIAL[(case * 7 + 3) % SPECIAL.len()];
                compare_circle_aabb(
                    &pair,
                    "C29",
                    case,
                    C2Circle {
                        p: v(s0, s1),
                        r: s2,
                    },
                    C2Aabb {
                        min: v(s1, s2),
                        max: v(s3, s0),
                    },
                );
            }
        }
        30..=32 => {
            let label = format!("C{row:02}");
            for case in 0..CASES {
                let horizontal = rng.bool();
                let origin = v(rng.range(-100.0, 100.0), rng.range(-100.0, 100.0));
                let length = rng.range(1.0, 100.0);
                let (a, b, p) = if horizontal {
                    let b = v(origin.x + length, origin.y);
                    let x = match row {
                        30 => origin.x - rng.range(0.01, 100.0),
                        31 => origin.x + rng.range(0.01, length * 0.99),
                        _ => b.x + rng.range(0.0, 100.0),
                    };
                    (origin, b, v(x, origin.y + rng.range(-100.0, 100.0)))
                } else {
                    let b = v(origin.x, origin.y + length);
                    let y = match row {
                        30 => origin.y - rng.range(0.01, 100.0),
                        31 => origin.y + rng.range(0.01, length * 0.99),
                        _ => b.y + rng.range(0.0, 100.0),
                    };
                    (origin, b, v(origin.x + rng.range(-100.0, 100.0), y))
                };
                compare_circle_capsule(
                    &pair,
                    &label,
                    case,
                    C2Circle {
                        p,
                        r: rng.range(-20.0, 100.0),
                    },
                    C2Capsule {
                        a,
                        b,
                        r: rng.range(-20.0, 100.0),
                    },
                );
            }
        }
        33 => {
            for case in 0..CASES {
                let ar = (case % 13) as f32;
                let br = (case % 17) as f32;
                let capsule = C2Capsule {
                    a: v(-20.0, 5.0),
                    b: v(20.0, 5.0),
                    r: if case % 5 == 0 { -br } else { br },
                };
                let circle = C2Circle {
                    p: v(0.0, 5.0 + ar + capsule.r),
                    r: if case % 7 == 0 { -ar } else { ar },
                };
                compare_circle_capsule(&pair, "C33", case, circle, capsule);
            }
        }
        34 => {
            for case in 0..CASES {
                let point = v(rng.finite(), rng.finite());
                compare_circle_capsule(
                    &pair,
                    "C34",
                    case,
                    random_circle(&mut rng),
                    C2Capsule {
                        a: point,
                        b: point,
                        r: rng.range(-100.0, 100.0),
                    },
                );
            }
        }
        35 => {
            for case in 0..CASES {
                let s0 = SPECIAL[case % SPECIAL.len()];
                let s1 = SPECIAL[(case * 3 + 1) % SPECIAL.len()];
                let s2 = SPECIAL[(case * 5 + 2) % SPECIAL.len()];
                let s3 = SPECIAL[(case * 7 + 3) % SPECIAL.len()];
                compare_circle_capsule(
                    &pair,
                    "C35",
                    case,
                    C2Circle {
                        p: v(s0, s1),
                        r: s2,
                    },
                    C2Capsule {
                        a: v(s1, s2),
                        b: v(s3, s0),
                        r: SPECIAL[(case * 11 + 4) % SPECIAL.len()],
                    },
                );
            }
        }
        36..=38 => {
            let label = format!("C{row:02}");
            for case in 0..CASES {
                let circle = random_circle(&mut rng);
                unsafe {
                    let (c_result, rust_result) = match row {
                        36 => {
                            let other = random_circle(&mut rng);
                            (
                                (pair.c.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Circle).cast::<c_void>(),
                                    0,
                                ),
                                (pair.rust.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Circle).cast::<c_void>(),
                                    0,
                                ),
                            )
                        }
                        37 => {
                            let other = random_aabb(&mut rng);
                            (
                                (pair.c.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Aabb).cast::<c_void>(),
                                    1,
                                ),
                                (pair.rust.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Aabb).cast::<c_void>(),
                                    1,
                                ),
                            )
                        }
                        _ => {
                            let other = random_capsule(&mut rng);
                            (
                                (pair.c.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Capsule).cast::<c_void>(),
                                    2,
                                ),
                                (pair.rust.collided)(
                                    (&circle as *const C2Circle).cast::<c_void>(),
                                    (&other as *const C2Capsule).cast::<c_void>(),
                                    2,
                                ),
                            )
                        }
                    };
                    assert_i32(&label, case, c_result, rust_result);
                }
            }
        }
        39 => {
            for case in 0..(CASES * 4) {
                let x = rng.range(-250.0, 150.0);
                let y = rng.range(-150.0, 180.0);
                let r = rng.range(0.001, 150.0);
                unsafe {
                    assert_i32(
                        "C39",
                        case,
                        (pair.c.circle_collide)(x, y, r),
                        (pair.rust.circle_collide)(x, y, r),
                    );
                }
            }
        }
        40 => {
            for case in 0..CASES {
                let (x, y, r) = if case % 3 == 0 {
                    (rng.finite(), rng.finite(), -rng.range(0.0, 100.0))
                } else if case % 3 == 1 {
                    (
                        rng.finite(),
                        rng.finite(),
                        if rng.bool() { 0.0 } else { -0.0 },
                    )
                } else {
                    (
                        SPECIAL[case % SPECIAL.len()],
                        SPECIAL[(case * 3 + 1) % SPECIAL.len()],
                        SPECIAL[(case * 5 + 2) % SPECIAL.len()],
                    )
                };
                unsafe {
                    assert_i32(
                        "C40",
                        case,
                        (pair.c.circle_collide)(x, y, r),
                        (pair.rust.circle_collide)(x, y, r),
                    );
                }
            }
        }
        _ => unreachable!(),
    }
}

macro_rules! config_tests {
    ($($name:ident: $row:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                run($row);
            }
        )+
    };
}

config_tests!(
    config_c21: 21,
    config_c22: 22,
    config_c23: 23,
    config_c24: 24,
    config_c25: 25,
    config_c26: 26,
    config_c27: 27,
    config_c28: 28,
    config_c29: 29,
    config_c30: 30,
    config_c31: 31,
    config_c32: 32,
    config_c33: 33,
    config_c34: 34,
    config_c35: 35,
    config_c36: 36,
    config_c37: 37,
    config_c38: 38,
    config_c39: 39,
    config_c40: 40,
);
