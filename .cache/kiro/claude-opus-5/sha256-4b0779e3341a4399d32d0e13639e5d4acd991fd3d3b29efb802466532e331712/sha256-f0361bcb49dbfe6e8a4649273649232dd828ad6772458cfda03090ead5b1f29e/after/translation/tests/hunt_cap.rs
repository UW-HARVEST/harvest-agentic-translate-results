//! Targeted hunt: can `c2GJK`'s `while (iter < 20)` cap actually fire?
//!
//! This matters because exiting the loop via the cap is the ONLY way the C can
//! leave `verts[s.count-1].u` unwritten while `s.count >= 2` (every other exit
//! runs `c22`/`c23` first, which writes every live `u`). If the cap fires, the C
//! reads uninitialised stack there while the Rust reads a zeroed field.

mod common;

use common::*;
use std::ffi::c_void;

#[allow(clippy::type_complexity)]
type GjkFn = unsafe extern "C" fn(
    *const c_void,
    i32,
    *const c2x,
    *const c_void,
    i32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;

fn wild(rng: &mut Rng) -> f32 {
    match rng.below(10) {
        0 => 0.0,
        1 => -0.0,
        2 => f32::from_bits(rng.next_u32()),
        3 => f32::from_bits(rng.next_u32() & 0x807f_ffff), // signed subnormal
        4 => rng.range(-1.0, 1.0) * 3.0e38,
        5 => FLT_EPSILON * rng.range(-8.0, 8.0),
        6 => (rng.next_u32() % 5) as f32 - 2.0,
        _ => rng.range(-50.0, 50.0),
    }
}

fn wild_v(rng: &mut Rng) -> c2v {
    c2v {
        x: wild(rng),
        y: wild(rng),
    }
}

#[test]
fn hunt_gjk_iteration_cap() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x1CA9_u64.wrapping_mul(0x9E3779B97F4A7C15));

    let mut hist = std::collections::BTreeMap::<i32, usize>::new();
    let mut worst: Option<String> = None;
    let mut worst_iter = i32::MIN;

    for i in 0..1_500_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let circ = c2Circle {
            p: wild_v(&mut rng),
            r: wild(&mut rng),
        };
        let aabb = c2AABB {
            min: wild_v(&mut rng),
            max: wild_v(&mut rng),
        };
        let cap = c2Capsule {
            a: wild_v(&mut rng),
            b: wild_v(&mut rng),
            r: wild(&mut rng),
        };
        let circ2 = c2Circle {
            p: wild_v(&mut rng),
            r: wild(&mut rng),
        };
        let aabb2 = c2AABB {
            min: wild_v(&mut rng),
            max: wild_v(&mut rng),
        };
        let cap2 = c2Capsule {
            a: wild_v(&mut rng),
            b: wild_v(&mut rng),
            r: wild(&mut rng),
        };
        let sel = |t: i32, c: &c2Circle, a: &c2AABB, p: &c2Capsule| -> *const c_void {
            match t {
                C2_TYPE_CIRCLE => c as *const c2Circle as *const c_void,
                C2_TYPE_AABB => a as *const c2AABB as *const c_void,
                _ => p as *const c2Capsule as *const c_void,
            }
        };
        let pa = sel(ta, &circ, &aabb, &cap);
        let pb = sel(tb, &circ2, &aabb2, &cap2);
        let ax = c2x {
            p: wild_v(&mut rng),
            r: c2r {
                c: wild(&mut rng),
                s: wild(&mut rng),
            },
        };
        let use_radius = rng.below(2) as i32;

        unsafe {
            let (mut ca, mut cb, mut ci) = (c2v::default(), c2v::default(), -1i32);
            let (mut ra, mut rb, mut ri) = (c2v::default(), c2v::default(), -1i32);
            let axp = if rng.below(2) == 0 {
                std::ptr::null()
            } else {
                &ax as *const c2x
            };
            let cd = c_f(
                pa,
                ta,
                axp,
                pb,
                tb,
                std::ptr::null(),
                &mut ca,
                &mut cb,
                use_radius,
                &mut ci,
                std::ptr::null_mut(),
            );
            let rd = r_f(
                pa,
                ta,
                axp,
                pb,
                tb,
                std::ptr::null(),
                &mut ra,
                &mut rb,
                use_radius,
                &mut ri,
                std::ptr::null_mut(),
            );
            let ctx = format!(
                "#{i} ta={ta} tb={tb} ur={use_radius} A={circ:?}/{aabb:?}/{cap:?} \
                 B={circ2:?}/{aabb2:?}/{cap2:?} ax={:?}",
                if axp.is_null() { None } else { Some(ax) }
            );
            assert_f32_bits!(cd, rd, "hunt dist :: {ctx}");
            assert_v_bits!(ca, ra, "hunt outA :: {ctx}");
            assert_v_bits!(cb, rb, "hunt outB :: {ctx}");
            assert_eq!(ci, ri, "hunt iterations :: {ctx}");

            *hist.entry(ci).or_insert(0) += 1;
            if ci > worst_iter {
                worst_iter = ci;
                worst = Some(ctx);
            }
        }
    }

    println!("iteration-count histogram over 1.5M randomized c2GJK calls:");
    for (k, v) in &hist {
        println!("  iter={k:>3}  {v}");
    }
    println!("max iterations observed: {worst_iter}");
    if worst_iter >= 19 {
        println!("WORST CASE INPUT: {}", worst.unwrap());
    }
    assert!(
        worst_iter <= 20,
        "iterations exceeded the C's own cap of 20: {worst_iter}"
    );
}
