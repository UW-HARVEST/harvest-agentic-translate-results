//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported symbols and compares the exact bytes they
//! write to stdout.

mod common;

use common::{assert_same, foo_bytes, impls, Rng, Which};

/// 4-byte aligned scratch buffer with room to also test misaligned pointers.
#[repr(align(8))]
struct Aligned([u8; 16]);

// ---------------------------------------------------------------------------
// row 1 — exhaustive small grid over driver's four arguments
// ---------------------------------------------------------------------------
fn row01_driver_small_exhaustive_grid() {
    let mut cases = Vec::new();
    for x in 0u32..=7 {
        for y in 0u32..=15 {
            for b in 0u8..=1 {
                for z in [0i32, 1, -1] {
                    cases.push(format!("driver(x={x}, y={y}, b={b}, z={z})"));
                }
            }
        }
    }
    let args: Vec<(u32, u32, u8, i32)> = (0u32..=7)
        .flat_map(|x| {
            (0u32..=15).flat_map(move |y| {
                (0u8..=1).flat_map(move |b| [0i32, 1, -1].map(move |z| (x, y, b, z)))
            })
        })
        .collect();

    assert_same("row01", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 2 — random full-range x/y (bit-field truncation), z = 0
// ---------------------------------------------------------------------------
fn row02_driver_random_xy_full_range() {
    let mut rng = Rng::new(0x0000_0002_5EED);
    let mut args = Vec::new();
    for _ in 0..4096 {
        args.push((rng.interesting_u32(), rng.interesting_u32(), (rng.next_u8() & 1), 0i32));
    }
    let cases: Vec<String> = args
        .iter()
        .map(|(x, y, b, z)| format!("driver(x={x}, y={y}, b={b}, z={z})"))
        .collect();
    assert_same("row02", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 3 — in-range x/y, random full-range i32 z
// ---------------------------------------------------------------------------
fn row03_driver_random_z() {
    let mut rng = Rng::new(0x0000_0003_5EED);
    let mut args = Vec::new();
    for _ in 0..4096 {
        args.push((
            rng.below(4),
            rng.below(8),
            (rng.next_u8() & 1),
            rng.interesting_u32() as i32,
        ));
    }
    let cases: Vec<String> = args
        .iter()
        .map(|(x, y, b, z)| format!("driver(x={x}, y={y}, b={b}, z={z})"))
        .collect();
    assert_same("row03", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 4 — fully random 4-tuples, including non-canonical bool bytes
// ---------------------------------------------------------------------------
fn row04_driver_fully_random() {
    let mut rng = Rng::new(0x0000_0004_5EED);
    let mut args = Vec::new();
    for _ in 0..20_000 {
        args.push((
            rng.interesting_u32(),
            rng.interesting_u32(),
            rng.next_u8(),
            rng.interesting_u32() as i32,
        ));
    }
    let cases: Vec<String> = args
        .iter()
        .map(|(x, y, b, z)| format!("driver(x={x}, y={y}, b={b:#04x}, z={z})"))
        .collect();
    assert_same("row04", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 5 — boundary cross-product
// ---------------------------------------------------------------------------
fn row05_driver_boundary_cross_product() {
    const ZS: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    const XS: [u32; 4] = [0, 3, 4, u32::MAX];
    const YS: [u32; 4] = [0, 7, 8, u32::MAX];
    let mut args = Vec::new();
    for &x in &XS {
        for &y in &YS {
            for b in 0u8..=1 {
                for &z in &ZS {
                    args.push((x, y, b, z));
                }
            }
        }
    }
    let cases: Vec<String> = args
        .iter()
        .map(|(x, y, b, z)| format!("driver(x={x}, y={y}, b={b}, z={z})"))
        .collect();
    assert_same("row05", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 6 — every possible bool byte 0..=255
// ---------------------------------------------------------------------------
fn row06_driver_every_bool_byte() {
    let cases: Vec<String> = (0u16..=255)
        .map(|b| format!("driver(1, 5, b={b:#04x}, -7)"))
        .collect();
    assert_same("row06", &cases, |w| {
        let f = impls().driver(w);
        for b in 0u16..=255 {
            unsafe { f(1, 5, b as u8, -7) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 7 — wide value in the `_Bool` argument register (only %dl is ABI-relevant)
// ---------------------------------------------------------------------------
fn row07_driver_wide_bool_register() {
    const WIDE: [u32; 10] = [
        0x0000_0000,
        0x0000_0001,
        0x0000_0002,
        0x0000_0100,
        0x0000_0101,
        0x0000_FF00,
        0x0000_FF01,
        0xFFFF_FF00,
        0xFFFF_FF01,
        0xFFFF_FFFF,
    ];
    let cases: Vec<String> = WIDE
        .iter()
        .map(|b| format!("driver(2, 6, b_reg={b:#010x}, 42)"))
        .collect();
    assert_same("row07", &cases, |w| {
        let f = impls().driver_wide(w);
        for &b in &WIDE {
            unsafe { f(2, 6, b, 42) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 8 — print_foo, exhaustive bit-field byte, zero padding, z = 0
// ---------------------------------------------------------------------------
fn row08_print_foo_exhaustive_bitfield_byte() {
    let cases: Vec<String> = (0u16..=255)
        .map(|b| format!("print_foo([{b:#04x}, 0,0,0, z=0])"))
        .collect();
    assert_same("row08", &cases, |w| {
        let f = impls().print_foo(w);
        for b in 0u16..=255 {
            let buf = Aligned([b as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            unsafe { f(buf.0.as_ptr()) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 9 — same, with 0xFF padding and z = INT_MIN
// ---------------------------------------------------------------------------
fn row09_print_foo_exhaustive_with_ff_padding() {
    let cases: Vec<String> = (0u16..=255)
        .map(|b| format!("print_foo([{b:#04x}, FF,FF,FF, z=INT_MIN])"))
        .collect();
    assert_same("row09", &cases, |w| {
        let f = impls().print_foo(w);
        for b in 0u16..=255 {
            let z = i32::MIN.to_le_bytes();
            let buf = Aligned([
                b as u8, 0xFF, 0xFF, 0xFF, z[0], z[1], z[2], z[3], 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            unsafe { f(buf.0.as_ptr()) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 10 — fully random heap-allocated 8-byte images
// ---------------------------------------------------------------------------
fn row10_print_foo_random_heap_buffers() {
    let mut rng = Rng::new(0x0000_0010_5EED);
    let mut bufs: Vec<[u8; 8]> = Vec::new();
    for _ in 0..20_000 {
        let mut b = [0u8; 8];
        for byte in b.iter_mut() {
            *byte = rng.next_u8();
        }
        bufs.push(b);
    }
    let cases: Vec<String> = bufs.iter().map(|b| format!("print_foo({b:02x?})")).collect();
    // 8-byte aligned heap storage.
    let mut storage: Vec<u64> = bufs.iter().map(|b| u64::from_le_bytes(*b)).collect();
    let base = storage.as_mut_ptr() as *const u8;
    assert_same("row10", &cases, |w| {
        let f = impls().print_foo(w);
        for i in 0..bufs.len() {
            unsafe { f(base.add(i * 8)) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 11 — misaligned pointers (offset 1, 2, 3)
// ---------------------------------------------------------------------------
fn row11_print_foo_misaligned_pointers() {
    let mut rng = Rng::new(0x0000_0011_5EED);
    let mut bufs: Vec<[u8; 16]> = Vec::new();
    for _ in 0..3000 {
        let mut b = [0u8; 16];
        for byte in b.iter_mut() {
            *byte = rng.next_u8();
        }
        bufs.push(b);
    }
    let mut cases = Vec::new();
    for b in &bufs {
        for off in 1..=3usize {
            cases.push(format!("print_foo(buf{b:02x?} + {off})"));
        }
    }
    // 8-byte aligned backing store so that base+off is misaligned by exactly off.
    let mut storage: Vec<[u64; 2]> = bufs
        .iter()
        .map(|b| {
            [
                u64::from_le_bytes(b[0..8].try_into().unwrap()),
                u64::from_le_bytes(b[8..16].try_into().unwrap()),
            ]
        })
        .collect();
    let base = storage.as_mut_ptr() as *const u8;
    let n = bufs.len();
    assert_same("row11", &cases, |w| {
        let f = impls().print_foo(w);
        for i in 0..n {
            for off in 1..=3usize {
                unsafe { f(base.add(i * 16 + off)) };
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 12 — static / stack storage
// ---------------------------------------------------------------------------
static STATIC_FOO: [u8; 8] = [0x2D, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x7F];

fn row12_print_foo_static_and_stack_storage() {
    let cases: Vec<String> = vec![
        "print_foo(&STATIC_FOO)".into(),
        "print_foo(&stack_copy)".into(),
        "print_foo(&heap_copy)".into(),
    ];
    assert_same("row12", &cases, |w| {
        let f = impls().print_foo(w);
        unsafe { f(STATIC_FOO.as_ptr()) };
        let stack = Aligned([
            0x2D, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x7F, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        unsafe { f(stack.0.as_ptr()) };
        let heap: Box<[u8; 8]> = Box::new(STATIC_FOO);
        unsafe { f(heap.as_ptr()) };
    });
}

// ---------------------------------------------------------------------------
// row 13 — specific z byte patterns
// ---------------------------------------------------------------------------
fn row13_print_foo_z_patterns() {
    const ZS: [u32; 8] = [
        0x0000_0000,
        0x0000_0001,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0x0102_0304,
        0x8000_0001,
        0x00FF_00FF,
    ];
    let mut cases = Vec::new();
    for bits in [0x00u8, 0x1F, 0x20, 0x3F, 0xFF] {
        for z in ZS {
            cases.push(format!("print_foo(bits={bits:#04x}, z={z:#010x})"));
        }
    }
    assert_same("row13", &cases, |w| {
        let f = impls().print_foo(w);
        for bits in [0x00u8, 0x1F, 0x20, 0x3F, 0xFF] {
            for z in ZS {
                let zb = z.to_le_bytes();
                let buf = Aligned([
                    bits, 0, 0, 0, zb[0], zb[1], zb[2], zb[3], 0, 0, 0, 0, 0, 0, 0, 0,
                ]);
                unsafe { f(buf.0.as_ptr()) };
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 14 — composed pipeline: driver(x,y,b,z) == print_foo(byte image)
// ---------------------------------------------------------------------------
fn row14_driver_matches_print_foo_on_equivalent_image() {
    let mut rng = Rng::new(0x0000_0014_5EED);
    let mut args = Vec::new();
    for _ in 0..5000 {
        args.push((
            rng.interesting_u32(),
            rng.interesting_u32(),
            rng.next_u8(),
            rng.interesting_u32() as i32,
            rng.next_u8(),
        ));
    }
    let cases: Vec<String> = args
        .iter()
        .map(|(x, y, b, z, p)| format!("driver({x},{y},{b:#04x},{z}) vs image(pad={p:#04x})"))
        .collect();

    // Same-implementation cross-check first (C driver vs C print_foo), then the
    // differential comparison of both implementations' driver output.
    let _g = common::stdout_guard();
    let via_driver = common::capture(|| {
        let f = impls().driver(Which::C);
        for &(x, y, b, z, _) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
    let via_print = common::capture(|| {
        let f = impls().print_foo(Which::C);
        for &(x, y, b, z, p) in &args {
            let img = foo_bytes(x, y, b, z, p);
            unsafe { f(img.as_ptr()) };
        }
    });
    drop(_g);
    assert_eq!(
        String::from_utf8_lossy(&via_driver),
        String::from_utf8_lossy(&via_print),
        "C driver output must equal C print_foo output on the equivalent byte image"
    );

    assert_same("row14-driver", &cases, |w| {
        let f = impls().driver(w);
        for &(x, y, b, z, _) in &args {
            unsafe { f(x, y, b, z) };
        }
    });
    assert_same("row14-print_foo", &cases, |w| {
        let f = impls().print_foo(w);
        for &(x, y, b, z, p) in &args {
            let img = foo_bytes(x, y, b, z, p);
            unsafe { f(img.as_ptr()) };
        }
    });
}

// ---------------------------------------------------------------------------
// row 15 — long interleaved sequence of mixed calls in one process
// ---------------------------------------------------------------------------
fn row15_interleaved_sequence() {
    #[derive(Clone, Copy)]
    enum Op {
        Driver(u32, u32, u8, i32),
        PrintFoo([u8; 8]),
    }
    let mut rng = Rng::new(0x0000_0015_5EED);
    let mut ops = Vec::new();
    for _ in 0..10_000 {
        if rng.next_u8() & 1 == 0 {
            ops.push(Op::Driver(
                rng.interesting_u32(),
                rng.interesting_u32(),
                rng.next_u8(),
                rng.interesting_u32() as i32,
            ));
        } else {
            let mut b = [0u8; 8];
            for byte in b.iter_mut() {
                *byte = rng.next_u8();
            }
            ops.push(Op::PrintFoo(b));
        }
    }
    let cases: Vec<String> = ops
        .iter()
        .map(|op| match op {
            Op::Driver(x, y, b, z) => format!("driver({x},{y},{b:#04x},{z})"),
            Op::PrintFoo(img) => format!("print_foo({img:02x?})"),
        })
        .collect();
    assert_same("row15", &cases, |w| {
        let d = impls().driver(w);
        let p = impls().print_foo(w);
        for op in &ops {
            match *op {
                Op::Driver(x, y, b, z) => unsafe { d(x, y, b, z) },
                Op::PrintFoo(img) => {
                    let buf = Aligned([
                        img[0], img[1], img[2], img[3], img[4], img[5], img[6], img[7], 0, 0, 0,
                        0, 0, 0, 0, 0,
                    ]);
                    unsafe { p(buf.0.as_ptr()) }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 16 — repeated identical calls (no hidden accumulating state)
// ---------------------------------------------------------------------------
fn row16_repeated_identical_calls() {
    let cases: Vec<String> = (0..500).map(|i| format!("repeat #{i}")).collect();
    assert_same("row16", &cases, |w| {
        let d = impls().driver(w);
        let p = impls().print_foo(w);
        for _ in 0..250 {
            unsafe { d(3, 7, 1, -12345) };
            let buf = Aligned([
                0x3F, 0xAA, 0xBB, 0xCC, 0xC7, 0xCF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            unsafe { p(buf.0.as_ptr()) };
        }
    });

    // Also assert every repetition produced the identical line.
    let _g = common::stdout_guard();
    let out = common::capture(|| {
        let d = impls().driver(Which::Rust);
        for _ in 0..10 {
            unsafe { d(3, 7, 1, -12345) };
        }
    });
    drop(_g);
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 10);
    assert!(lines.iter().all(|l| *l == lines[0]), "output not idempotent: {lines:?}");
}

fn main() {
    common::run_tests(&[
        ("row01_driver_small_exhaustive_grid", row01_driver_small_exhaustive_grid),
        ("row02_driver_random_xy_full_range", row02_driver_random_xy_full_range),
        ("row03_driver_random_z", row03_driver_random_z),
        ("row04_driver_fully_random", row04_driver_fully_random),
        ("row05_driver_boundary_cross_product", row05_driver_boundary_cross_product),
        ("row06_driver_every_bool_byte", row06_driver_every_bool_byte),
        ("row07_driver_wide_bool_register", row07_driver_wide_bool_register),
        ("row08_print_foo_exhaustive_bitfield_byte", row08_print_foo_exhaustive_bitfield_byte),
        ("row09_print_foo_exhaustive_with_ff_padding", row09_print_foo_exhaustive_with_ff_padding),
        ("row10_print_foo_random_heap_buffers", row10_print_foo_random_heap_buffers),
        ("row11_print_foo_misaligned_pointers", row11_print_foo_misaligned_pointers),
        ("row12_print_foo_static_and_stack_storage", row12_print_foo_static_and_stack_storage),
        ("row13_print_foo_z_patterns", row13_print_foo_z_patterns),
        ("row14_driver_matches_print_foo_on_equivalent_image", row14_driver_matches_print_foo_on_equivalent_image),
        ("row15_interleaved_sequence", row15_interleaved_sequence),
        ("row16_repeated_identical_calls", row16_repeated_identical_calls),
    ]);
}
