//! In-process FFI differential checks (`libloading` in this very process).
//!
//! This test binary runs with `harness = false` (see Cargo.toml) because the
//! checks temporarily redirect fd 1 in order to capture what the two loaded
//! shared objects `printf`/`write` — `libtest`'s own progress output must not
//! be able to land in that capture from another thread.
//!
//! Rows covered: CONFIGS.md C1..C12 + C27, ERRORS.md F1..F4 + F6..F8.

mod common;

use common::*;

// ===========================================================================
// print_foo — the lowest level entry point
// ===========================================================================

/// C1: exhaustive over all 256 values of the bit-field storage byte.
fn cfg_c01_print_foo_all_bits() {
    let cases: Vec<(u8, [u8; 3], i32)> = (0u16..=255).map(|b| (b as u8, [0, 0, 0], 0)).collect();
    assert_print_foo_batch(&cases, "C1 print_foo all bit patterns");
}

/// C2: exhaustive bit-field byte × interesting `z` values.
fn cfg_c02_print_foo_bits_x_z() {
    let zs = [0i32, 1, -1, i32::MIN, i32::MAX, 0x7f7f_7f7f, -0x7f7f_7f7f, 42];
    let mut cases = Vec::new();
    for z in zs {
        for b in 0u16..=255 {
            cases.push((b as u8, [0, 0, 0], z));
        }
    }
    assert_print_foo_batch(&cases, "C2 print_foo bits × z");
}

/// C3: randomized bit-field byte × randomized `z`.
fn cfg_c03_print_foo_random() {
    let mut rng = Rng::new(SEED ^ 3);
    let cases: Vec<(u8, [u8; 3], i32)> = (0..5000)
        .map(|_| {
            (
                rng.next_u32() as u8,
                [0, 0, 0],
                rng.next_u32() as i32,
            )
        })
        .collect();
    assert_print_foo_batch(&cases, "C3 print_foo random");
}

/// C4: the two padding bits (6..7) of the storage byte are set — the C code
/// masks them away.
fn cfg_c04_print_foo_padding_bits_set() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases = Vec::new();
    for hi in [0x00u8, 0x40, 0x80, 0xC0] {
        for lo in 0u8..64 {
            cases.push((hi | lo, [0, 0, 0], rng.next_u32() as i32));
        }
    }
    assert_print_foo_batch(&cases, "C4 print_foo padding bits set");
}

/// C5: the padding bytes 1..3 of the bit-field allocation unit hold garbage.
fn cfg_c05_print_foo_padding_bytes() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut cases = Vec::new();
    for pad in [[0u8; 3], [0xAA; 3], [0xFF; 3], [0x01, 0x80, 0x7f]] {
        for _ in 0..256 {
            cases.push((rng.next_u32() as u8, pad, rng.next_u32() as i32));
        }
    }
    for _ in 0..1000 {
        let pad = [rng.next_u32() as u8, rng.next_u32() as u8, rng.next_u32() as u8];
        cases.push((rng.next_u32() as u8, pad, rng.next_u32() as i32));
    }
    assert_print_foo_batch(&cases, "C5 print_foo padding bytes");
}

/// C6: `z` sweeping all powers of two and their negations.
fn cfg_c06_print_foo_z_powers() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut cases = Vec::new();
    for bit in 0..32u32 {
        let v = 1i32.wrapping_shl(bit);
        for z in [v, v.wrapping_neg(), v.wrapping_sub(1), v.wrapping_add(1)] {
            cases.push((rng.next_u32() as u8, [0, 0, 0], z));
        }
    }
    assert_print_foo_batch(&cases, "C6 print_foo z powers of two");
}

// ===========================================================================
// driver
// ===========================================================================

/// C7: exhaustive small in-range grid.
fn cfg_c07_driver_small_grid() {
    let mut cases = Vec::new();
    for x in 0u32..=8 {
        for y in 0u32..=8 {
            for b in 0u8..=1 {
                for z in [0i32, 1, -1] {
                    cases.push((x, y, b, z));
                }
            }
        }
    }
    assert_driver_batch(&cases, "C7 driver small grid");
}

/// C8: every possible `_Bool` byte a C caller can pass.
fn cfg_c08_driver_all_bool_bytes() {
    let mut rng = Rng::new(SEED ^ 8);
    let cases: Vec<(u32, u32, u8, i32)> = (0u16..=255)
        .map(|b| (rng.next_u32(), rng.next_u32(), b as u8, rng.next_u32() as i32))
        .collect();
    assert_driver_batch(&cases, "C8 driver all bool bytes");
}

/// C9: `x`/`y` around every bit-field residue boundary.
fn cfg_c09_driver_xy_boundaries() {
    let mut rng = Rng::new(SEED ^ 9);
    let interesting: Vec<u32> = (0u32..=16)
        .chain([31, 32, 63, 64, 255, 256, 1 << 15, 1 << 16, 1 << 30])
        .chain([i32::MAX as u32, 1 << 31, u32::MAX - 1, u32::MAX])
        .collect();
    let mut cases = Vec::new();
    for &x in &interesting {
        for &y in &interesting {
            cases.push((x, y, (rng.next_u32() & 1) as u8, rng.next_u32() as i32));
        }
    }
    assert_driver_batch(&cases, "C9 driver x/y boundaries");
}

/// C10: `z` at the extremes of the `int` range.
fn cfg_c10_driver_z_boundaries() {
    let mut rng = Rng::new(SEED ^ 10);
    let zs = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        -2147483647,
    ];
    let cases: Vec<(u32, u32, u8, i32)> = zs
        .iter()
        .flat_map(|&z| {
            (0..16).map(move |_| z).collect::<Vec<_>>()
        })
        .map(|z| (rng.next_u32(), rng.next_u32(), rng.next_u32() as u8, z))
        .collect();
    assert_driver_batch(&cases, "C10 driver z boundaries");
}

/// C11: fully randomized arguments.
fn cfg_c11_driver_random() {
    let mut rng = Rng::new(SEED ^ 11);
    let cases: Vec<(u32, u32, u8, i32)> = (0..5000)
        .map(|_| {
            (
                rng.next_u32(),
                rng.next_u32(),
                rng.next_u32() as u8,
                rng.next_u32() as i32,
            )
        })
        .collect();
    assert_driver_batch(&cases, "C11 driver random");
}

/// C12: many calls in one process — neither side may accumulate state.
fn cfg_c12_driver_repeated_calls() {
    let cases: Vec<(u32, u32, u8, i32)> = (0..500).map(|i| (i, i * 3, (i % 2) as u8, -(i as i32))).collect();
    assert_driver_batch(&cases, "C12 driver repeated calls");
    // and again, to prove the second batch is unaffected by the first
    assert_driver_batch(&cases, "C12 driver repeated calls (2)");
}

/// C27: composition — `driver(x,y,b,z)` must equal `print_foo` applied to the
/// byte image that the C bit-field store produces.
fn cfg_c27_driver_print_foo_pipeline() {
    let mut rng = Rng::new(SEED ^ 27);
    let args: Vec<(u32, u32, u8, i32)> = (0..2000)
        .map(|_| {
            (
                rng.next_u32(),
                rng.next_u32(),
                rng.next_u32() as u8,
                rng.next_u32() as i32,
            )
        })
        .collect();
    let (c, r) = impls();

    // C's driver, C's print_foo, Rust's driver, Rust's print_foo — all four
    // must produce the same stream.
    let c_driver = capture_stdout(|| {
        let f = c.driver();
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
    let c_pf = capture_stdout(|| {
        let f = c.print_foo();
        for &(x, y, b, z) in &args {
            let bits = ((x & 3) as u8) | (((y & 7) as u8) << 2) | ((b & 1) << 5);
            let raw = RawFoo::new(bits, [0, 0, 0], z);
            unsafe { f(raw.as_ptr()) };
        }
    });
    let r_driver = capture_stdout(|| {
        let f = r.driver();
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
    let r_pf = capture_stdout(|| {
        let f = r.print_foo();
        for &(x, y, b, z) in &args {
            let bits = ((x & 3) as u8) | (((y & 7) as u8) << 2) | ((b & 1) << 5);
            let raw = RawFoo::new(bits, [0, 0, 0], z);
            unsafe { f(raw.as_ptr()) };
        }
    });
    assert_eq!(c_driver, c_pf, "C27: C driver vs C print_foo");
    assert_eq!(c_driver, r_driver, "C27: C driver vs Rust driver");
    assert_eq!(c_pf, r_pf, "C27: C print_foo vs Rust print_foo");
}


// ===========================================================================
// ERRORS.md rows reachable in-process (F1..F4, F6..F8)
// ===========================================================================

/// F1: `x` out of range for `unsigned int x : 2` — silently truncated.
fn err_f01_driver_x_out_of_range() {
    let mut cases = Vec::new();
    for x in [4u32, 5, 6, 7, 8, 100, 0xFFFF_FFFC, 0xFFFF_FFFD, u32::MAX] {
        cases.push((x, 0, 0u8, 0i32));
    }
    // and every value 0..=1024, whose residue mod 4 is what must survive
    for x in 0u32..=1024 {
        cases.push((x, 1, 1, -1));
    }
    assert_driver_batch(&cases, "F1 driver x out of range");
}

/// F2: `y` out of range for `unsigned int y : 3` — silently truncated.
fn err_f02_driver_y_out_of_range() {
    let mut cases = Vec::new();
    for y in [8u32, 9, 15, 16, 17, 255, 256, 0xFFFF_FFF8, u32::MAX] {
        cases.push((0, y, 0u8, 0i32));
    }
    for y in 0u32..=1024 {
        cases.push((1, y, 1, 7));
    }
    assert_driver_batch(&cases, "F2 driver y out of range");
}

/// F3: `_Bool` byte outside `{0, 1}` — an out-of-range value for a type with
/// only two valid variants, which C nevertheless accepts across the FFI
/// boundary.  gcc stores `b & 1`.
fn err_f03_driver_bool_out_of_range() {
    let cases: Vec<(u32, u32, u8, i32)> = (0u16..=255).map(|b| (3, 7, b as u8, -12345)).collect();
    assert_driver_batch(&cases, "F3 driver bool out of range");
}

/// F4: `z` at the extremes of `int`.
fn err_f04_driver_z_extremes() {
    let cases = vec![
        (0u32, 0u32, 0u8, i32::MIN),
        (0, 0, 0, i32::MIN + 1),
        (0, 0, 0, i32::MAX),
        (0, 0, 0, i32::MAX - 1),
        (3, 7, 1, i32::MIN),
        (3, 7, 1, i32::MAX),
        (3, 7, 1, -1),
    ];
    assert_driver_batch(&cases, "F4 driver z extremes");
}

/// F6: padding bits 6..7 of the bit-field byte are ignored by `print_foo`.
fn err_f06_print_foo_padding_bits() {
    let mut cases = Vec::new();
    for lo in 0u8..64 {
        for hi in [0x00u8, 0x40, 0x80, 0xC0] {
            cases.push((lo | hi, [0u8; 3], 5i32));
        }
    }
    assert_print_foo_batch(&cases, "F6 print_foo padding bits");
    // Also assert the four hi variants of one lo value produce equal lines.
    let (c, _r) = impls();
    let out = capture_stdout(|| {
        let f = c.print_foo();
        for hi in [0x00u8, 0x40, 0x80, 0xC0] {
            let raw = RawFoo::new(0x2A | hi, [0; 3], 9);
            unsafe { f(raw.as_ptr()) };
        }
    });
    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 4);
    assert!(
        lines.iter().all(|l| *l == lines[0]),
        "F6: padding bits changed the C output: {out:?}"
    );
}

/// F7: padding bytes 1..3 are never read.
fn err_f07_print_foo_padding_bytes() {
    let mut cases = Vec::new();
    for pad in [
        [0u8, 0, 0],
        [0xFF, 0xFF, 0xFF],
        [0xAA, 0x55, 0xF0],
        [0x01, 0x00, 0x00],
        [0x00, 0x00, 0x80],
    ] {
        for bits in [0u8, 1, 0x2A, 0x3F, 0xFF] {
            cases.push((bits, pad, -7i32));
        }
    }
    assert_print_foo_batch(&cases, "F7 print_foo padding bytes");
}

/// F8: every interesting `z` bit pattern.
fn err_f08_print_foo_z_patterns() {
    let mut cases = Vec::new();
    let mut push = |z: i32| cases.push((0x2Au8, [0u8; 3], z));
    for b in 0..32 {
        push(1i32.wrapping_shl(b));
        push((1i32.wrapping_shl(b)).wrapping_neg());
    }
    for z in [0, -1, 1, i32::MIN, i32::MAX, -2147483647, 0x5555_5555u32 as i32] {
        push(z);
    }
    assert_print_foo_batch(&cases, "F8 print_foo z patterns");
}

/// Sanity: both shared objects really do export the three C symbols (the
/// in-process `dlsym` view of Phase D).
fn sym_exports_present() {
    let (c, r) = impls();
    for sym in [&b"driver"[..], b"print_foo", b"main"] {
        let name = String::from_utf8_lossy(sym).to_string();
        assert!(c.has(sym), "C .so is missing `{name}`");
        assert!(r.has(sym), "Rust .so is missing `{name}`");
    }
}

fn main() {
    run_checks(&[
        ("sym_exports_present", sym_exports_present),
        ("cfg_c01_print_foo_all_bits", cfg_c01_print_foo_all_bits),
        ("cfg_c02_print_foo_bits_x_z", cfg_c02_print_foo_bits_x_z),
        ("cfg_c03_print_foo_random", cfg_c03_print_foo_random),
        (
            "cfg_c04_print_foo_padding_bits_set",
            cfg_c04_print_foo_padding_bits_set,
        ),
        ("cfg_c05_print_foo_padding_bytes", cfg_c05_print_foo_padding_bytes),
        ("cfg_c06_print_foo_z_powers", cfg_c06_print_foo_z_powers),
        ("cfg_c07_driver_small_grid", cfg_c07_driver_small_grid),
        ("cfg_c08_driver_all_bool_bytes", cfg_c08_driver_all_bool_bytes),
        ("cfg_c09_driver_xy_boundaries", cfg_c09_driver_xy_boundaries),
        ("cfg_c10_driver_z_boundaries", cfg_c10_driver_z_boundaries),
        ("cfg_c11_driver_random", cfg_c11_driver_random),
        ("cfg_c12_driver_repeated_calls", cfg_c12_driver_repeated_calls),
        (
            "cfg_c27_driver_print_foo_pipeline",
            cfg_c27_driver_print_foo_pipeline,
        ),
        ("err_f01_driver_x_out_of_range", err_f01_driver_x_out_of_range),
        ("err_f02_driver_y_out_of_range", err_f02_driver_y_out_of_range),
        (
            "err_f03_driver_bool_out_of_range",
            err_f03_driver_bool_out_of_range,
        ),
        ("err_f04_driver_z_extremes", err_f04_driver_z_extremes),
        ("err_f06_print_foo_padding_bits", err_f06_print_foo_padding_bits),
        ("err_f07_print_foo_padding_bytes", err_f07_print_foo_padding_bytes),
        ("err_f08_print_foo_z_patterns", err_f08_print_foo_z_patterns),
    ]);
}
