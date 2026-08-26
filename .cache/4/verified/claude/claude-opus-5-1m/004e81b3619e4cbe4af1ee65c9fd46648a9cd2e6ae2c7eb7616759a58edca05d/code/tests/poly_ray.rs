//! Phase B — CONFIGS.md rows 98..100 (`poly_ray`, the only symbol declared in
//! the public header `c_src/include/lib.h`).

#![allow(non_snake_case)]

mod common;
use common::*;

// --- rows 98 & 100 -------------------------------------------------------
#[test]
fn row98_row100_poly_ray_bit_exact() {
    let (c, r) = (c(), rs());
    for seed in [
        0x0000_0000u32,
        0xffff_ffff,
        0x5555_5555,
        0xaaaa_aaaa,
        0xdead_beef,
        0x0bad_f00d,
        0x7fc0_0000,
        1,
    ] {
        let mut c1c = poison(seed);
        let mut c2c = poison(seed ^ 0x1234_5678);
        let mut c1r = poison(seed);
        let mut c2r = poison(seed ^ 0x1234_5678);

        let rc = unsafe { (c.poly_ray)(&mut c1c, &mut c2c) };
        let rr = unsafe { (r.poly_ray)(&mut c1r, &mut c2r) };

        assert_eq!(
            rc, rr,
            "poly_ray return value: C={rc} RUST={rr} (poison 0x{seed:08x})"
        );
        assert!(
            rceq(c1c, c1r),
            "poly_ray cast1: C={} RUST={} (poison 0x{seed:08x})",
            rcshow(c1c),
            rcshow(c1r)
        );
        assert!(
            rceq(c2c, c2r),
            "poly_ray cast2: C={} RUST={} (poison 0x{seed:08x})",
            rcshow(c2c),
            rcshow(c2r)
        );
    }
}

/// Also compare the raw bytes of the two out-parameters (row 100).
#[test]
fn row100_poly_ray_raw_bytes() {
    let (c, r) = (c(), rs());
    for seed in [0u32, 0xffff_ffff, 0x3c3c_3c3c] {
        let mut c1c = poison(seed);
        let mut c2c = poison(seed);
        let mut c1r = poison(seed);
        let mut c2r = poison(seed);
        let rc = unsafe { (c.poly_ray)(&mut c1c, &mut c2c) };
        let rr = unsafe { (r.poly_ray)(&mut c1r, &mut c2r) };
        assert_eq!(rc, rr);

        let as_bytes = |x: &C2Raycast| -> Vec<u8> {
            unsafe {
                std::slice::from_raw_parts(
                    (x as *const C2Raycast) as *const u8,
                    std::mem::size_of::<C2Raycast>(),
                )
                .to_vec()
            }
        };
        // cast1 is written on a hit, so its bytes must be byte-identical.
        assert_eq!(
            as_bytes(&c1c),
            as_bytes(&c1r),
            "poly_ray cast1 raw bytes differ: C={} RUST={}",
            rcshow(c1c),
            rcshow(c1r)
        );
        // cast2: whichever way the C leaves it, the Rust must match.
        assert_eq!(
            as_bytes(&c2c),
            as_bytes(&c2r),
            "poly_ray cast2 raw bytes differ: C={} RUST={}",
            rcshow(c2c),
            rcshow(c2r)
        );
    }
}

// --- row 99: repeated calls, no hidden global state --------------------
//
// Ground truth (measured from the C `.so`): on the hard-coded geometry
//   poly   = box x in [-0.875, 0.875], y in [-11.5, 11.5]
//   ray0   = p (-3.869416, 13.0693407), d (1, 0),  t 4
//   ray1   = p (-3.869416, 13.0693407), d (0, -1), t 4
// *both* casts miss, so `poly_ray` returns 0 and leaves **both** out-params
// completely untouched.  The Rust translation must do exactly the same, i.e.
// the poison pattern must survive both calls identically.
#[test]
fn row99_poly_ray_idempotent() {
    let (c, r) = (c(), rs());
    let mut first_ret: Option<(i32, i32)> = None;
    for i in 0..1000 {
        let seed = (i as u32).wrapping_mul(0x9e37_79b9);
        let mut c1c = poison(seed);
        let mut c2c = poison(seed);
        let mut c1r = poison(seed);
        let mut c2r = poison(seed);
        let rc = unsafe { (c.poly_ray)(&mut c1c, &mut c2c) };
        let rr = unsafe { (r.poly_ray)(&mut c1r, &mut c2r) };
        assert_eq!(rc, rr, "iteration {i}: return value");
        assert!(rceq(c1c, c1r), "iteration {i}: cast1");
        assert!(rceq(c2c, c2r), "iteration {i}: cast2");

        // Whatever the C does to each buffer, the Rust must do the same: if the
        // C left the poison in place, so must the Rust, and vice versa.
        let c1_written = !rceq(c1c, poison(seed));
        let r1_written = !rceq(c1r, poison(seed));
        assert_eq!(
            c1_written, r1_written,
            "iteration {i}: cast1 written by C={c1_written} but by RUST={r1_written}"
        );
        let c2_written = !rceq(c2c, poison(seed));
        let r2_written = !rceq(c2r, poison(seed));
        assert_eq!(
            c2_written, r2_written,
            "iteration {i}: cast2 written by C={c2_written} but by RUST={r2_written}"
        );

        // No hidden global state: the return value is stable across calls.
        match first_ret {
            None => first_ret = Some((rc, rr)),
            Some((rc0, rr0)) => {
                assert_eq!(rc0, rc, "C: poly_ray is not idempotent");
                assert_eq!(rr0, rr, "RUST: poly_ray is not idempotent");
            }
        }
    }
    // Pin the measured ground truth so a future regression is loud.
    let (rc0, _) = first_ret.unwrap();
    assert_eq!(rc0, 0, "C poly_ray ground truth changed");
}

/// Aliasing: pass the *same* buffer as both out-parameters.
#[test]
fn row98b_poly_ray_aliased_outputs() {
    let (c, r) = (c(), rs());
    for seed in [0u32, 0xffff_ffff] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe {
            let p: *mut C2Raycast = &mut oc;
            (c.poly_ray)(p, p)
        };
        let rr = unsafe {
            let p: *mut C2Raycast = &mut orr;
            (r.poly_ray)(p, p)
        };
        assert_eq!(rc, rr, "aliased poly_ray return: C={rc} RUST={rr}");
        assert!(
            rceq(oc, orr),
            "aliased poly_ray out: C={} RUST={}",
            rcshow(oc),
            rcshow(orr)
        );
    }
}
