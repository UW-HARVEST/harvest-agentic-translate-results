//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and compares the printed bytes exactly. Randomized rows use the
//! fixed seed in `harness::SEED` so failures reproduce.

mod harness;

use harness::*;

/// The `z` values used wherever a row calls for "a fixed set of z".
const Z_SET: [i32; 8] = [0, 1, -1, 2, -2, 12345, i32::MIN, i32::MAX];

/// How many randomized cases each property-style row uses.
const N_RANDOM: usize = 4000;

// ---------------------------------------------------------------------------
// Row 1 — all-zero input
// ---------------------------------------------------------------------------

#[test]
fn cfg01_driver_all_zero() {
    check_driver_row("cfg01 all-zero", &[(0, 0, 0, 0)]);
}

// ---------------------------------------------------------------------------
// Row 2 — exhaustive in-range cross-product of x, y, b against the z set
// ---------------------------------------------------------------------------

#[test]
fn cfg02_driver_exhaustive_in_range_cross_product() {
    let mut cases = Vec::new();
    for x in 0u32..=3 {
        for y in 0u32..=7 {
            for b in 0u32..=1 {
                for z in Z_SET {
                    cases.push((x, y, b, z));
                }
            }
        }
    }
    assert_eq!(cases.len(), 4 * 8 * 2 * Z_SET.len());
    check_driver_row("cfg02 exhaustive in-range", &cases);
}

// ---------------------------------------------------------------------------
// Row 3 — in-range x, y, b with z randomized across the whole i32 range
// ---------------------------------------------------------------------------

#[test]
fn cfg03_driver_in_range_random_z() {
    let mut rng = Rng::new(SEED ^ 3);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| {
            (
                rng.below(4),
                rng.below(8),
                rng.below(2),
                rng.interesting_i32(),
            )
        })
        .collect();
    check_driver_row("cfg03 in-range, random z", &cases);
}

// ---------------------------------------------------------------------------
// Rows 4/5/6 — one argument out of range at a time (silent truncation)
// ---------------------------------------------------------------------------

#[test]
fn cfg04_driver_x_out_of_range() {
    let mut rng = Rng::new(SEED ^ 4);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| {
            // x >= 4 only: anything at or past the `x : 2` maximum.
            let x = 4u32.wrapping_add(rng.next_u32() % (u32::MAX - 3));
            (x, rng.below(8), rng.below(2), rng.interesting_i32())
        })
        .collect();
    check_driver_row("cfg04 x out of range", &cases);
}

#[test]
fn cfg05_driver_y_out_of_range() {
    let mut rng = Rng::new(SEED ^ 5);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| {
            let y = 8u32.wrapping_add(rng.next_u32() % (u32::MAX - 7));
            (rng.below(4), y, rng.below(2), rng.interesting_i32())
        })
        .collect();
    check_driver_row("cfg05 y out of range", &cases);
}

#[test]
fn cfg06_driver_b_out_of_range() {
    let mut rng = Rng::new(SEED ^ 6);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| {
            let b = 2u32.wrapping_add(rng.next_u32() % (u32::MAX - 1));
            (rng.below(4), rng.below(8), b, rng.interesting_i32())
        })
        .collect();
    check_driver_row("cfg06 b out of range", &cases);
}

// ---------------------------------------------------------------------------
// Row 7 — everything randomized over the full 32-bit domains at once.
// This is the interaction row: x, y and b all truncate simultaneously.
// ---------------------------------------------------------------------------

#[test]
fn cfg07_driver_fully_random_all_args() {
    let mut rng = Rng::new(SEED ^ 7);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| {
            (
                rng.interesting_u32(),
                rng.interesting_u32(),
                rng.interesting_u32(),
                rng.interesting_i32(),
            )
        })
        .collect();
    check_driver_row("cfg07 fully random", &cases);
}

// ---------------------------------------------------------------------------
// Row 8 — boundary values only, full cross-product
// ---------------------------------------------------------------------------

#[test]
fn cfg08_driver_boundary_cross_product() {
    // 0, the largest in-range value, one past it, and the type maximum.
    const XS: [u32; 5] = [0, 3, 4, 5, u32::MAX];
    const YS: [u32; 5] = [0, 7, 8, 9, u32::MAX];
    const BS: [u32; 6] = [0, 1, 2, 3, 255, u32::MAX];
    const ZS: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];
    let mut cases = Vec::new();
    for x in XS {
        for y in YS {
            for b in BS {
                for z in ZS {
                    cases.push((x, y, b, z));
                }
            }
        }
    }
    assert_eq!(cases.len(), 5 * 5 * 6 * 5);
    check_driver_row("cfg08 boundary cross-product", &cases);
}

// ---------------------------------------------------------------------------
// Row 9 — z sign/magnitude sweep (every power of two and its negation)
// ---------------------------------------------------------------------------

#[test]
fn cfg09_driver_z_power_of_two_sweep() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut cases = Vec::new();
    for bit in 0..32u32 {
        let v = 1i32.wrapping_shl(bit);
        for z in [v, v.wrapping_neg(), v.wrapping_sub(1), v.wrapping_add(1)] {
            cases.push((rng.interesting_u32(), rng.interesting_u32(), rng.interesting_u32(), z));
        }
    }
    check_driver_row("cfg09 z power-of-two sweep", &cases);
}

// ---------------------------------------------------------------------------
// Row 10 — print_foo over EVERY storage byte (all x/y/b combinations plus both
// padding-bit states) crossed with several z values.
// ---------------------------------------------------------------------------

#[test]
fn cfg10_print_foo_all_storage_bytes() {
    let mut cases = Vec::new();
    for storage in 0u16..=255 {
        for z in [0i32, -1, 1, i32::MIN, i32::MAX] {
            cases.push(FooImage::new(storage as u8, z));
        }
    }
    assert_eq!(cases.len(), 256 * 5);
    check_print_foo_row("cfg10 all storage bytes", &cases);
}

// ---------------------------------------------------------------------------
// Row 11 — print_foo with fully randomized raw 8-byte images
// ---------------------------------------------------------------------------

#[test]
fn cfg11_print_foo_random_images() {
    let mut rng = Rng::new(SEED ^ 11);
    let cases: Vec<_> = (0..N_RANDOM)
        .map(|_| FooImage::new(rng.next_u32() as u8, rng.interesting_i32()))
        .collect();
    check_print_foo_row("cfg11 random images", &cases);
}

// ---------------------------------------------------------------------------
// Row 12 — print_foo through deliberately misaligned pointers
// ---------------------------------------------------------------------------

#[test]
fn cfg12_print_foo_misaligned_pointer() {
    let mut rng = Rng::new(SEED ^ 12);
    // Cases are (offset, storage, z); the image is copied into a 16-byte
    // aligned buffer at `offset`, so offsets 1..3 break foo_t's 4-byte
    // alignment requirement.
    let cases: Vec<(usize, u8, i32)> = (0..512)
        .map(|i| {
            (
                i % 4,
                rng.next_u32() as u8,
                rng.interesting_i32(),
            )
        })
        .collect();

    let run = |f: &PrintFooFn| -> Vec<u8> {
        capture(|| {
            #[repr(C, align(4))]
            struct Buf([u8; 16]);
            for &(off, storage, z) in &cases {
                let mut buf = Buf([0u8; 16]);
                let img = FooImage::new(storage, z);
                buf.0[off..off + 8].copy_from_slice(&img.0);
                unsafe { f(buf.0.as_ptr().add(off)) };
            }
        })
    };

    let c = run(&*c_print_foo());
    let r = run(&*rs_print_foo());
    assert_same("cfg12 misaligned pointer", &cases, &c, &r);
}

// ---------------------------------------------------------------------------
// Row 13 — z extremes crossed with padding bits set and clear
// ---------------------------------------------------------------------------

#[test]
fn cfg13_print_foo_z_extremes_and_padding() {
    let mut cases = Vec::new();
    for pad in [0x00u8, 0x40, 0x80, 0xC0] {
        for x in 0u32..=3 {
            for y in 0u32..=7 {
                for b in 0u32..=1 {
                    for z in [i32::MIN, -1, 0, i32::MAX] {
                        cases.push(FooImage::new(pack_storage(x, y, b, pad), z));
                    }
                }
            }
        }
    }
    check_print_foo_row("cfg13 z extremes x padding", &cases);
}

// ---------------------------------------------------------------------------
// Row 14 — cross-library round trip. This pins the *private* foo_t layout
// across the ABI: C's bit-field packer must feed Rust's unpacker and vice
// versa, producing identical text.
// ---------------------------------------------------------------------------

#[test]
fn cfg14_cross_library_driver_to_print_foo_roundtrip() {
    let mut rng = Rng::new(SEED ^ 14);
    let cases: Vec<(u32, u32, u32, i32)> = (0..N_RANDOM)
        .map(|_| {
            (
                rng.interesting_u32(),
                rng.interesting_u32(),
                rng.interesting_u32(),
                rng.interesting_i32(),
            )
        })
        .collect();

    // The images an external caller would hand to print_foo for these inputs.
    let images: Vec<FooImage> = cases
        .iter()
        .map(|&(x, y, b, z)| FooImage::new(pack_storage(x, y, b, 0), z))
        .collect();

    let c_driver_out = run_driver_batch(&*c_driver(), &cases);
    let rs_driver_out = run_driver_batch(&*rs_driver(), &cases);
    let c_pf_out = run_print_foo_batch(&*c_print_foo(), &images);
    let rs_pf_out = run_print_foo_batch(&*rs_print_foo(), &images);

    // C's packer vs Rust's unpacker, and Rust's packer vs C's unpacker.
    assert_same("cfg14 C driver vs Rust print_foo", &cases, &c_driver_out, &rs_pf_out);
    assert_same("cfg14 Rust driver vs C print_foo", &cases, &rs_driver_out, &c_pf_out);
    // ... and the direct pairings, for completeness.
    assert_same("cfg14 driver C vs Rust", &cases, &c_driver_out, &rs_driver_out);
    assert_same("cfg14 print_foo C vs Rust", &images, &c_pf_out, &rs_pf_out);
}

// ---------------------------------------------------------------------------
// Row 15 — long interleaved sequence of both entry points in one process,
// checking that no state leaks between calls and that the multi-line stdout
// byte stream matches.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Op {
    Driver(u32, u32, u32, i32),
    PrintFoo(FooImage),
}

#[test]
fn cfg15_interleaved_entry_points() {
    let mut rng = Rng::new(SEED ^ 15);
    let ops: Vec<Op> = (0..4000)
        .map(|_| {
            if rng.next_u32() & 1 == 0 {
                Op::Driver(
                    rng.interesting_u32(),
                    rng.interesting_u32(),
                    rng.interesting_u32(),
                    rng.interesting_i32(),
                )
            } else {
                Op::PrintFoo(FooImage::new(rng.next_u32() as u8, rng.interesting_i32()))
            }
        })
        .collect();

    let run = |d: &DriverFn, p: &PrintFooFn| -> Vec<u8> {
        capture(|| {
            for op in &ops {
                match *op {
                    Op::Driver(x, y, b, z) => unsafe { d(x, y, b, z) },
                    Op::PrintFoo(img) => unsafe { p(img.as_ptr()) },
                }
            }
        })
    };

    let c = run(&*c_driver(), &*c_print_foo());
    let r = run(&*rs_driver(), &*rs_print_foo());
    assert_same("cfg15 interleaved", &ops, &c, &r);
}

// ---------------------------------------------------------------------------
// Sanity: confirm the two libraries actually loaded are different files and
// that both export the two symbols (guards against a test that silently
// compares a library with itself).
// ---------------------------------------------------------------------------

#[test]
fn cfg00_harness_sanity() {
    let l = libs();
    assert_ne!(l.c_path, l.rs_path, "must load two distinct .so files");
    assert!(
        l.c_path.to_string_lossy().contains("c_src"),
        "C library should come from c_src, got {:?}",
        l.c_path
    );
    // Both symbol pairs resolve.
    let _ = c_driver();
    let _ = rs_driver();
    let _ = c_print_foo();
    let _ = rs_print_foo();

    // The capture mechanism really does capture printf output.
    let out = run_driver_batch(&*c_driver(), &[(1, 2, 1, -5)]);
    assert_eq!(out, b"1 2 1 -5\n", "capture harness is broken: {out:?}");
}
