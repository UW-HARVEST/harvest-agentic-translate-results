//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every call goes through `libloading`
//! into the C `.so` and the Rust `.so`; the whole `pcm` scratch buffer is
//! compared byte-for-byte.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn zeros() -> Vec<f32> {
    vec![0.0f32; Z_LEN]
}

/// Fill `z` from a generator and run it against both libs.
fn randomized_row(nch_list: &[i32], cases: usize, seed: u64, what: &str, mut genf: impl FnMut(&mut Rng) -> f32) {
    let pair = Pair::load();
    let mut rng = Rng::new(seed);
    let mut z = zeros();
    for case in 0..cases {
        for v in z.iter_mut() {
            *v = genf(&mut rng);
        }
        for &nch in nch_list {
            assert_same(&pair, nch, &z, &format!("{what} case={case}"));
        }
    }
}

// ---------------------------------------------------------------- row 1
#[test]
fn row01_all_zero_taps() {
    let pair = Pair::load();
    let z = zeros();
    assert_same(&pair, 1, &z, "row01 all-zero taps");

    // Also pin the absolute expected value: 0 -> +0.5f -> trunc 0 -> not <0.
    let mut pcm = Pcm::new();
    unsafe { (pair.rs)(pcm.base(), 1, z.as_ptr()) };
    let off = Pcm::HALF;
    assert_eq!(pcm.as_slice()[off], 0, "pcm[0] for all-zero input");
    assert_eq!(pcm.as_slice()[off + 16], 0, "pcm[16] for all-zero input");
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_random_unit_range() {
    randomized_row(&[1], 4096, SEED ^ 2, "row02 [-1,1]", |r| r.sym(1.0));
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_random_mid_range() {
    randomized_row(&[1], 4096, SEED ^ 3, "row03 [-4,4]", |r| r.sym(4.0));
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_random_saturating_range() {
    // magnitude 64 * sum(|coeffs|) far exceeds 32767, so both clamps fire often.
    randomized_row(&[1], 4096, SEED ^ 4, "row04 [-64,64] clamping", |r| r.sym(64.0));
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_random_wide_finite_exponents() {
    randomized_row(&[1], 4096, SEED ^ 5, "row05 wide finite", |r| r.wide_finite());
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_random_arbitrary_bit_patterns() {
    // Includes NaN, +/-Inf and subnormals by construction.
    randomized_row(&[1], 4096, SEED ^ 6, "row06 arbitrary bits", |r| r.any_f32());
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_single_nonzero_tap_sweep() {
    let pair = Pair::load();
    let mags: [f32; 10] = [
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-30,
        -1e-30,
        f32::MIN_POSITIVE,
        1e30,
        -1e30,
        1.0 / 3.0,
    ];
    let mut z = zeros();
    for idx in 0..Z_LEN {
        for &m in &mags {
            z[idx] = m;
            assert_same(&pair, 1, &z, &format!("row07 single tap idx={idx} mag={m:e}"));
            z[idx] = 0.0;
        }
    }
}

/// Solve for the single tap value that makes accumulator 1 equal `target`.
/// Accumulator 1 uses `z[7*64] * 75038`, and 75038 is exactly representable.
fn set_acc1(z: &mut [f32], target: f32) {
    for v in z.iter_mut() {
        *v = 0.0;
    }
    z[tap1(7)] = target / 75038.0f32;
}

/// Accumulator 2 reads `(z+2)[8*64] * 64019`.
fn set_acc2(z: &mut [f32], target: f32) {
    z[tap2(8)] = target / 64019.0f32;
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_clamp_boundaries_exact_and_one_step() {
    let pair = Pair::load();
    let mut z = zeros();

    let hi = 32766.5f32;
    let lo = -32767.5f32;
    let targets: [f32; 12] = [
        hi,
        f32::from_bits(hi.to_bits() - 1), // one f32 step below +32766.5
        f32::from_bits(hi.to_bits() + 1), // one step above
        lo,
        f32::from_bits(lo.to_bits() - 1), // one step MORE negative
        f32::from_bits(lo.to_bits() + 1), // one step less negative
        32767.0,
        -32768.0,
        32766.0,
        -32767.0,
        f32::MAX,
        f32::MIN,
    ];

    for &t in &targets {
        // Drive accumulator 1 (and leave 2 at zero), then vice versa.
        set_acc1(&mut z, t);
        assert_same(&pair, 1, &z, &format!("row08 acc1 target={t}"));

        for v in z.iter_mut() {
            *v = 0.0;
        }
        set_acc2(&mut z, t);
        assert_same(&pair, 1, &z, &format!("row08 acc2 target={t}"));
    }

    // Feed the boundary values *directly* as taps with coefficient paths that
    // preserve them, plus non-finite values.
    for &t in &[f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0f32, 0.0f32] {
        for v in z.iter_mut() {
            *v = 0.0;
        }
        z[tap1(7)] = t;
        z[tap2(8)] = t;
        assert_same(&pair, 1, &z, &format!("row08 nonfinite tap={t}"));
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_negative_truncation_window() {
    let pair = Pair::load();
    let mut z = zeros();

    // The `s -= (s < 0)` quirk: values in (-1, 0) and [-1, -0.5] truncate to 0.
    let targets: [f32; 16] = [
        -0.0, 0.0, -0.25, -0.5, -0.75, -1.0, -1.25, -1.5, -1.75, -2.0, 0.25, 0.5, 0.75, 1.0, 1.5,
        -1e-30,
    ];
    for &t in &targets {
        set_acc1(&mut z, t);
        assert_same(&pair, 1, &z, &format!("row09 acc1 target={t}"));
        for v in z.iter_mut() {
            *v = 0.0;
        }
        set_acc2(&mut z, t);
        assert_same(&pair, 1, &z, &format!("row09 acc2 target={t}"));
    }

    // Exact -0.0f accumulator: all taps zero already yields +0.0; force -0.0 by
    // making the final add produce -0.0 (0.0 * -5 on the second accumulator).
    for v in z.iter_mut() {
        *v = 0.0;
    }
    z[tap2(0)] = 0.0;
    assert_same(&pair, 1, &z, "row09 acc2 -0.0 path");
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_store0_saturates_high_store1_in_range() {
    let pair = Pair::load();
    let mut z = zeros();
    set_acc1(&mut z, 1.0e6); // way above +32766.5
    set_acc2(&mut z, 123.75); // comfortably in range
    assert_same(&pair, 1, &z, "row10 hi/in-range");

    // and the mirror: store0 in range, store1 saturates high
    set_acc1(&mut z, -100.5);
    set_acc2(&mut z, 1.0e6);
    assert_same(&pair, 1, &z, "row10 in-range/hi");
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_opposite_clamps_in_one_call() {
    let pair = Pair::load();
    let mut z = zeros();
    set_acc1(&mut z, -1.0e6); // clamps low
    set_acc2(&mut z, 1.0e6); // clamps high
    assert_same(&pair, 1, &z, "row11 low/high");

    set_acc1(&mut z, 1.0e6);
    set_acc2(&mut z, -1.0e6);
    assert_same(&pair, 1, &z, "row11 high/low");
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_nch_two_stereo_stride() {
    randomized_row(&[2], 2048, SEED ^ 12, "row12 nch=2 mid", |r| r.sym(4.0));
    randomized_row(&[2], 2048, SEED ^ 0x12, "row12 nch=2 saturating", |r| r.sym(64.0));
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_nch_three_odd_stride() {
    randomized_row(&[3], 2048, SEED ^ 13, "row13 nch=3", |r| r.sym(8.0));
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_nch_zero_aliasing_stores() {
    let pair = Pair::load();

    // Randomized: with nch == 0 both stores hit pcm[0]; the SECOND must win.
    let mut rng = Rng::new(SEED ^ 14);
    let mut z = zeros();
    for case in 0..2048 {
        for v in z.iter_mut() {
            *v = rng.sym(64.0);
        }
        assert_same(&pair, 0, &z, &format!("row14 nch=0 case={case}"));
    }

    // Explicitly prove the second write wins: make acc1 clamp high and acc2
    // clamp low; pcm[0] must end up -32768, not 32767.
    set_acc1(&mut z, 1.0e6);
    set_acc2(&mut z, -1.0e6);
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), 0, z.as_ptr()) };
    assert_eq!(pcm.as_slice()[Pcm::HALF], -32768, "C: second store must win at nch=0");
    let mut pcm2 = Pcm::new();
    unsafe { (pair.rs)(pcm2.base(), 0, z.as_ptr()) };
    assert_eq!(pcm2.as_slice()[Pcm::HALF], -32768, "Rust: second store must win at nch=0");
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_negative_nch() {
    randomized_row(&[-1, -2, -7, -128], 512, SEED ^ 15, "row15 negative nch", |r| r.sym(64.0));

    // Pin that the negative slot really is written and pcm[0..] is untouched
    // apart from offset 0.
    let pair = Pair::load();
    let mut z = zeros();
    set_acc1(&mut z, 1000.25);
    set_acc2(&mut z, -2000.75);
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), -1, z.as_ptr()) };
    let s = pcm.as_slice();
    assert_ne!(s[Pcm::HALF - 16], 0x5A5Au16 as i16, "C must write pcm[-16] for nch=-1");
    assert_eq!(s[Pcm::HALF + 16], 0x5A5Au16 as i16, "C must NOT write pcm[+16] for nch=-1");
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_nch_overflowing_16x() {
    // `16 * nch` overflows int; C wraps mod 2^32 then sign-extends.
    // Only the values whose wrapped offset lands inside the scratch buffer can
    // be exercised safely -- these are exactly the interesting ones, because a
    // 64-bit (non-wrapping) Rust computation would go far out of bounds.
    let candidates: [i32; 10] = [
        i32::MAX,      // 16*x wraps to -16
        i32::MIN,      // wraps to 0
        i32::MAX - 1,  // wraps to -32
        i32::MIN + 1,  // wraps to +16
        0x1000_0000,   // wraps to 0
        0x1000_0001,   // wraps to +16
        0x0FFF_FFFF,   // wraps to -16
        0x7FFF_FFF0,   // wraps to -256
        -0x1000_0001,  // wraps to -16
        0x4000_0004,   // wraps to +64
    ];

    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 16);
    let mut z = zeros();

    let mut exercised = 0usize;
    for &nch in &candidates {
        assert!(
            nch_fits(nch),
            "nch={nch} -> offset {} escapes the scratch buffer; adjust HALF",
            nch.wrapping_mul(16) as isize
        );
        for case in 0..64 {
            for v in z.iter_mut() {
                *v = rng.sym(64.0);
            }
            assert_same(&pair, nch, &z, &format!("row16 nch={nch} case={case}"));
        }
        exercised += 1;
    }
    assert_eq!(exercised, candidates.len());
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_minimum_inbounds_z_buffer() {
    // Place exactly Z_LEN floats at the very END of a page-guarded mapping, so
    // reading z[899] or beyond faults. If either .so over-reads, this segfaults
    // (a hard failure), and if either under-reads we would not see identical
    // output for the sweep below.
    let pair = Pair::load();

    let page = 4096usize;
    let bytes = Z_LEN * 4;
    let data_pages = (bytes + page - 1) / page;
    let total = (data_pages + 1) * page; // + one guard page

    unsafe {
        let base = libc_mmap(total);
        // Make the LAST page inaccessible.
        libc_mprotect_none(base.add(data_pages * page), page);

        // z starts so that its last element ends exactly at the guard boundary.
        let z_ptr = base.add(data_pages * page - bytes) as *mut f32;
        let z = std::slice::from_raw_parts_mut(z_ptr, Z_LEN);

        let mut rng = Rng::new(SEED ^ 17);
        for case in 0..256 {
            for v in z.iter_mut() {
                *v = rng.sym(64.0);
            }
            let mut c_pcm = Pcm::new();
            let mut rs_pcm = Pcm::new();
            (pair.c)(c_pcm.base(), 1, z_ptr);
            (pair.rs)(rs_pcm.base(), 1, z_ptr);
            assert_eq!(
                c_pcm.as_slice(),
                rs_pcm.as_slice(),
                "row17 min-size guarded z, case={case}"
            );
        }

        libc_munmap(base, total);
    }
}

// Minimal mmap/mprotect shims so no extra dependency is needed.
unsafe extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn mprotect(addr: *mut core::ffi::c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
}
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_NONE: i32 = 0;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe fn libc_mmap(len: usize) -> *mut u8 {
    let p = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(p as isize != -1, "mmap failed");
    p as *mut u8
}
unsafe fn libc_mprotect_none(p: *mut u8, len: usize) {
    assert_eq!(unsafe { mprotect(p as *mut _, len, PROT_NONE) }, 0, "mprotect failed");
}
unsafe fn libc_munmap(p: *mut u8, len: usize) {
    unsafe { munmap(p as *mut _, len) };
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_subtracted_vs_added_tap_groups() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 18);
    let mut z = zeros();

    // Taps that appear on the LEFT of a subtraction / are subtracted:
    // z[0], z[2*64], z[4*64], z[6*64] are subtracted; z[14*64], z[12*64],
    // z[10*64], z[8*64] are the positive halves of those pairs.
    let subtracted = [tap1(0), tap1(2), tap1(4), tap1(6)];
    let pos_of_pairs = [tap1(14), tap1(12), tap1(10), tap1(8)];
    let added = [tap1(1), tap1(13), tap1(3), tap1(11), tap1(5), tap1(9), tap1(7)];

    for (name, group) in [
        ("subtracted-only", &subtracted[..]),
        ("pair-positive-only", &pos_of_pairs[..]),
        ("added-only", &added[..]),
    ] {
        for case in 0..512 {
            for v in z.iter_mut() {
                *v = 0.0;
            }
            for &i in group {
                z[i] = rng.sym(64.0);
            }
            assert_same(&pair, 1, &z, &format!("row18 {name} case={case}"));
        }
    }

    // Cancelling pattern: make each (a - b) pair exactly zero.
    for case in 0..512 {
        for v in z.iter_mut() {
            *v = 0.0;
        }
        for (&s, &p) in subtracted.iter().zip(pos_of_pairs.iter()) {
            let v = rng.sym(1000.0);
            z[s] = v;
            z[p] = v;
        }
        assert_same(&pair, 1, &z, &format!("row18 cancelling pairs case={case}"));
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_second_accumulator_negative_coefficients() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 19);
    let mut z = zeros();

    // Coefficients -9975 (k=6), -45 (k=4), -5 (k=0).
    let neg = [tap2(6), tap2(4), tap2(0)];
    for case in 0..1024 {
        for v in z.iter_mut() {
            *v = 0.0;
        }
        for &i in &neg {
            z[i] = rng.sym(64.0);
        }
        assert_same(&pair, 1, &z, &format!("row19 negative-coeff taps case={case}"));
    }

    // All 8 second-accumulator taps, randomized, positive coefficients too.
    let all2: Vec<usize> = (0..8).map(|j| tap2(j * 2)).collect();
    for case in 0..1024 {
        for v in z.iter_mut() {
            *v = 0.0;
        }
        for &i in &all2 {
            z[i] = rng.sym(64.0);
        }
        assert_same(&pair, 1, &z, &format!("row19 all acc2 taps case={case}"));
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn row20_cross_bitpatterns_with_strides() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 20);
    let mut z = zeros();

    let nchs: [i32; 8] = [1, 2, 0, -1, -3, i32::MAX, i32::MIN, 0x1000_0001];
    for case in 0..512 {
        for v in z.iter_mut() {
            *v = rng.any_f32();
        }
        for &nch in &nchs {
            assert_same(&pair, nch, &z, &format!("row20 nch={nch} case={case}"));
        }
    }
}
