mod common;

use common::*;

const CASES: usize = 512;

fn run(row: u8) {
    let pair = Pair::load();
    let mut rng = Rng::new(0x5eed_cafe_9e37_79b9 ^ u64::from(row));

    unsafe {
        match row {
            1 => {
                for case in 0..CASES {
                    let (x, y) = (rng.finite(), rng.finite());
                    assert_v("C01", case, (pair.c.c2v)(x, y), (pair.rust.c2v)(x, y));
                }
            }
            2 => {
                for (case, (&x, &y)) in SPECIAL
                    .iter()
                    .zip(SPECIAL.iter().rev())
                    .cycle()
                    .take(CASES)
                    .enumerate()
                {
                    assert_v("C02", case, (pair.c.c2v)(x, y), (pair.rust.c2v)(x, y));
                }
            }
            3 => {
                for case in 0..CASES {
                    let a = v(rng.finite(), rng.finite());
                    let b = rng.range(0.001, 100.0);
                    assert_v("C03", case, (pair.c.mulvs)(a, b), (pair.rust.mulvs)(a, b));
                }
            }
            4 => {
                for case in 0..CASES {
                    let a = v(rng.finite(), rng.finite());
                    let b = match case % 3 {
                        0 => -rng.range(0.001, 100.0),
                        1 => 0.0,
                        _ => -0.0,
                    };
                    assert_v("C04", case, (pair.c.mulvs)(a, b), (pair.rust.mulvs)(a, b));
                }
            }
            5 => {
                for case in 0..CASES {
                    let a = v(
                        SPECIAL[case % SPECIAL.len()],
                        SPECIAL[(case * 5 + 3) % SPECIAL.len()],
                    );
                    let b = SPECIAL[(case * 7 + 1) % SPECIAL.len()];
                    assert_v("C05", case, (pair.c.mulvs)(a, b), (pair.rust.mulvs)(a, b));
                }
            }
            6 => {
                for case in 0..CASES {
                    let b = v(rng.range(-1000.0, 999.0), rng.range(-1000.0, 999.0));
                    let a = v(b.x + rng.range(0.01, 1000.0), b.y + rng.range(0.01, 1000.0));
                    assert_v("C06", case, (pair.c.maxv)(a, b), (pair.rust.maxv)(a, b));
                }
            }
            7 => {
                for case in 0..CASES {
                    let (a, b) = if case % 3 == 0 {
                        (v(0.0, -0.0), v(-0.0, 0.0))
                    } else if case % 3 == 1 {
                        let a = v(rng.finite(), rng.finite());
                        (a, a)
                    } else {
                        let b = v(rng.range(-999.0, 1000.0), rng.range(-999.0, 1000.0));
                        (
                            v(b.x - rng.range(0.01, 1000.0), b.y - rng.range(0.01, 1000.0)),
                            b,
                        )
                    };
                    assert_v("C07", case, (pair.c.maxv)(a, b), (pair.rust.maxv)(a, b));
                }
            }
            8 => {
                for case in 0..CASES {
                    let nan = f32::from_bits(0x7fc0_0000 | (rng.next_u32() & 0x003f_ffff));
                    let finite = rng.finite();
                    let (a, b) = match case % 4 {
                        0 => (v(nan, finite), v(finite, nan)),
                        1 => (v(finite, nan), v(nan, finite)),
                        2 => (v(nan, nan), v(finite, finite)),
                        _ => (v(finite, finite), v(nan, nan)),
                    };
                    assert_v("C08", case, (pair.c.maxv)(a, b), (pair.rust.maxv)(a, b));
                }
            }
            9 => {
                for case in 0..CASES {
                    let b = v(rng.range(-999.0, 1000.0), rng.range(-999.0, 1000.0));
                    let a = v(b.x - rng.range(0.01, 1000.0), b.y - rng.range(0.01, 1000.0));
                    assert_v("C09", case, (pair.c.minv)(a, b), (pair.rust.minv)(a, b));
                }
            }
            10 => {
                for case in 0..CASES {
                    let (a, b) = if case % 3 == 0 {
                        (v(0.0, -0.0), v(-0.0, 0.0))
                    } else if case % 3 == 1 {
                        let a = v(rng.finite(), rng.finite());
                        (a, a)
                    } else {
                        let b = v(rng.range(-1000.0, 999.0), rng.range(-1000.0, 999.0));
                        (
                            v(b.x + rng.range(0.01, 1000.0), b.y + rng.range(0.01, 1000.0)),
                            b,
                        )
                    };
                    assert_v("C10", case, (pair.c.minv)(a, b), (pair.rust.minv)(a, b));
                }
            }
            11 => {
                for case in 0..CASES {
                    let nan = f32::from_bits(0x7fc0_0000 | (rng.next_u32() & 0x003f_ffff));
                    let finite = rng.finite();
                    let (a, b) = match case % 4 {
                        0 => (v(nan, finite), v(finite, nan)),
                        1 => (v(finite, nan), v(nan, finite)),
                        2 => (v(nan, nan), v(finite, finite)),
                        _ => (v(finite, finite), v(nan, nan)),
                    };
                    assert_v("C11", case, (pair.c.minv)(a, b), (pair.rust.minv)(a, b));
                }
            }
            12..=15 => {
                let label = format!("C{row:02}");
                for case in 0..CASES {
                    let lo = v(rng.range(-1000.0, 0.0), rng.range(-1000.0, 0.0));
                    let hi = v(lo.x + rng.range(1.0, 1000.0), lo.y + rng.range(1.0, 1000.0));
                    let a = match row {
                        12 => v(
                            lo.x - rng.range(0.01, 1000.0),
                            lo.y - rng.range(0.01, 1000.0),
                        ),
                        13 => v(rng.range(lo.x, hi.x), rng.range(lo.y, hi.y)),
                        14 => v(
                            hi.x + rng.range(0.01, 1000.0),
                            hi.y + rng.range(0.01, 1000.0),
                        ),
                        _ => match case % 6 {
                            0 => v(lo.x - 1.0, rng.range(lo.y, hi.y)),
                            1 => v(lo.x - 1.0, hi.y + 1.0),
                            2 => v(rng.range(lo.x, hi.x), lo.y - 1.0),
                            3 => v(rng.range(lo.x, hi.x), hi.y + 1.0),
                            4 => v(hi.x + 1.0, lo.y - 1.0),
                            _ => v(hi.x + 1.0, rng.range(lo.y, hi.y)),
                        },
                    };
                    assert_v(
                        &label,
                        case,
                        (pair.c.clampv)(a, lo, hi),
                        (pair.rust.clampv)(a, lo, hi),
                    );
                }
            }
            16 => {
                for case in 0..CASES {
                    let nan = f32::from_bits(0x7fc0_0000 | (rng.next_u32() & 0x003f_ffff));
                    let (a, lo, hi) = match case % 4 {
                        0 => (
                            v(rng.finite(), rng.finite()),
                            v(10.0, 20.0),
                            v(-10.0, -20.0),
                        ),
                        1 => (v(rng.finite(), rng.finite()), v(3.0, -4.0), v(3.0, -4.0)),
                        2 => (v(nan, rng.finite()), v(-1.0, nan), v(1.0, 1.0)),
                        _ => (v(rng.finite(), nan), v(nan, -1.0), v(1.0, nan)),
                    };
                    assert_v(
                        "C16",
                        case,
                        (pair.c.clampv)(a, lo, hi),
                        (pair.rust.clampv)(a, lo, hi),
                    );
                }
            }
            17 => {
                for case in 0..CASES {
                    let a = if case % 5 == 0 {
                        v(0.0, -0.0)
                    } else {
                        v(rng.finite(), rng.finite())
                    };
                    let b = if case % 7 == 0 {
                        a
                    } else {
                        v(rng.finite(), rng.finite())
                    };
                    assert_v("C17", case, (pair.c.sub)(a, b), (pair.rust.sub)(a, b));
                }
            }
            18 => {
                for case in 0..CASES {
                    let a = v(
                        SPECIAL[case % SPECIAL.len()],
                        SPECIAL[(case * 3 + 1) % SPECIAL.len()],
                    );
                    let b = v(
                        SPECIAL[(case * 5 + 2) % SPECIAL.len()],
                        SPECIAL[(case * 7 + 3) % SPECIAL.len()],
                    );
                    assert_v("C18", case, (pair.c.sub)(a, b), (pair.rust.sub)(a, b));
                }
            }
            19 => {
                for case in 0..CASES {
                    let a = v(rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0));
                    let b = if case % 4 == 0 {
                        v(a.y, -a.x)
                    } else {
                        v(rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0))
                    };
                    assert_f32("C19", case, (pair.c.dot)(a, b), (pair.rust.dot)(a, b));
                }
            }
            20 => {
                for case in 0..CASES {
                    let a = v(
                        SPECIAL[case % SPECIAL.len()],
                        SPECIAL[(case * 3 + 1) % SPECIAL.len()],
                    );
                    let b = v(
                        SPECIAL[(case * 5 + 2) % SPECIAL.len()],
                        SPECIAL[(case * 7 + 3) % SPECIAL.len()],
                    );
                    assert_f32("C20", case, (pair.c.dot)(a, b), (pair.rust.dot)(a, b));
                }
            }
            _ => unreachable!(),
        }
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
    config_c01: 1,
    config_c02: 2,
    config_c03: 3,
    config_c04: 4,
    config_c05: 5,
    config_c06: 6,
    config_c07: 7,
    config_c08: 8,
    config_c09: 9,
    config_c10: 10,
    config_c11: 11,
    config_c12: 12,
    config_c13: 13,
    config_c14: 14,
    config_c15: 15,
    config_c16: 16,
    config_c17: 17,
    config_c18: 18,
    config_c19: 19,
    config_c20: 20,
);
