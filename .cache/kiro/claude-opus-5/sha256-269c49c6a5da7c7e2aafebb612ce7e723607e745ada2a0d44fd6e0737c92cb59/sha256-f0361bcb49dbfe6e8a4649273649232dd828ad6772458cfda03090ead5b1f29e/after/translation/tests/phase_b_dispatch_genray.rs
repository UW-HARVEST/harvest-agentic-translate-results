//! Phase B — CONFIGS.md rows 49..60: `c2CastRay` (the runtime mode dispatcher)
//! and the `gen_ray` one-shot entry point driven end to end.

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 20_000;

// ===========================================================================
// c2CastRay — rows 49..52
// ===========================================================================

fn cmp_cast(l: &Pair, ray: C2Ray, buf: &[u8; 32], ty: i32, ctx: String) {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let p = buf.as_ptr() as *const c_void;
    let rc = unsafe { (l.c.c2CastRay)(ray, p, ty, &mut oc) };
    let rr = unsafe { (l.r.c2CastRay)(ray, p, ty, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
}

fn shape_buf<T: Copy>(v: T) -> [u8; 32] {
    let mut buf = [0u8; 32];
    assert!(std::mem::size_of::<T>() <= 32);
    unsafe {
        std::ptr::copy_nonoverlapping(
            &v as *const T as *const u8,
            buf.as_mut_ptr(),
            std::mem::size_of::<T>(),
        )
    };
    buf
}

#[test]
fn row49_castray_circle() {
    let l = libs();
    let mut rng = Rng::new(0x4949);
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.v(50.0),
            r: rng.unit() * 20.0,
        };
        let ray = C2Ray {
            p: rng.v(50.0),
            d: rng.unit_dir(),
            t: rng.unit() * 120.0,
        };
        cmp_cast(
            l,
            ray,
            &shape_buf(cir),
            C2_TYPE_CIRCLE,
            format!("row49 {ray:?} {cir:?}"),
        );
    }
    // specials
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.v_special(1e5),
            r: rng.special(1e5),
        };
        let ray = C2Ray {
            p: rng.v_special(1e5),
            d: rng.v_special(1e5),
            t: rng.special(1e5),
        };
        cmp_cast(
            l,
            ray,
            &shape_buf(cir),
            C2_TYPE_CIRCLE,
            format!("row49 special {ray:?} {cir:?}"),
        );
    }
}

#[test]
fn row50_castray_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x5050);
    for _ in 0..N {
        let c = rng.v(40.0);
        let h = C2v {
            x: rng.unit() * 20.0,
            y: rng.unit() * 20.0,
        };
        let b = C2AABB {
            min: C2v { x: c.x - h.x, y: c.y - h.y },
            max: C2v { x: c.x + h.x, y: c.y + h.y },
        };
        let p = rng.v(80.0);
        let dx = c.x - p.x;
        let dy = c.y - p.y;
        let dl = (dx * dx + dy * dy).sqrt();
        let ray = C2Ray {
            p,
            d: if dl > 0.0 {
                C2v { x: dx / dl, y: dy / dl }
            } else {
                rng.unit_dir()
            },
            t: dl * (0.5 + rng.unit() * 1.5),
        };
        cmp_cast(l, ray, &shape_buf(b), C2_TYPE_AABB, format!("row50 {ray:?} {b:?}"));
    }
    for _ in 0..N {
        let b = C2AABB {
            min: rng.v_special(1e5),
            max: rng.v_special(1e5),
        };
        let ray = C2Ray {
            p: rng.v_special(1e5),
            d: rng.v_special(1e5),
            t: rng.special(1e5),
        };
        cmp_cast(
            l,
            ray,
            &shape_buf(b),
            C2_TYPE_AABB,
            format!("row50 special {ray:?} {b:?}"),
        );
    }
}

#[test]
fn row51_castray_capsule() {
    let l = libs();
    let mut rng = Rng::new(0x5151);
    for _ in 0..N {
        let a = rng.v(30.0);
        let ang = rng.unit() * std::f32::consts::TAU;
        let ln = rng.unit() * 40.0 + 0.5;
        let cap = C2Capsule {
            a,
            b: C2v {
                x: a.x + ang.cos() * ln,
                y: a.y + ang.sin() * ln,
            },
            r: rng.unit() * 8.0 + 0.05,
        };
        let p = rng.v(60.0);
        let s = rng.unit();
        let tx = cap.a.x + (cap.b.x - cap.a.x) * s;
        let ty = cap.a.y + (cap.b.y - cap.a.y) * s;
        let (dx, dy) = (tx - p.x, ty - p.y);
        let dl = (dx * dx + dy * dy).sqrt();
        let ray = C2Ray {
            p,
            d: if dl > 0.0 {
                C2v { x: dx / dl, y: dy / dl }
            } else {
                rng.unit_dir()
            },
            t: dl * (0.4 + rng.unit() * 1.6),
        };
        cmp_cast(
            l,
            ray,
            &shape_buf(cap),
            C2_TYPE_CAPSULE,
            format!("row51 {ray:?} {cap:?}"),
        );
    }
    for _ in 0..N {
        let cap = C2Capsule {
            a: rng.v_special(1e5),
            b: rng.v_special(1e5),
            r: rng.special(1e5),
        };
        let ray = C2Ray {
            p: rng.v_special(1e5),
            d: rng.v_special(1e5),
            t: rng.special(1e5),
        };
        cmp_cast(
            l,
            ray,
            &shape_buf(cap),
            C2_TYPE_CAPSULE,
            format!("row51 special {ray:?} {cap:?}"),
        );
    }
}

/// row 52: identical 20-byte shape payload reinterpreted under all three valid
/// `typeB` values — checks that each arm reads the same bytes the C does.
#[test]
fn row52_castray_same_bytes_all_types() {
    let l = libs();
    let mut rng = Rng::new(0x5252);
    for _ in 0..N {
        let mut buf = [0u8; 32];
        for i in 0..20 {
            buf[i] = (rng.next_u32() & 0xFF) as u8;
        }
        // keep the bytes in a "sane float" range often enough to reach the
        // interesting branches, but keep raw-garbage cases too
        if rng.next_u32() % 2 == 0 {
            let vals = [
                rng.sym(20.0),
                rng.sym(20.0),
                rng.sym(20.0),
                rng.sym(20.0),
                rng.unit() * 10.0,
            ];
            for (i, v) in vals.iter().enumerate() {
                buf[i * 4..i * 4 + 4].copy_from_slice(&v.to_bits().to_le_bytes());
            }
        }
        let ray = C2Ray {
            p: rng.v(40.0),
            d: rng.unit_dir(),
            t: rng.unit() * 100.0,
        };
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            cmp_cast(l, ray, &buf, ty, format!("row52 ty={ty} bytes={buf:?} {ray:?}"));
        }
    }
}

// ===========================================================================
// gen_ray — rows 53..60
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn cmp_gen(l: &Pair, a: &[f32; 16], ctx: String) {
    let mut c1 = SENTINEL;
    let mut c2 = SENTINEL;
    let mut c3 = SENTINEL;
    let mut r1 = SENTINEL;
    let mut r2 = SENTINEL;
    let mut r3 = SENTINEL;
    let rc = unsafe {
        (l.c.gen_ray)(
            &mut c1, &mut c2, &mut c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
            a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            &mut r1, &mut r2, &mut r3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
            a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    diff_eq!(
        ctx,
        (rc, rcb(c1), rcb(c2), rcb(c3)),
        (rr, rcb(r1), rcb(r2), rcb(r3))
    );
    rc
    .to_le_bytes(); // keep rc used
}

fn gen_return(l: &Pair, a: &[f32; 16]) -> i32 {
    let mut c1 = SENTINEL;
    let mut c2 = SENTINEL;
    let mut c3 = SENTINEL;
    unsafe {
        (l.c.gen_ray)(
            &mut c1, &mut c2, &mut c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
            a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    }
}

/// A geometrically plausible argument vector.
fn gen_args(rng: &mut Rng) -> [f32; 16] {
    let rp = rng.v(30.0);
    let mp = C2v {
        x: rp.x + rng.sym(60.0),
        y: rp.y + rng.sym(60.0),
    };
    let cp = rng.v(40.0);
    let ca = rng.v(40.0);
    let cb = C2v {
        x: ca.x + rng.sym(30.0),
        y: ca.y + rng.sym(30.0),
    };
    let bc = rng.v(40.0);
    let bh = C2v {
        x: rng.unit() * 20.0,
        y: rng.unit() * 20.0,
    };
    [
        mp.x,
        mp.y,
        rp.x,
        rp.y,
        cp.x,
        cp.y,
        rng.unit() * 20.0,
        ca.x,
        ca.y,
        cb.x,
        cb.y,
        rng.unit() * 8.0 + 0.05,
        bc.x - bh.x,
        bc.y - bh.y,
        bc.x + bh.x,
        bc.y + bh.y,
    ]
}

// --- row 53: end-to-end random finite ------------------------------------
#[test]
fn row53_gen_ray_random() {
    let l = libs();
    let mut rng = Rng::new(0x5353);
    for _ in 0..N {
        let a = gen_args(&mut rng);
        cmp_gen(l, &a, format!("row53 {a:?}"));
    }
}

// --- row 54: every bitmask value 0..7 reached ----------------------------
#[test]
fn row54_gen_ray_all_bitmasks() {
    let l = libs();
    let mut rng = Rng::new(0x5454);
    let mut seen = [0usize; 8];
    for _ in 0..300_000 {
        let a = gen_args(&mut rng);
        let m = gen_return(l, &a);
        if (0..8).contains(&m) {
            seen[m as usize] += 1;
            // compare the first 400 samples of each mask value in full
            if seen[m as usize] <= 400 {
                cmp_gen(l, &a, format!("row54 mask={m} {a:?}"));
            }
        } else {
            cmp_gen(l, &a, format!("row54 unexpected mask={m} {a:?}"));
        }
    }
    for m in 0..8 {
        eprintln!("gen_ray bitmask {m} reached {} times", seen[m]);
        assert!(seen[m] > 0, "gen_ray never returned bitmask {m}");
    }
}

// --- row 55: mp == ray.p (zero-length ray -> NaN direction) --------------
#[test]
fn row55_gen_ray_zero_length() {
    let l = libs();
    let mut rng = Rng::new(0x5555);
    for _ in 0..N {
        let mut a = gen_args(&mut rng);
        a[0] = a[2];
        a[1] = a[3];
        cmp_gen(l, &a, format!("row55 {a:?}"));
    }
    // and the -0.0 / 0.0 variants
    for (x, y) in [(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        for _ in 0..2_000 {
            let mut a = gen_args(&mut rng);
            a[0] = x;
            a[1] = y;
            a[2] = -x;
            a[3] = -y;
            cmp_gen(l, &a, format!("row55 signedzero {a:?}"));
        }
    }
}

// --- row 56: all-zero arguments -----------------------------------------
#[test]
fn row56_gen_ray_all_zero() {
    let l = libs();
    let z = [0.0f32; 16];
    cmp_gen(l, &z, "row56 all-zero".to_string());
    let nz = [-0.0f32; 16];
    cmp_gen(l, &nz, "row56 all-negative-zero".to_string());
    // one non-zero slot at a time
    for i in 0..16 {
        for v in [1.0f32, -1.0, 1e-30, 1e30, f32::MIN_POSITIVE] {
            let mut a = [0.0f32; 16];
            a[i] = v;
            cmp_gen(l, &a, format!("row56 slot{i}={v}"));
        }
    }
}

// --- row 57: wide-magnitude arguments -----------------------------------
#[test]
fn row57_gen_ray_wide_magnitudes() {
    let l = libs();
    let mut rng = Rng::new(0x5757);
    for _ in 0..N {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.wide(30);
        }
        cmp_gen(l, &a, format!("row57 {a:?}"));
    }
    for _ in 0..N {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.wide(6);
        }
        cmp_gen(l, &a, format!("row57 mid {a:?}"));
    }
}

// --- row 58: full f32 bit space, specials --------------------------------
#[test]
fn row58_gen_ray_bit_space() {
    let l = libs();
    let mut rng = Rng::new(0x5858);
    for _ in 0..N {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.any_bits();
        }
        cmp_gen(l, &a, format!("row58 bits {a:?}"));
    }
    for _ in 0..N {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.special(1e6);
        }
        cmp_gen(l, &a, format!("row58 special {a:?}"));
    }
    // one special slot at a time on an otherwise plausible configuration
    for &s in &specials_nan_payloads() {
        for slot in 0..16 {
            for _ in 0..200 {
                let mut a = gen_args(&mut rng);
                a[slot] = s;
                cmp_gen(l, &a, format!("row58 slot{slot}={:#x} {a:?}", fb(s)));
            }
        }
    }
}

// --- row 59: aliasing out buffers ---------------------------------------
#[test]
fn row59_gen_ray_aliased_outputs() {
    let l = libs();
    let mut rng = Rng::new(0x5959);
    for _ in 0..N {
        let a = gen_args(&mut rng);
        // all three pointing at one buffer
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe {
            (l.c.gen_ray)(
                &mut oc, &mut oc, &mut oc, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
            )
        };
        let rr = unsafe {
            (l.r.gen_ray)(
                &mut or, &mut or, &mut or, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
            )
        };
        diff_eq!(format!("row59 alias3 {a:?}"), (rc, rcb(oc)), (rr, rcb(or)));

        // two aliased, one separate (each pairing)
        for pair in 0..3 {
            let mut xc = SENTINEL;
            let mut yc = SENTINEL;
            let mut xr = SENTINEL;
            let mut yr = SENTINEL;
            let (rc, rr) = unsafe {
                let (c1, c2, c3): (*mut C2Raycast, *mut C2Raycast, *mut C2Raycast) = match pair {
                    0 => (&mut xc, &mut xc, &mut yc),
                    1 => (&mut xc, &mut yc, &mut xc),
                    _ => (&mut yc, &mut xc, &mut xc),
                };
                let (r1, r2, r3): (*mut C2Raycast, *mut C2Raycast, *mut C2Raycast) = match pair {
                    0 => (&mut xr, &mut xr, &mut yr),
                    1 => (&mut xr, &mut yr, &mut xr),
                    _ => (&mut yr, &mut xr, &mut xr),
                };
                (
                    (l.c.gen_ray)(
                        c1, c2, c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9],
                        a[10], a[11], a[12], a[13], a[14], a[15],
                    ),
                    (l.r.gen_ray)(
                        r1, r2, r3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9],
                        a[10], a[11], a[12], a[13], a[14], a[15],
                    ),
                )
            };
            diff_eq!(
                format!("row59 pair{pair} {a:?}"),
                (rc, rcb(xc), rcb(yc)),
                (rr, rcb(xr), rcb(yr))
            );
        }
    }
}

// --- row 60: sentinel prefill, untouched fields compared ----------------
#[test]
fn row60_gen_ray_sentinel_untouched() {
    let l = libs();
    let mut rng = Rng::new(0x6060);
    // Configurations where all three sub-casts miss, so all three out structs
    // must retain the sentinel identically.
    let mut misses = 0usize;
    for _ in 0..200_000 {
        let mut a = gen_args(&mut rng);
        // push the shapes far away from the ray
        a[4] = 1e5;
        a[5] = 1e5;
        a[6] = 0.01;
        a[7] = -1e5;
        a[8] = -1e5;
        a[9] = -1e5 + 1.0;
        a[10] = -1e5 + 1.0;
        a[11] = 0.01;
        a[12] = 5e4;
        a[13] = 5e4;
        a[14] = 5e4 + 1.0;
        a[15] = 5e4 + 1.0;
        let m = gen_return(l, &a);
        cmp_gen(l, &a, format!("row60 miss m={m} {a:?}"));
        if m == 0 {
            misses += 1;
        }
        if misses > 5_000 {
            break;
        }
    }
    assert!(misses > 0, "no all-miss configuration generated");
    eprintln!("row60: {misses} all-miss configurations compared");
}
