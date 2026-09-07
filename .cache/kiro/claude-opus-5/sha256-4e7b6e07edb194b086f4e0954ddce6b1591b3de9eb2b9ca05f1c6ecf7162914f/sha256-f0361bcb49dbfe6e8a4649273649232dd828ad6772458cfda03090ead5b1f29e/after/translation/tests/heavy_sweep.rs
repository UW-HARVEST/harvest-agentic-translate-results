//! Heavy differential sweeps. Cheap enough to run in the normal suite, but
//! covering orders of magnitude more of the input space than the per-row
//! tests: a strided exhaustive walk over the full 32-bit `radius` space and a
//! dense walk over the `size` space.

mod common;

use common::*;
use std::os::raw::c_int;

/// Strided exhaustive walk over every `f32` bit pattern for `radius`.
///
/// The stride is prime, so the walk visits every exponent field and a
/// well-spread set of mantissas, including all the infinity and NaN encodings
/// it lands on. Override with `SWEEP_STRIDE=1` for a full 2^32 exhaustive run
/// (takes ~35 min; the default keeps the suite under a minute).
#[test]
fn sweep_radius_bit_space_strided() {
    let stride: u64 = std::env::var("SWEEP_STRIDE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&s| s >= 1)
        .unwrap_or(101);
    let sizes: [c_int; 5] = [1, 2, 3, 4, 7];
    let mut bufs: Vec<(usize, Vec<f32>, Vec<f32>)> = sizes
        .iter()
        .map(|&s| {
            let n = buffer_len(s);
            (n, vec![poison(); n], vec![poison(); n])
        })
        .collect();

    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let mut bits: u64 = 0;
    let mut count: u64 = 0;
    while bits < (1u64 << 32) {
        let radius = f32::from_bits(bits as u32);
        for (si, &size) in sizes.iter().enumerate() {
            let (n, cbuf, rbuf) = &mut bufs[si];
            cbuf[..*n].fill(poison());
            rbuf[..*n].fill(poison());
            // SAFETY: buffers are sized by `buffer_len`, which accounts for the
            // C's one-past-the-end write on even `size`.
            unsafe {
                cf(cbuf.as_mut_ptr(), size, radius);
                rf(rbuf.as_mut_ptr(), size, radius);
            }
            for i in 0..*n {
                let (cv, rv) = (cbuf[i].to_bits(), rbuf[i].to_bits());
                if cv != rv {
                    panic!(
                        "DIVERGENCE sweep: radius bits {:#010x} ({radius:e}) size={size} \
                         slot[{i}]: C={cv:#010x} RUST={rv:#010x}",
                        bits as u32
                    );
                }
            }
        }
        bits += stride;
        count += 1;
    }
    let expected = (1u64 << 32).div_ceil(stride);
    assert_eq!(count, expected, "walked {count} radii, expected {expected}");
    eprintln!("swept {count} radius bit patterns x {} sizes", sizes.len());
}

/// Dense walk over `size` from the degenerate negatives up through a few
/// thousand, crossed with a handful of radius regimes.
#[test]
fn sweep_size_space_dense() {
    let radii: [f32; 12] = [
        1.0, 1.6, 2.4, 0.5, 0.01, 1e-8, 1e-40, 1e8, f32::MAX, 0.0, f32::NAN, -3.25,
    ];
    for size in -8i32..=2048 {
        for &radius in &radii {
            check(size, radius, "sweep size dense");
        }
    }
}

/// Fully random joint fuzz: random `size` bit pattern restricted to an
/// allocatable range, random `radius` bit pattern including NaN/inf.
#[test]
fn fuzz_joint_random_bits() {
    let mut rng = Rng::new(0xF00D_BEEF);
    for _ in 0..200_000 {
        let size = rng.int_in(-6, 300) as c_int;
        let radius = f32::from_bits(rng.next_u32());
        check(size, radius, "fuzz joint random bits");
    }
}

/// Random fuzz for the no-write regime with a null destination: `size <= -2`
/// over the whole negative int range, arbitrary radius bit patterns.
#[test]
fn fuzz_null_dest_no_write_regime() {
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let mut rng = Rng::new(0x0BAD_F00D);
    for _ in 0..200_000 {
        let size = (i32::MIN as i64 + rng.int_in(0, (i32::MAX as i64) - 1)) as c_int;
        debug_assert!(size <= -2);
        let radius = f32::from_bits(rng.next_u32());
        // SAFETY: `size <= -2` provably performs no dereference in the C.
        unsafe {
            cf(std::ptr::null_mut(), size, radius);
            rf(std::ptr::null_mut(), size, radius);
        }
    }
}
