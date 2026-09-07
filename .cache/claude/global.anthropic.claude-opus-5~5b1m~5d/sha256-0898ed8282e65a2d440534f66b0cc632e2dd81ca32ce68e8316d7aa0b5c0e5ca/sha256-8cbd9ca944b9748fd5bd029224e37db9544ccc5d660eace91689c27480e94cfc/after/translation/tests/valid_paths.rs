//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every row that is not inherently a single
//! point is driven with many randomized inputs from a fixed-seed PRNG.

mod common;

use common::*;

/// How many randomized inputs each property-style row uses.
const N: usize = 400;

// ---------------------------------------------------------------------------
// Row 1 — driver, all-zero input (minimal shape).
// ---------------------------------------------------------------------------
#[test]
fn cfg01_driver_all_zero() {
    assert_driver_eq(0, 0, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 2 — driver, exhaustive in-range bit-field cross-product, z = 0.
// ---------------------------------------------------------------------------
#[test]
fn cfg02_driver_exhaustive_inrange_bitfields() {
    for x in 0u32..4 {
        for y in 0u32..8 {
            for b in 0u8..2 {
                assert_driver_eq(x, y, b, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — the same 64 combinations x randomized z.
// ---------------------------------------------------------------------------
#[test]
fn cfg03_driver_exhaustive_bitfields_x_random_z() {
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for x in 0u32..4 {
        for y in 0u32..8 {
            for b in 0u8..2 {
                for _ in 0..8 {
                    assert_driver_eq(x, y, b, rng.next_i32());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — in-range x/y/b, z randomized over the full i32 range.
// ---------------------------------------------------------------------------
#[test]
fn cfg04_driver_random_z_full_range() {
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    for _ in 0..N {
        let x = rng.below(4);
        let y = rng.below(8);
        let b = (rng.next_u8() & 1) as u8;
        assert_driver_eq(x, y, b, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 5 — in-range x/y/b, z from the boundary set.
// ---------------------------------------------------------------------------
#[test]
fn cfg05_driver_z_boundaries() {
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for &z in Z_BOUNDARIES.iter() {
        // every in-range bit-field combination against each boundary z
        for x in 0u32..4 {
            for y in 0u32..8 {
                for b in 0u8..2 {
                    assert_driver_eq(x, y, b, z);
                }
            }
        }
        // plus a few randomized bit-fields for good measure
        for _ in 0..8 {
            assert_driver_eq(rng.below(4), rng.below(8), rng.next_u8() & 1, z);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — x truncating (x >= 4, full u32) x in-range y/b x random z.
// ---------------------------------------------------------------------------
#[test]
fn cfg06_driver_x_truncating() {
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for _ in 0..N {
        let x = rng.next_u32() | 4; // guarantee >= 4
        assert_driver_eq(x, rng.below(8), rng.next_u8() & 1, rng.next_i32());
    }
    // and the small out-of-range values explicitly
    for x in 4u32..32 {
        assert_driver_eq(x, 5, 1, -7);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — y truncating (y >= 8, full u32) x in-range x/b x random z.
// ---------------------------------------------------------------------------
#[test]
fn cfg07_driver_y_truncating() {
    let mut rng = Rng::new(0x0707_0707_0707_0707);
    for _ in 0..N {
        let y = rng.next_u32() | 8; // guarantee >= 8
        assert_driver_eq(rng.below(4), y, rng.next_u8() & 1, rng.next_i32());
    }
    for y in 8u32..64 {
        assert_driver_eq(2, y, 0, 12345);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — both x and y truncating simultaneously.
// ---------------------------------------------------------------------------
#[test]
fn cfg08_driver_both_truncating() {
    let mut rng = Rng::new(0x0808_0808_0808_0808);
    for _ in 0..N {
        let x = rng.next_u32() | 4;
        let y = rng.next_u32() | 8;
        assert_driver_eq(x, y, rng.next_u8() & 1, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 9 — b non-canonical byte (2..=255) x randomized x/y/z.
// ---------------------------------------------------------------------------
#[test]
fn cfg09_driver_b_non_canonical() {
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for b in 2u8..=255 {
        assert_driver_eq(rng.next_u32(), rng.next_u32(), b, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 10 — fully unconstrained fuzz: every axis random at once.
// ---------------------------------------------------------------------------
#[test]
fn cfg10_driver_unconstrained_fuzz() {
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for _ in 0..(N * 3) {
        assert_driver_eq(
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u8(),
            rng.next_i32(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 11 — all axes simultaneously at their extremes.
// ---------------------------------------------------------------------------
#[test]
fn cfg11_driver_all_axes_extreme() {
    assert_driver_eq(u32::MAX, u32::MAX, 0xFF, i32::MIN);
    assert_driver_eq(u32::MAX, u32::MAX, 0xFF, i32::MAX);
    assert_driver_eq(u32::MAX, u32::MAX, 0x00, i32::MIN);
    assert_driver_eq(0, 0, 0xFF, i32::MIN);
    assert_driver_eq(u32::MAX, 0, 1, 0);
    assert_driver_eq(0, u32::MAX, 1, 0);
}

// ---------------------------------------------------------------------------
// Row 12 — print_foo, minimal low-level shape.
// ---------------------------------------------------------------------------
#[test]
fn cfg12_print_foo_all_zero() {
    assert_print_foo_eq(0, [0, 0, 0], 0);
}

// ---------------------------------------------------------------------------
// Row 13 — print_foo, exhaustive significant storage bytes 0x00..=0x3F.
// ---------------------------------------------------------------------------
#[test]
fn cfg13_print_foo_exhaustive_significant_bits() {
    for storage in 0x00u8..=0x3F {
        assert_print_foo_eq(storage, [0, 0, 0], 0);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — print_foo, exhaustive storage byte 0x00..=0xFF x randomized z.
// ---------------------------------------------------------------------------
#[test]
fn cfg14_print_foo_exhaustive_all_storage_bytes() {
    let mut rng = Rng::new(0x1414_1414_1414_1414);
    for storage in 0x00u8..=0xFF {
        for _ in 0..4 {
            assert_print_foo_eq(storage, [0, 0, 0], rng.next_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — print_foo, random storage x garbage inter-field padding x random z.
// ---------------------------------------------------------------------------
#[test]
fn cfg15_print_foo_garbage_padding() {
    let mut rng = Rng::new(0x1515_1515_1515_1515);
    for _ in 0..N {
        let storage = rng.next_u8();
        let pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        assert_print_foo_eq(storage, pad, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 16 — print_foo, random storage x boundary z.
// ---------------------------------------------------------------------------
#[test]
fn cfg16_print_foo_z_boundaries() {
    let mut rng = Rng::new(0x1616_1616_1616_1616);
    for &z in Z_BOUNDARIES.iter() {
        for storage in 0x00u8..=0x3F {
            assert_print_foo_eq(storage, [0, 0, 0], z);
        }
        for _ in 0..8 {
            let pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
            assert_print_foo_eq(rng.next_u8(), pad, z);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — print_foo, fully randomized 8-byte struct image.
// ---------------------------------------------------------------------------
#[test]
fn cfg17_print_foo_random_full_image() {
    let p = pair();
    let mut rng = Rng::new(0x1717_1717_1717_1717);
    for _ in 0..(N * 2) {
        let mut img = [0u8; FOO_SIZE];
        for byte in img.iter_mut() {
            *byte = rng.next_u8();
        }
        let c = run_print_foo(&p.c, &img);
        let r = run_print_foo(&p.rust, &img);
        assert_eq!(
            c,
            r,
            "print_foo(raw image {img:02x?}) diverged:\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 18 — composed pipeline: driver's packing must agree with print_foo.
//
// Four outputs are compared for the same logical input:
//   C driver, Rust driver, C print_foo(packed image), Rust print_foo(packed).
// This catches a divergence in the composition (bit packing) that per-function
// tests would miss.
// ---------------------------------------------------------------------------
#[test]
fn cfg18_composed_driver_and_print_foo_agree() {
    let p = pair();
    let mut rng = Rng::new(0x1818_1818_1818_1818);
    for i in 0..(N * 2) {
        // Mix exhaustive small cases with unconstrained random ones.
        let (x, y, b, z) = if i < 64 {
            let x = (i as u32) & 0x3;
            let y = ((i as u32) >> 2) & 0x7;
            let b = ((i as u32) >> 5) as u8 & 0x1;
            (x, y, b, rng.next_i32())
        } else {
            (
                rng.next_u32(),
                rng.next_u32(),
                rng.next_u8(),
                rng.next_i32(),
            )
        };

        let img = foo_image(packed_storage(x, y, b), [0, 0, 0], z);

        let c_drv = run_driver(&p.c, x, y, b, z);
        let r_drv = run_driver(&p.rust, x, y, b, z);
        let c_pf = run_print_foo(&p.c, &img);
        let r_pf = run_print_foo(&p.rust, &img);

        assert_eq!(
            c_drv, r_drv,
            "driver diverged for (x={x:#x}, y={y:#x}, b={b:#x}, z={z})"
        );
        assert_eq!(
            c_pf, r_pf,
            "print_foo diverged for packed image {img:02x?}"
        );
        // The pipeline: driver(x,y,b,z) == print_foo(pack(x,y,b,z)) in C ...
        assert_eq!(
            c_drv, c_pf,
            "C driver vs C print_foo disagree for (x={x:#x}, y={y:#x}, b={b:#x}, z={z})"
        );
        // ... and the Rust must reproduce that same relationship.
        assert_eq!(
            r_drv, r_pf,
            "Rust driver vs Rust print_foo disagree for (x={x:#x}, y={y:#x}, b={b:#x}, z={z})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 19 — many interleaved calls in one capture: compares the whole
// accumulated stdout stream, catching buffering / statefulness divergence.
// ---------------------------------------------------------------------------
#[test]
fn cfg19_interleaved_stream_without_flush() {
    let p = pair();
    let mut rng = Rng::new(0x1919_1919_1919_1919);

    // Build one shared input script.
    let mut script: Vec<(bool, u32, u32, u8, i32, u8, [u8; 3])> = Vec::new();
    for _ in 0..500 {
        let use_driver = rng.next_u8() & 1 == 0;
        script.push((
            use_driver,
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u8(),
            rng.next_i32(),
            rng.next_u8(),
            [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        ));
    }

    let replay = |imp: &Impl| -> Vec<u8> {
        capture_stdout(|| {
            for &(use_driver, x, y, b, z, storage, pad) in script.iter() {
                unsafe {
                    if use_driver {
                        (imp.driver)(x, y, b, z);
                    } else {
                        let img = foo_image(storage, pad, z);
                        (imp.print_foo)(img.as_ptr());
                    }
                }
            }
        })
    };

    let c = replay(&p.c);
    let r = replay(&p.rust);
    assert_eq!(
        c.len(),
        r.len(),
        "interleaved stream length differs: C={} Rust={}",
        c.len(),
        r.len()
    );
    // Report the first differing line for a useful failure message.
    if c != r {
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            assert_eq!(
                a,
                b,
                "interleaved stream diverged at line {i}: C={:?} Rust={:?}",
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b)
            );
        }
    }
    assert_eq!(c, r, "interleaved stdout streams differ");
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        script.len(),
        "expected exactly one line per call"
    );
}

// ---------------------------------------------------------------------------
// Row 20 — there is no binary/driver executable to compare, by construction.
// ---------------------------------------------------------------------------
#[test]
fn cfg20_no_binary_target_exists() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[[bin]]"),
        "Cargo.toml unexpectedly declares a binary target; Phase B must then \
         compare C and Rust executable stdout"
    );
    let cmakelists = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/CMakeLists.txt"
    ))
    .expect("read CMakeLists.txt");
    assert!(
        !cmakelists.contains("add_executable"),
        "CMakeLists.txt unexpectedly declares an executable target"
    );
    assert!(
        cmakelists.contains("add_library(driver SHARED"),
        "CMakeLists.txt no longer builds the expected shared library"
    );
    // No `main` in the C sources either.
    let c = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/src/driver.c"))
        .expect("read driver.c");
    assert!(!c.contains("int main"), "driver.c unexpectedly has a main()");
}
