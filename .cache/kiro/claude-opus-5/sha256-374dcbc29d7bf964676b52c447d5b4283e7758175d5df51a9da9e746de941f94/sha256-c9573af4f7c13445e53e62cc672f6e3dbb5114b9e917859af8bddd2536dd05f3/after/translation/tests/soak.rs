//! High-volume soak: the same rows as `CONFIGS.md` C6 / C14 / C19, but with a
//! far larger randomized sample so that value-dependent divergences (a single
//! mishandled bit pattern, a non-wrapping add, a sign-extension slip) cannot
//! hide in a few thousand draws.
//!
//! Fixed seeds keep it reproducible. Sample sizes are tunable via
//! `STATICLOOP_SOAK` (default keeps the whole file well under a second).

mod common;

use common::{Rng, with_libs};
use std::ffi::c_int;

fn scale() -> u64 {
    std::env::var("STATICLOOP_SOAK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
}

/// C6 at scale: full-`int`-domain random updates into the shared accumulator.
#[test]
fn soak_static_sum_full_domain() {
    let n = 2_000_000 * scale();
    let mut rng = Rng::new(0xC0FF_EE00_0000_0006);
    with_libs(|h| {
        for _ in 0..n {
            let v = rng.next_c_int();
            let c = unsafe { (h.c.static_sum)(v) };
            let r = unsafe { (h.rust.static_sum)(v) };
            assert_eq!(c, r, "soak: static_sum({v}) diverged (C={c}, Rust={r})");
        }
    });
}

/// Walks the accumulator across every wrap boundary many times using only
/// extremal and near-extremal updates.
#[test]
fn soak_static_sum_boundary_biased() {
    let n = 200_000 * scale();
    let mut rng = Rng::new(0xC0FF_EE00_0000_0008);
    let pool: [c_int; 12] = [
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        c_int::MAX / 2,
        c_int::MIN / 2,
        1,
        -1,
        0,
        2,
        -2,
        c_int::MAX / 3,
    ];
    with_libs(|h| {
        for _ in 0..n {
            let v = pool[(rng.next_u32() as usize) % pool.len()];
            let c = unsafe { (h.c.static_sum)(v) };
            let r = unsafe { (h.rust.static_sum)(v) };
            assert_eq!(c, r, "soak: static_sum({v}) diverged (C={c}, Rust={r})");
        }
    });
}

/// C14 / C19 at scale: randomized interleaving of the wrapper and the
/// lowest-level entry point, comparing the exact stdout bytes each time.
#[test]
fn soak_driver_and_interleaving() {
    let n = 3_000 * scale();
    let mut rng = Rng::new(0xC0FF_EE00_0000_0019);
    with_libs(|h| {
        for _ in 0..n {
            match rng.next_u32() % 3 {
                0 => {
                    h.static_sum(rng.next_c_int(), "soak");
                }
                1 => {
                    h.driver(rng.next_c_int(), "soak");
                }
                _ => {
                    h.driver(rng.next_in_range(-1_000_000, 1_000_000) as c_int, "soak");
                }
            }
        }
    });
}

/// Every low byte value as a stride, from a freshly parked accumulator, so the
/// printed digit strings cover a dense contiguous range.
#[test]
fn soak_driver_dense_small_strides() {
    with_libs(|h| {
        for s in -300..=300 {
            h.park_accumulator_at(0, "soak-dense");
            h.driver(s, "soak-dense");
        }
    });
}
