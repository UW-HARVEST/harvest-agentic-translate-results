//! Level 4: `gjk`, the only function in `include/lib.h`.

#![allow(non_snake_case)]

mod harness;
use harness::*;

type FnGjk = unsafe extern "C" fn(
    std::ffi::c_char,
    *mut V,
    *mut V,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
);

#[derive(Clone, Copy)]
struct Args {
    reverse: i8,
    p: [f32; 9],
}

impl std::fmt::Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "gjk(reverse={}, aabb=[{:?},{:?},{:?},{:?}], cap=[{:?},{:?},{:?},{:?}], r={:?})",
            self.reverse,
            self.p[0],
            self.p[1],
            self.p[2],
            self.p[3],
            self.p[4],
            self.p[5],
            self.p[6],
            self.p[7],
            self.p[8]
        )
    }
}

fn call(f: &FnGjk, a: &Args, out_null: bool) -> (V, V) {
    let mut oa = V { x: 3.5, y: -11.0 };
    let mut ob = V { x: -2.25, y: 6.0 };
    unsafe {
        f(
            a.reverse,
            if out_null {
                std::ptr::null_mut()
            } else {
                &mut oa
            },
            if out_null {
                std::ptr::null_mut()
            } else {
                &mut ob
            },
            a.p[0],
            a.p[1],
            a.p[2],
            a.p[3],
            a.p[4],
            a.p[5],
            a.p[6],
            a.p[7],
            a.p[8],
        )
    };
    (oa, ob)
}

#[track_caller]
fn check(f_c: &FnGjk, f_r: &FnGjk, args: &Args) {
    let (ca, cb) = call(f_c, args, false);
    let (ra, rb) = call(f_r, args, false);
    assert_v("gjk outA", args, ca, ra);
    assert_v("gjk outB", args, cb, rb);
    // Null outputs must not crash either side.
    call(f_c, args, true);
    call(f_r, args, true);
}

fn sweep(seed: u64, n0: u32, make: impl Fn(&mut Rng) -> Args) {
    let (c, r) = pair::<FnGjk>("gjk");
    let mut rng = Rng::new(seed);
    for _ in 0..volume(n0) {
        let args = make(&mut rng);
        check(&c, &r, &args);
    }
}

fn reverse_byte(rng: &mut Rng) -> i8 {
    match rng.below(4) {
        0 => 0,
        1 => 1,
        2 => -1,
        _ => (rng.next_u32() & 0xff) as u8 as i8,
    }
}

#[test]
fn gjk_matches_random() {
    sweep(201, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.float()),
    });
}

#[test]
fn gjk_matches_moderate_geometry() {
    sweep(202, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.unit() * 10.0),
    });
}

#[test]
fn gjk_matches_integer_geometry() {
    // Lots of exact coincidences: shared corners, zero-area boxes, degenerate
    // capsules, radii that exactly cover the gap.
    sweep(203, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| (rng.below(9) as f32) - 4.0),
    });
}

#[test]
fn gjk_matches_half_integer_geometry() {
    sweep(204, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| ((rng.below(17) as f32) - 8.0) * 0.5),
    });
}

#[test]
fn gjk_matches_epsilon_scale() {
    sweep(205, 40_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.unit() * 1.0e-7),
    });
}

#[test]
fn gjk_matches_denormal_scale() {
    sweep(206, 40_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.unit() * 1.0e-40),
    });
}

#[test]
fn gjk_matches_large_scale() {
    sweep(207, 40_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.unit() * 1.0e20),
    });
}

#[test]
fn gjk_matches_overflow_scale() {
    // Squared distances become +inf here.
    sweep(208, 40_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.unit() * 1.0e35),
    });
}

#[test]
fn gjk_matches_bit_pattern_fuzz() {
    // Arbitrary bit patterns, so NaN and infinity reach every branch.
    sweep(209, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| f32::from_bits(rng.next_u32())),
    });
}

#[test]
fn gjk_matches_mixed_magnitudes() {
    sweep(210, 60_000, |rng| Args {
        reverse: reverse_byte(rng),
        p: std::array::from_fn(|_| rng.float()),
    });
}

/// A capsule sliding past a fixed unit box on an exhaustive lattice, both
/// orientations, radii spanning "not touching" to "fully engulfing".
#[test]
fn gjk_matches_exhaustive_lattice() {
    let (c, r) = pair::<FnGjk>("gjk");
    let step = 0.5f32;
    let mut n = 0u64;
    for rev in [0i8, 1i8] {
        let mut ix = -6i32;
        while ix <= 6 {
            let mut iy = -6i32;
            while iy <= 6 {
                for &(dx, dy) in &[
                    (0.0f32, 0.0f32),
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (1.5, 1.5),
                    (-1.0, 2.0),
                    (2.0, -0.5),
                ] {
                    for &rad in &[0.0f32, 0.25, 1.0, 3.0] {
                        let bx = ix as f32 * step;
                        let by = iy as f32 * step;
                        let args = Args {
                            reverse: rev,
                            p: [
                                -1.0,
                                -1.0,
                                1.0,
                                1.0,
                                bx,
                                by,
                                bx + dx,
                                by + dy,
                                rad,
                            ],
                        };
                        check(&c, &r, &args);
                        n += 1;
                    }
                }
                iy += 1;
            }
            ix += 1;
        }
    }
    assert!(n > 4000, "lattice sweep covered only {n} cases");
}
