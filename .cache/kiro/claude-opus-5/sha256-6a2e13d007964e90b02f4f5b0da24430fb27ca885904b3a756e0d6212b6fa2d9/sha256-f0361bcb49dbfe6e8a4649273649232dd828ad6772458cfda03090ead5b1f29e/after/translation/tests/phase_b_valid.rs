//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH shared objects with
//! `libloading` and compares the post-call bytes produced by the C
//! `flip_horizontal` against the Rust `flip_horizontal`.

mod common;

use common::*;

const ITERS: usize = 32;

#[test]
fn phase_b_row01_empty_0x0() {
    let impls = load_impls();
    assert_same_randomized(&impls, 0, 0, ITERS, "CONFIGS row 1 (w=0,h=0)");
}

#[test]
fn phase_b_row02_w1_h0() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 0, ITERS, "CONFIGS row 2 (w=1,h=0)");
}

#[test]
fn phase_b_row03_w0_h1() {
    let impls = load_impls();
    assert_same_randomized(&impls, 0, 1, ITERS, "CONFIGS row 3 (w=0,h=1)");
}

#[test]
fn phase_b_row04_w0_many_rows() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..ITERS {
        let h = rng.in_range_i32(2, 64);
        assert_same_randomized(&impls, 0, h, 4, "CONFIGS row 4 (w=0, h in 2..64)");
    }
}

#[test]
fn phase_b_row05_single_pixel() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 1, ITERS, "CONFIGS row 5 (w=1,h=1)");
}

#[test]
fn phase_b_row06_minimum_swap_w1_h2() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 2, ITERS, "CONFIGS row 6 (w=1,h=2)");
}

#[test]
fn phase_b_row07_odd_h_w1_h3() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 3, ITERS, "CONFIGS row 7 (w=1,h=3)");
}

#[test]
fn phase_b_row08_w2_h2() {
    let impls = load_impls();
    assert_same_randomized(&impls, 2, 2, ITERS, "CONFIGS row 8 (w=2,h=2)");
}

#[test]
fn phase_b_row09_w2_h3() {
    let impls = load_impls();
    assert_same_randomized(&impls, 2, 3, ITERS, "CONFIGS row 9 (w=2,h=3)");
}

#[test]
fn phase_b_row10_w3_h4() {
    let impls = load_impls();
    assert_same_randomized(&impls, 3, 4, ITERS, "CONFIGS row 10 (w=3,h=4)");
}

#[test]
fn phase_b_row11_w5_h5() {
    let impls = load_impls();
    assert_same_randomized(&impls, 5, 5, ITERS, "CONFIGS row 11 (w=5,h=5)");
}

#[test]
fn phase_b_row12_random_small_cross_product() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..600 {
        let w = rng.in_range_i32(1, 17);
        let h = rng.in_range_i32(1, 17);
        let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
        assert_same(&impls, w, h, &buf, "CONFIGS row 12 (random 1..17 x 1..17)");
    }
}

#[test]
fn phase_b_row13_exhaustive_0_to_8() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x13);
    for w in 0..=8i32 {
        for h in 0..=8i32 {
            for _ in 0..8 {
                let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
                assert_same(&impls, w, h, &buf, "CONFIGS row 13 (exhaustive 0..8 x 0..8)");
            }
        }
    }
}

#[test]
fn phase_b_row14_large_64x64() {
    let impls = load_impls();
    assert_same_randomized(&impls, 64, 64, 16, "CONFIGS row 14 (w=64,h=64)");
}

#[test]
fn phase_b_row15_tall_thin_w1_h257() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 257, 8, "CONFIGS row 15 (w=1,h=257)");
}

#[test]
fn phase_b_row16_wide_flat_w257_h2() {
    let impls = load_impls();
    assert_same_randomized(&impls, 257, 2, 8, "CONFIGS row 16 (w=257,h=2)");
}

#[test]
fn phase_b_row17_odd_large_h1023() {
    let impls = load_impls();
    assert_same_randomized(&impls, 1, 1023, 4, "CONFIGS row 17 (w=1,h=1023)");
}

#[test]
fn phase_b_row18_all_zero_content() {
    let impls = load_impls();
    for (w, h) in [(1, 2), (3, 5), (8, 8), (7, 9)] {
        let buf = Buffer::filled(pixel_count(w, h), 0x00);
        assert_same(&impls, w, h, &buf, "CONFIGS row 18 (all-zero content)");
    }
}

#[test]
fn phase_b_row19_all_ff_content() {
    let impls = load_impls();
    for (w, h) in [(1, 2), (3, 5), (8, 8), (7, 9)] {
        let buf = Buffer::filled(pixel_count(w, h), 0xFF);
        assert_same(&impls, w, h, &buf, "CONFIGS row 19 (all-0xFF content)");
    }
}

/// Row 20: per-channel distinct patterns. If the translation mixed up channel
/// order, copied only part of the pixel, or swapped fields individually in the
/// wrong order, this diverges.
#[test]
fn phase_b_row20_per_channel_patterns() {
    let impls = load_impls();
    for (w, h) in [(1, 2), (2, 3), (4, 4), (5, 6), (9, 7)] {
        let pixels = pixel_count(w, h);
        let mut buf = Buffer::new(pixels);
        for p in 0..pixels {
            // Four independent, mutually distinguishable channel streams.
            buf.bytes[p * 4] = (p as u8).wrapping_mul(31).wrapping_add(1); // r
            buf.bytes[p * 4 + 1] = (p as u8).wrapping_mul(57).wrapping_add(2); // g
            buf.bytes[p * 4 + 2] = (p as u8).wrapping_mul(97).wrapping_add(3); // b
            buf.bytes[p * 4 + 3] = (p as u8).wrapping_mul(131).wrapping_add(4); // a
        }
        assert_same(&impls, w, h, &buf, "CONFIGS row 20 (per-channel patterns)");
    }
}

/// Row 21: the guard region check inside `assert_same` is the assertion; this
/// test makes it explicit for a spread of shapes, including shapes where the
/// last row is the one being swapped.
#[test]
fn phase_b_row21_no_write_past_end() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x21);
    for (w, h) in [(1, 2), (1, 3), (2, 2), (3, 3), (4, 6), (16, 16), (33, 5)] {
        let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
        let r = run_both(&impls, w, h, &buf);
        let payload = buf.payload_len;
        assert!(
            r.c_bytes[payload..].iter().all(|b| *b == GUARD_BYTE),
            "CONFIGS row 21: C wrote past w*h for w={w} h={h}"
        );
        assert!(
            r.rust_bytes[payload..].iter().all(|b| *b == GUARD_BYTE),
            "CONFIGS row 21: Rust wrote past w*h for w={w} h={h}"
        );
        assert_eq!(r.c_bytes, r.rust_bytes, "CONFIGS row 21: w={w} h={h}");
    }
}

/// Row 22: allocation exactly `w*h` pixels (guard immediately follows), so any
/// over-read that fed a write would show up as a differing byte.
#[test]
fn phase_b_row22_exact_allocation_no_slack() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x22);
    for w in 1..=6i32 {
        for h in 1..=6i32 {
            let buf = Buffer::randomized((w * h) as usize, &mut rng);
            assert_same(&impls, w, h, &buf, "CONFIGS row 22 (exact allocation)");
        }
    }
}

/// Row 23: `cp_image_t` must be left untouched (checked inside `assert_same`,
/// asserted explicitly here including the `pix` pointer field).
#[test]
fn phase_b_row23_struct_not_mutated() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x23);
    for (w, h) in [(0, 0), (1, 1), (3, 4), (8, 9), (-1, 5), (5, -1)] {
        let mut c_buf = Buffer::randomized(pixel_count(w, h), &mut rng);
        let mut rust_buf = c_buf.clone();
        let c_ptr = c_buf.bytes.as_mut_ptr();
        let rust_ptr = rust_buf.bytes.as_mut_ptr();
        let mut c_img = CpImage { w, h, pix: c_ptr };
        let mut rust_img = CpImage {
            w,
            h,
            pix: rust_ptr,
        };
        unsafe {
            (impls.c)(&mut c_img);
            (impls.rust)(&mut rust_img);
        }
        assert_eq!(c_img.w, w, "CONFIGS row 23: C changed w");
        assert_eq!(c_img.h, h, "CONFIGS row 23: C changed h");
        assert!(
            std::ptr::eq(c_img.pix, c_ptr),
            "CONFIGS row 23: C changed pix"
        );
        assert_eq!(rust_img.w, w, "CONFIGS row 23: Rust changed w");
        assert_eq!(rust_img.h, h, "CONFIGS row 23: Rust changed h");
        assert!(
            std::ptr::eq(rust_img.pix, rust_ptr),
            "CONFIGS row 23: Rust changed pix"
        );
        assert_eq!(c_buf.bytes, rust_buf.bytes, "CONFIGS row 23: w={w} h={h}");
    }
}

/// Row 24: applying the operation twice must restore the original buffer, and
/// must do so identically in both implementations at every intermediate step.
#[test]
fn phase_b_row24_double_application() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x24);
    for (w, h) in [(1, 2), (2, 3), (3, 4), (5, 5), (16, 17), (7, 8)] {
        let original = Buffer::randomized(pixel_count(w, h), &mut rng);

        let mut c_buf = original.clone();
        let mut rust_buf = original.clone();
        let mut c_img = CpImage {
            w,
            h,
            pix: c_buf.bytes.as_mut_ptr(),
        };
        let mut rust_img = CpImage {
            w,
            h,
            pix: rust_buf.bytes.as_mut_ptr(),
        };
        unsafe {
            (impls.c)(&mut c_img);
            (impls.rust)(&mut rust_img);
        }
        assert_eq!(
            c_buf.bytes, rust_buf.bytes,
            "CONFIGS row 24: first application diverges for w={w} h={h}"
        );
        unsafe {
            (impls.c)(&mut c_img);
            (impls.rust)(&mut rust_img);
        }
        assert_eq!(
            c_buf.bytes, rust_buf.bytes,
            "CONFIGS row 24: second application diverges for w={w} h={h}"
        );
        assert_eq!(
            c_buf.bytes, original.bytes,
            "CONFIGS row 24: C is not an involution for w={w} h={h} (ground truth changed)"
        );
    }
}

/// Row 25: negative `w`. These are ordinary `int` values the C API accepts.
#[test]
fn phase_b_row25_negative_w() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x25);
    for w in [-1i32, -5, -1000, i32::MIN] {
        for h in [0i32, 1, 2, 7] {
            for _ in 0..4 {
                let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
                assert_same(&impls, w, h, &buf, "CONFIGS row 25 (negative w)");
            }
        }
    }
}

/// Row 26: negative `h`. `h / 2` truncates toward zero in C, so `flips <= 0`
/// and nothing happens — a Rust translation that used unsigned math or
/// `h as usize / 2` would loop instead.
#[test]
fn phase_b_row26_negative_h() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x26);
    for h in [-1i32, -2, -7, -1023, i32::MIN] {
        for w in [0i32, 1, 4] {
            for _ in 0..4 {
                let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
                assert_same(&impls, w, h, &buf, "CONFIGS row 26 (negative h)");
            }
        }
    }
}

/// Row 27: `INT_MIN` / `INT_MAX` boundary combinations that provably perform no
/// memory access, so they can be compared safely and terminate immediately.
///
/// `h == INT_MAX` combined with a non-positive `w` is a ~1.07e9-iteration
/// no-op loop in both implementations; it is deliberately excluded here as a
/// runtime hazard rather than a behavioral difference (`flips` is positive and
/// the inner loop is empty, which rows 4 and 25 already cover for smaller `h`).
#[test]
fn phase_b_row27_int_boundaries() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x27);
    let ws = [i32::MIN, -1, 0, 1, i32::MAX];
    let hs = [i32::MIN, -1, 0, 1];
    for &w in &ws {
        for &h in &hs {
            let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
            assert_same(&impls, w, h, &buf, "CONFIGS row 27 (INT_MIN/INT_MAX bounds)");
        }
    }
}

/// Row 28: randomized fuzz sweep across the whole small signed range, with the
/// buffer always allocated for the largest shape so no case can fault.
#[test]
fn phase_b_row28_fuzz_sweep() {
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x28);
    for _ in 0..4000 {
        let w = rng.in_range_i32(-4, 24);
        let h = rng.in_range_i32(-4, 24);
        let pixels = pixel_count(w, h);
        let buf = Buffer::randomized(pixels, &mut rng);
        assert_same(&impls, w, h, &buf, "CONFIGS row 28 (fuzz sweep)");
    }
}

/// The project builds no binary/driver, so there is no stdout to compare.
/// Asserted structurally so the claim cannot silently go stale.
#[test]
fn phase_b_no_binary_target() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    assert!(
        !root.join("src/main.rs").exists(),
        "src/main.rs appeared: the crate now builds a binary and Phase B must \
         compare C vs Rust stdout"
    );
    assert!(
        !root.join("src/bin").exists(),
        "src/bin appeared: the crate now builds binaries and Phase B must \
         compare C vs Rust stdout"
    );
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !cargo_toml.contains("[[bin]]"),
        "Cargo.toml declares [[bin]]: Phase B must compare stdout"
    );
    let cmake = std::fs::read_to_string(root.join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src/CMakeLists.txt declares an executable: Phase B must compare stdout"
    );
}

/// Sanity: the two `.so` files under test really are two different files, and
/// the Rust one really does export the symbol via its `#[no_mangle]` wrapper
/// (i.e. the harness is not accidentally testing C against C).
#[test]
fn phase_b_harness_loads_two_distinct_libraries() {
    let c = c_so_path().canonicalize().unwrap();
    let r = rust_so_path().canonicalize().unwrap();
    assert_ne!(c, r, "harness loaded the same library twice");
    assert!(c.to_string_lossy().contains("c_src"), "unexpected C path: {c:?}");
    assert!(
        r.to_string_lossy().contains("flip_horizontal_lib"),
        "unexpected Rust path: {r:?}"
    );
    let _ = load_impls();
}
