//! Differential tests: `gaussian_kernel` is the only symbol the C library
//! exports, so it is exercised directly across sizes, radii and edge cases.

mod common;

use common::{Impls, assert_same};

#[test]
fn exports_match_c() {
    // Loading both symbols through the C ABI is itself the export check.
    let _ = Impls::load();
}

#[test]
fn typical_sizes_and_radii() {
    let impls = Impls::load();
    for size in 0..=33 {
        for radius in [0.5f32, 1.0, 1.6, 2.0, 3.0, 7.5, 16.0, 100.0, 1e6] {
            assert_same(&impls, size, radius);
        }
    }
}

#[test]
fn odd_and_even_sizes_large() {
    let impls = Impls::load();
    for size in [63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024, 4095, 4096] {
        for radius in [1.0f32, 2.5, 9.0, 1234.5] {
            assert_same(&impls, size, radius);
        }
    }
}

#[test]
fn negative_and_zero_sizes() {
    let impls = Impls::load();
    for size in [-1, -2, -3, -4, -7, -8, -64, -1000, 0, 1, 2] {
        for radius in [1.0f32, 4.0, -2.0] {
            assert_same(&impls, size, radius);
        }
    }
}

#[test]
fn degenerate_radii() {
    let impls = Impls::load();
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -0.25,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30,
        1e-45, // subnormal
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
    ];
    for size in [0, 1, 2, 3, 4, 5, 8, 9, 15, 16, 31, 32] {
        for radius in radii {
            assert_same(&impls, size, radius);
        }
    }
}

#[test]
fn radius_sweep_bitwise() {
    let impls = Impls::load();
    // Sweep a wide exponent range to catch any last-bit divergence in the
    // `1/expf(x*x)` formulation and in the normalisation pass.
    let mut radius = 1e-8f32;
    while radius < 1e8 {
        for size in [7, 8, 21, 32] {
            assert_same(&impls, size, radius);
        }
        radius *= 1.7;
    }
}

#[test]
fn dense_small_radius_grid() {
    let impls = Impls::load();
    for i in 1..=400 {
        let radius = i as f32 * 0.037;
        assert_same(&impls, 17, radius);
        assert_same(&impls, 18, radius);
    }
}

#[test]
fn adjacent_float_radii() {
    let impls = Impls::load();
    // Neighbouring representable floats around interesting values.
    for base in [1.0f32, 1.6, 2.25, 3.0, 10.0] {
        let mut r = base;
        for _ in 0..16 {
            assert_same(&impls, 25, r);
            assert_same(&impls, 26, r);
            r = f32::from_bits(r.to_bits() + 1);
        }
        let mut r = base;
        for _ in 0..16 {
            assert_same(&impls, 25, r);
            r = f32::from_bits(r.to_bits() - 1);
        }
    }
}

#[test]
fn unaligned_offset_buffers() {
    // Ensures the pointer walk (`k++`) and the indexed normalisation agree
    // when the destination is not at the start of an allocation.
    let impls = Impls::load();
    for size in [5, 6, 15, 16] {
        for off in 0..4usize {
            let cap = size as usize + off + common::SLACK;
            let mut c_buf = vec![0.0f32; cap];
            let mut r_buf = vec![0.0f32; cap];
            for i in 0..cap {
                let v = f32::from_bits(0x4000_0000u32 ^ (i as u32));
                c_buf[i] = v;
                r_buf[i] = v;
            }
            unsafe {
                (impls.c)(c_buf.as_mut_ptr().add(off), size, 3.25);
                (impls.rust)(r_buf.as_mut_ptr().add(off), size, 3.25);
            }
            let cb: Vec<u32> = c_buf.iter().map(|v| v.to_bits()).collect();
            let rb: Vec<u32> = r_buf.iter().map(|v| v.to_bits()).collect();
            assert_eq!(cb, rb, "size={size} off={off}");
        }
    }
}

#[test]
fn randomized_fuzz() {
    let impls = Impls::load();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..20_000 {
        let size = (next() % 131) as i32 - 3; // -3..=127
        // Arbitrary bit patterns, including NaNs/infinities/subnormals.
        let radius = f32::from_bits(next() as u32);
        assert_same(&impls, size, radius);
    }
}

#[test]
fn randomized_fuzz_plausible_radii() {
    let impls = Impls::load();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..20_000 {
        let size = (next() % 96) as i32;
        // Positive radii spread over ~1e-6 .. ~1e6.
        let u = (next() >> 11) as f64 / (1u64 << 53) as f64;
        let radius = 10f64.powf(u * 12.0 - 6.0) as f32;
        assert_same(&impls, size, radius);
    }
}
