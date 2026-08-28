//! Phase B, Group 5 — CONFIGS.md rows C66..C72 (`gjk_cache`, the only symbol
//! declared in `c_src/include/lib.h`), plus ERRORS.md rows E10, E11, E85..E87.
//!
//! `gjk_cache` returns `void` and — this is a quirk of the C, not an oversight
//! in the translation — never dereferences its `a9` / `b9` out-parameters.  Its
//! entire observable contract across the FFI boundary is therefore:
//!
//!   * it must return normally for every input (no crash, no trap), and
//!   * it must leave `*a9` / `*b9` (and the memory around them) untouched.
//!
//! Both properties are checked here for the whole parameter surface, using a
//! guarded buffer so that an out-of-bounds write would be caught too.

mod common;
use common::*;

const N: u32 = 4096;

/// `a9`/`b9` plus canary padding, so any stray write is detected.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Guarded {
    front: [u32; 4],
    a9: C2v,
    mid: [u32; 4],
    b9: C2v,
    back: [u32; 4],
}

fn guarded(seed: u32) -> Guarded {
    let p = |k: u32| 0xA5A5_0000u32 ^ seed.wrapping_mul(2_654_435_761).wrapping_add(k);
    Guarded {
        front: [p(1), p(2), p(3), p(4)],
        a9: C2v {
            x: f32::from_bits(p(5)),
            y: f32::from_bits(p(6)),
        },
        mid: [p(7), p(8), p(9), p(10)],
        b9: C2v {
            x: f32::from_bits(p(11)),
            y: f32::from_bits(p(12)),
        },
        back: [p(13), p(14), p(15), p(16)],
    }
}

#[allow(clippy::too_many_arguments)]
#[track_caller]
fn diff(reverse: i8, seed: u32, p: [f32; 9], ctx: &str) {
    let (c, r): (FnGjkCache, FnGjkCache) = sym(b"gjk_cache");
    let before = guarded(seed);
    let mut gc = before;
    let mut gr = before;
    unsafe {
        c(
            reverse as core::ffi::c_char,
            &mut gc.a9,
            &mut gc.b9,
            p[0],
            p[1],
            p[2],
            p[3],
            p[4],
            p[5],
            p[6],
            p[7],
            p[8],
        )
    };
    unsafe {
        r(
            reverse as core::ffi::c_char,
            &mut gr.a9,
            &mut gr.b9,
            p[0],
            p[1],
            p[2],
            p[3],
            p[4],
            p[5],
            p[6],
            p[7],
            p[8],
        )
    };
    assert!(
        raw_same(&gc, &gr),
        "gjk_cache buffer mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_bytes(raw(&gc)),
        fmt_bytes(raw(&gr))
    );
    // The C never writes through a9/b9 -- neither may the Rust.
    assert!(
        raw_same(&before, &gc),
        "the C wrote through a9/b9 (unexpected) [{ctx}]"
    );
    assert!(
        raw_same(&before, &gr),
        "the Rust wrote through a9/b9 [{ctx}]\n  before = {}\n  after  = {}",
        fmt_bytes(raw(&before)),
        fmt_bytes(raw(&gr))
    );
    // Also exercise the NULL a9/b9 form with the same parameters.
    unsafe {
        c(
            reverse as core::ffi::c_char,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            p[0],
            p[1],
            p[2],
            p[3],
            p[4],
            p[5],
            p[6],
            p[7],
            p[8],
        )
    };
    unsafe {
        r(
            reverse as core::ffi::c_char,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            p[0],
            p[1],
            p[2],
            p[3],
            p[4],
            p[5],
            p[6],
            p[7],
            p[8],
        )
    };
}

fn rand_params(rng: &mut Rng) -> [f32; 9] {
    // A well-formed AABB (a1..a4) plus a capsule (b1..b5), the shapes the C
    // builds internally.
    let x0 = rng.range(-300.0, 300.0);
    let y0 = rng.range(-300.0, 300.0);
    let w = rng.range(0.0, 200.0);
    let h = rng.range(0.0, 200.0);
    [
        x0,
        y0,
        x0 + w,
        y0 + h,
        rng.range(-300.0, 300.0),
        rng.range(-300.0, 300.0),
        rng.range(-300.0, 300.0),
        rng.range(-300.0, 300.0),
        rng.range(0.0, 60.0),
    ]
}

// ---------------------------------------------------------------------------
// C66 / C67 — both `reverse` branches with random valid parameters
// ---------------------------------------------------------------------------

#[test]
fn c66_reverse_zero() {
    let mut rng = Rng::new(0xC66);
    for i in 0..N {
        diff(0, i, rand_params(&mut rng), &format!("reverse=0 #{i}"));
    }
}

#[test]
fn c67_reverse_one() {
    let mut rng = Rng::new(0xC67);
    for i in 0..N {
        diff(1, i, rand_params(&mut rng), &format!("reverse=1 #{i}"));
    }
}

// ---------------------------------------------------------------------------
// C68 / E85 — non-boolean `char` values for `reverse`
// ---------------------------------------------------------------------------

#[test]
fn c68_reverse_non_boolean() {
    let mut rng = Rng::new(0xC68);
    for &rev in &[-1i8, 2, 0x7F, -128, 3, -3, 0x10] {
        for i in 0..256 {
            diff(rev, i, rand_params(&mut rng), &format!("reverse={rev} #{i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C69 / E10 — NULL a9 / b9
// ---------------------------------------------------------------------------

#[test]
fn c69_null_out_pointers() {
    let (c, r): (FnGjkCache, FnGjkCache) = sym(b"gjk_cache");
    let mut rng = Rng::new(0xC69);
    for rev in [0i8, 1, -1, 2] {
        for _ in 0..512 {
            let p = rand_params(&mut rng);
            // all four NULL / non-NULL combinations
            let mut a = C2v { x: 1.0, y: 2.0 };
            let mut b = C2v { x: 3.0, y: 4.0 };
            for (pa, pb) in [
                (std::ptr::null_mut(), std::ptr::null_mut()),
                (&mut a as *mut C2v, std::ptr::null_mut()),
                (std::ptr::null_mut(), &mut b as *mut C2v),
                (&mut a as *mut C2v, &mut b as *mut C2v),
            ] {
                unsafe {
                    c(
                        rev as core::ffi::c_char,
                        pa,
                        pb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
                unsafe {
                    r(
                        rev as core::ffi::c_char,
                        pa,
                        pb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
            }
            assert!(v_same(a, C2v { x: 1.0, y: 2.0 }), "a9 was written");
            assert!(v_same(b, C2v { x: 3.0, y: 4.0 }), "b9 was written");
        }
    }
}

// ---------------------------------------------------------------------------
// C70 / E87 — degenerate parameters
// ---------------------------------------------------------------------------

#[test]
fn c70_degenerate_params() {
    let mut rng = Rng::new(0xC70);
    for i in 0..N {
        let x = rng.range(-100.0, 100.0);
        let y = rng.range(-100.0, 100.0);
        let cases: [[f32; 9]; 6] = [
            // inverted AABB
            [x + 50.0, y + 50.0, x - 50.0, y - 50.0, x, y, x + 10.0, y + 10.0, 5.0],
            // empty AABB
            [x, y, x, y, x + 1.0, y + 1.0, x + 2.0, y + 2.0, 3.0],
            // zero-length capsule
            [x - 5.0, y - 5.0, x + 5.0, y + 5.0, x, y, x, y, 4.0],
            // zero-radius capsule
            [x - 5.0, y - 5.0, x + 5.0, y + 5.0, x, y, x + 9.0, y, 0.0],
            // negative-radius capsule
            [x - 5.0, y - 5.0, x + 5.0, y + 5.0, x, y, x + 9.0, y, -7.0],
            // everything collapsed to the same point
            [x, y, x, y, x, y, x, y, 0.0],
        ];
        for (k, p) in cases.iter().enumerate() {
            for rev in [0i8, 1] {
                diff(rev, i, *p, &format!("degenerate case {k} rev={rev} #{i}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C71 / E86 — extreme float parameters
// ---------------------------------------------------------------------------

#[test]
fn c71_extreme_params() {
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        FLT_MAX,
        -FLT_MAX,
        FLT_MIN,
        1e-45,
        FLT_EPSILON,
        1e8,
        -1e8,
    ];
    let mut rng = Rng::new(0xC71);
    let mut i = 0u32;
    // Sweep each parameter slot through the special table with the other slots
    // taking random values, and also fill every slot with specials at once.
    for slot in 0..9usize {
        for &s in &specials {
            for _ in 0..8 {
                let mut p = rand_params(&mut rng);
                p[slot] = s;
                for rev in [0i8, 1] {
                    diff(rev, i, p, &format!("extreme slot {slot}={} rev={rev}", fmt_f32(s)));
                }
                i += 1;
            }
        }
    }
    for _ in 0..N {
        let mut p = [0.0f32; 9];
        for v in p.iter_mut() {
            *v = rng.spicy();
        }
        for rev in [0i8, 1] {
            diff(rev, i, p, "all-spicy params");
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// C72 — full random sweep over both branches
// ---------------------------------------------------------------------------

#[test]
fn c72_random_sweep() {
    let mut rng = Rng::new(0xC72);
    for i in 0..4096u32 {
        let rev = rng.next_u32() as i8;
        let mut p = [0.0f32; 9];
        for v in p.iter_mut() {
            *v = match rng.below(3) {
                0 => rng.range(-1e4, 1e4),
                1 => rng.finite(),
                _ => rng.spicy(),
            };
        }
        diff(rev, i, p, &format!("random sweep #{i} rev={rev}"));
    }
}
