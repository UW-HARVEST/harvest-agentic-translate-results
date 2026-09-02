//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH `libdriver.so`s via
//! `libloading` and compares the bytes each writes to `stdout`.
//!
//! Randomized rows use a fixed PRNG seed so failures reproduce exactly.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn libs() -> Libs {
    Libs::load()
}

// --- Row 1: exhaustive in-range x * y * b, z = 0 ---------------------------

#[test]
fn row01_driver_inrange_exhaustive_z_zero() {
    let l = libs();
    let mut ops = Vec::new();
    for x in 0u32..=3 {
        for y in 0u32..=7 {
            for b in 0u8..=1 {
                ops.push(Op::Driver { x, y, b, z: 0 });
            }
        }
    }
    assert_eq!(ops.len(), 64);
    assert_batch_eq(&l, &ops, "row01");
}

// --- Row 2: in-range x/y/b * randomized z ---------------------------------

#[test]
fn row02_driver_inrange_random_z() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    let mut ops = Vec::new();
    for _ in 0..64 {
        for x in 0u32..=3 {
            for y in 0u32..=7 {
                for b in 0u8..=1 {
                    ops.push(Op::Driver {
                        x,
                        y,
                        b,
                        z: rng.next_i32(),
                    });
                }
            }
        }
    }
    assert_eq!(ops.len(), 4096);
    assert_batch_eq_chunked(&l, &ops, "row02");
}

// --- Row 3: x out of range ------------------------------------------------

#[test]
fn row03_driver_x_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 3);
    let mut ops = Vec::new();
    // Every small out-of-range value, so each residue class mod 4 is hit.
    for x in 4u32..=64 {
        ops.push(Op::Driver {
            x,
            y: rng.below(8),
            b: (rng.next_u8() & 1),
            z: rng.next_i32(),
        });
    }
    // Plus wide randoms from the whole out-of-range space.
    for _ in 0..4000 {
        let x = 4u32.wrapping_add(rng.next_u32() % (u32::MAX - 3));
        ops.push(Op::Driver {
            x,
            y: rng.below(8),
            b: (rng.next_u8() & 1),
            z: rng.next_i32(),
        });
    }
    // And the extremes.
    for x in [4u32, 5, 6, 7, u32::MAX - 1, u32::MAX, 0x8000_0000, 1 << 31] {
        ops.push(Op::Driver { x, y: 5, b: 1, z: -7 });
    }
    assert_batch_eq_chunked(&l, &ops, "row03");
}

// --- Row 4: y out of range ------------------------------------------------

#[test]
fn row04_driver_y_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 4);
    let mut ops = Vec::new();
    for y in 8u32..=128 {
        ops.push(Op::Driver {
            x: rng.below(4),
            y,
            b: (rng.next_u8() & 1),
            z: rng.next_i32(),
        });
    }
    for _ in 0..4000 {
        let y = 8u32.wrapping_add(rng.next_u32() % (u32::MAX - 7));
        ops.push(Op::Driver {
            x: rng.below(4),
            y,
            b: (rng.next_u8() & 1),
            z: rng.next_i32(),
        });
    }
    for y in [8u32, 9, 15, 16, u32::MAX - 1, u32::MAX, 0x8000_0000] {
        ops.push(Op::Driver { x: 2, y, b: 0, z: 7 });
    }
    assert_batch_eq_chunked(&l, &ops, "row04");
}

// --- Row 5: non-canonical bool bytes 2..=255 ------------------------------

#[test]
fn row05_driver_noncanonical_bool_exhaustive() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 5);
    let mut ops = Vec::new();
    for b in 0u8..=255 {
        // Repeat each byte a few times with different surrounding values so a
        // value-dependent bug cannot hide behind one x/y/z combination.
        for _ in 0..8 {
            ops.push(Op::Driver {
                x: rng.next_u32(),
                y: rng.next_u32(),
                b,
                z: rng.next_i32(),
            });
        }
    }
    assert_eq!(ops.len(), 256 * 8);
    assert_batch_eq_chunked(&l, &ops, "row05");
}

// --- Row 6: z boundaries --------------------------------------------------

#[test]
fn row06_driver_z_boundaries() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    let zs = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        -2,
        i16::MIN as i32,
        i16::MAX as i32,
        u16::MAX as i32,
    ];
    let mut ops = Vec::new();
    for &z in &zs {
        for _ in 0..64 {
            ops.push(Op::Driver {
                x: rng.next_u32(),
                y: rng.next_u32(),
                b: rng.next_u8(),
                z,
            });
        }
    }
    assert_batch_eq_chunked(&l, &ops, "row06");
}

// --- Row 7: fully randomized interaction row ------------------------------

#[test]
fn row07_driver_fully_random() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 7);
    let mut ops = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        ops.push(Op::Driver {
            x: rng.next_u32(),
            y: rng.next_u32(),
            b: rng.next_u8(),
            z: rng.next_i32(),
        });
    }
    assert_batch_eq_chunked(&l, &ops, "row07");
}

// --- Row 8: boundary cross-product ---------------------------------------

#[test]
fn row08_driver_boundary_cross_product() {
    let l = libs();
    let mut ops = Vec::new();
    for &x in &[0u32, 3, 4, u32::MAX] {
        for &y in &[0u32, 7, 8, u32::MAX] {
            for &b in &[0u8, 1, 2, 0xFF] {
                for &z in &[i32::MIN, -1, 0, i32::MAX] {
                    ops.push(Op::Driver { x, y, b, z });
                }
            }
        }
    }
    assert_eq!(ops.len(), 256);
    assert_batch_eq(&l, &ops, "row08");
}

// --- Row 9: print_foo, exhaustive byte 0, zero padding --------------------

#[test]
fn row09_print_foo_byte0_exhaustive_zero_padding() {
    let l = libs();
    let mut ops = Vec::new();
    for byte0 in 0u16..=255 {
        ops.push(Op::PrintFoo {
            raw: foo_bytes(byte0 as u8, [0, 0, 0], 0),
        });
    }
    assert_eq!(ops.len(), 256);
    assert_batch_eq(&l, &ops, "row09");
}

// --- Row 10: print_foo, exhaustive byte 0, 0xFF padding -------------------

#[test]
fn row10_print_foo_byte0_exhaustive_ff_padding() {
    let l = libs();
    let mut ops = Vec::new();
    for byte0 in 0u16..=255 {
        ops.push(Op::PrintFoo {
            raw: foo_bytes(byte0 as u8, [0xFF, 0xFF, 0xFF], 0),
        });
    }
    assert_batch_eq(&l, &ops, "row10");

    // Same byte-0 decode regardless of padding: the two padding variants must
    // produce identical output within each library too.
    let cf = l.c_print_foo();
    for byte0 in 0u16..=255 {
        let zeroed = foo_bytes(byte0 as u8, [0, 0, 0], 0);
        let garbage = foo_bytes(byte0 as u8, [0xFF, 0xFF, 0xFF], 0);
        let a = capture_stdout(|| unsafe { cf(zeroed.as_ptr()) });
        let b = capture_stdout(|| unsafe { cf(garbage.as_ptr()) });
        assert_eq!(
            a, b,
            "C print_foo output depends on padding for byte0={byte0:#02x}"
        );
    }
}

// --- Row 11: print_foo fully randomized ----------------------------------

#[test]
fn row11_print_foo_fully_random() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 11);
    let mut ops = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        let mut raw = [0u8; FOO_SIZE];
        rng.fill(&mut raw);
        ops.push(Op::PrintFoo { raw });
    }
    assert_batch_eq_chunked(&l, &ops, "row11");
}

// --- Row 12: print_foo z boundaries --------------------------------------

#[test]
fn row12_print_foo_z_boundaries() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    let mut ops = Vec::new();
    for &z in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for &byte0 in &[0x00u8, 0x3F, 0xFF, 0x20, 0x1F, 0xC0] {
            let mut pad = [0u8; 3];
            rng.fill(&mut pad);
            ops.push(Op::PrintFoo {
                raw: foo_bytes(byte0, pad, z),
            });
        }
    }
    assert_batch_eq(&l, &ops, "row12");
}

// --- Row 13: print_foo with a misaligned pointer -------------------------

#[test]
fn row13_print_foo_misaligned_pointer() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    let mut ops = Vec::new();
    for _ in 0..2000 {
        let mut raw = [0u8; FOO_SIZE];
        rng.fill(&mut raw);
        ops.push(Op::PrintFooMisaligned { raw });
    }
    for &z in &[i32::MIN, -1, 0, i32::MAX] {
        for byte0 in [0x00u8, 0xFF, 0x2A] {
            ops.push(Op::PrintFooMisaligned {
                raw: foo_bytes(byte0, [0xAA, 0xBB, 0xCC], z),
            });
        }
    }
    assert_batch_eq_chunked(&l, &ops, "row13");
}

// --- Row 14: driver / print_foo pipeline equivalence ---------------------

#[test]
fn row14_pipeline_equivalence_driver_vs_print_foo() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);

    let c_driver = l.c_driver();
    let c_print = l.c_print_foo();
    let r_driver = l.rust_driver();
    let r_print = l.rust_print_foo();

    for _ in 0..8192 {
        let x = rng.next_u32();
        let y = rng.next_u32();
        let b = rng.next_u8();
        let z = rng.next_i32();
        // Padding is deliberately garbage: `driver` leaves it uninitialised, so
        // an equivalent `print_foo` call must ignore it.
        let mut pad = [0u8; 3];
        rng.fill(&mut pad);
        let raw = foo_bytes(pack_byte0(x, y, b), pad, z);

        let c_via_driver = capture_stdout(|| unsafe { c_driver(x, y, b, z) });
        let c_via_print = capture_stdout(|| unsafe { c_print(raw.as_ptr()) });
        let r_via_driver = capture_stdout(|| unsafe { r_driver(x, y, b, z) });
        let r_via_print = capture_stdout(|| unsafe { r_print(raw.as_ptr()) });

        assert_eq!(
            c_via_driver, c_via_print,
            "C: driver({x},{y},{b},{z}) != print_foo({raw:02x?})"
        );
        assert_eq!(
            c_via_driver, r_via_driver,
            "driver({x},{y},{b},{z}) C vs Rust"
        );
        assert_eq!(
            c_via_print, r_via_print,
            "print_foo({raw:02x?}) C vs Rust"
        );
        assert_eq!(
            r_via_driver, r_via_print,
            "Rust: driver({x},{y},{b},{z}) != print_foo({raw:02x?})"
        );
    }
}

// --- Row 15: repeated calls in one buffered session ----------------------

#[test]
fn row15_repeated_calls_one_session() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..20 {
        let mut ops = Vec::new();
        for _ in 0..100 {
            ops.push(Op::Driver {
                x: rng.next_u32(),
                y: rng.next_u32(),
                b: rng.next_u8(),
                z: rng.next_i32(),
            });
        }
        // Batched: one capture per library, so concatenation + ordering is
        // compared as a single byte stream.
        assert_batch_eq(&l, &ops, "row15");
    }
}

// --- Row 16: interleaved entry points in one session --------------------

#[test]
fn row16_interleaved_entry_points() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);
    let mut ops = Vec::new();
    for _ in 0..6000 {
        match rng.below(3) {
            0 => ops.push(Op::Driver {
                x: rng.next_u32(),
                y: rng.next_u32(),
                b: rng.next_u8(),
                z: rng.next_i32(),
            }),
            1 => {
                let mut raw = [0u8; FOO_SIZE];
                rng.fill(&mut raw);
                ops.push(Op::PrintFoo { raw });
            }
            _ => {
                let mut raw = [0u8; FOO_SIZE];
                rng.fill(&mut raw);
                ops.push(Op::PrintFooMisaligned { raw });
            }
        }
    }
    assert_batch_eq_chunked(&l, &ops, "row16");
}

// --- Row 17: both libraries live, calls interleaved on the same stdout ---

#[test]
fn row17_both_libraries_interleaved_same_stdout() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 17);
    let c_driver = l.c_driver();
    let r_driver = l.rust_driver();

    // Interleave C and Rust calls inside ONE capture. Each pair must emit two
    // identical lines, proving neither library carries divergent global state
    // and that both share the same stdout buffer semantics.
    let mut args = Vec::new();
    for _ in 0..2000 {
        args.push((
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u8(),
            rng.next_i32(),
        ));
    }
    let out = capture_stdout(|| {
        for &(x, y, b, z) in &args {
            unsafe { c_driver(x, y, b, z) };
            unsafe { r_driver(x, y, b, z) };
        }
    });
    let lines: Vec<&[u8]> = out.split(|&c| c == b'\n').collect();
    // Trailing empty element after the final newline.
    assert_eq!(
        lines.len(),
        args.len() * 2 + 1,
        "expected {} lines, got {}",
        args.len() * 2,
        lines.len() - 1
    );
    for (i, &(x, y, b, z)) in args.iter().enumerate() {
        assert_eq!(
            lines[2 * i],
            lines[2 * i + 1],
            "interleaved pair {i} diverged for driver({x},{y},{b},{z}): {:?} vs {:?}",
            String::from_utf8_lossy(lines[2 * i]),
            String::from_utf8_lossy(lines[2 * i + 1]),
        );
    }
}

// --- Row 18: dirty upper bits in the `bool` argument register -----------

#[test]
fn row18_bool_dirty_upper_argument_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);

    // Same ABI slot, but declared `u32` so the whole 32-bit register is written
    // rather than just the low byte. The C `driver` prologue keeps only `%al`,
    // so the upper 24 bits must be irrelevant in both libraries.
    type DriverWide = unsafe extern "C" fn(u32, u32, u32, i32);
    let c_wide: libloading::Symbol<DriverWide> =
        unsafe { l.c.get(b"driver\0").expect("C driver") };
    let r_wide: libloading::Symbol<DriverWide> =
        unsafe { l.rust.get(b"driver\0").expect("Rust driver") };

    for _ in 0..4000 {
        let x = rng.next_u32();
        let y = rng.next_u32();
        let wide_b = rng.next_u32();
        let z = rng.next_i32();

        let c_out = capture_stdout(|| unsafe { c_wide(x, y, wide_b, z) });
        let r_out = capture_stdout(|| unsafe { r_wide(x, y, wide_b, z) });
        assert_eq!(
            c_out, r_out,
            "driver({x},{y},{wide_b:#x} as wide bool,{z}) diverged"
        );

        // And it must agree with passing only the low byte.
        let low = (wide_b & 0xFF) as u8;
        let cf = l.c_driver();
        let narrow = capture_stdout(|| unsafe { cf(x, y, low, z) });
        assert_eq!(
            c_out, narrow,
            "C driver was sensitive to the upper 24 bits of the bool argument"
        );
    }
}
