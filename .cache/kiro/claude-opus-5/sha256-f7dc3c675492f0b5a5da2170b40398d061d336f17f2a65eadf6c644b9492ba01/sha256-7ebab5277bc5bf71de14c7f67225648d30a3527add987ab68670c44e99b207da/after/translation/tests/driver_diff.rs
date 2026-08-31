//! Differential tests of the C implementation in `c_src/src/driver.c` against
//! the Rust translation, both reached only through their `.so` exports.
//!
//! Call hierarchy in the C source (from `include/driver.h` plus the
//! implementation file):
//!
//! * `static void print_hex(unsigned char *p, int len)` — lowest level; it has
//!   internal linkage and is therefore not an export of either library, so it is
//!   exercised indirectly, through every `driver` call. The `%02x` formatting,
//!   the byte ordering, and the single trailing newline it emits are all
//!   observable in the captured bytes.
//! * `void driver(float x)` — the only public entry point: it copies the object
//!   representation of `x` into a `char[4]` and hands it to `print_hex`.
//!
//! Every assertion compares raw bytes, so any difference in digit case, field
//! width, separator, or newline placement fails.

mod common;

use common::{describe, load_pair, run_driver, Pair};
use libloading::{Library, Symbol};

/// One `driver` call per capture: the strictest form of the comparison.
fn assert_same(p: &Pair, x: f32) {
    let c_out = run_driver(&p.c, x);
    let rust_out = run_driver(&p.rust, x);
    assert_eq!(
        c_out,
        rust_out,
        "driver({}) mismatch:\n  C    = {:?}\n  Rust = {:?}",
        describe(x),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

/// Many `driver` calls inside a single capture. This keeps large sweeps fast and
/// additionally pins down cross-call behaviour (buffering, no extra separators).
fn assert_same_batch(p: &Pair, values: &[f32]) {
    let c_out = run_batch(&p.c, values);
    let rust_out = run_batch(&p.rust, values);

    if c_out != rust_out {
        // Output is one line per call, so a line-wise diff names the culprit.
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "batch mismatch at index {} ({}):\n  C    = {:?}\n  Rust = {:?}",
                    i,
                    values.get(i).map(|&v| describe(v)).unwrap_or_default(),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl)
                );
            }
        }
        panic!(
            "batch mismatch in overall length: C {} bytes vs Rust {} bytes",
            c_out.len(),
            rust_out.len()
        );
    }
}

fn run_batch(lib: &Library, values: &[f32]) -> Vec<u8> {
    let sym: Symbol<common::DriverFn> = unsafe { lib.get(b"driver\0").expect("`driver` symbol") };
    common::capture_stdout(|| {
        for &v in values {
            unsafe { sym(v) };
        }
    })
}

/// Deterministic 64-bit xorshift, so failures are reproducible.
struct Rng(u64);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
}

#[test]
fn both_libraries_export_driver() {
    let p = load_pair();
    unsafe {
        p.c.get::<common::DriverFn>(b"driver\0")
            .expect("C library must export `driver`");
        p.rust
            .get::<common::DriverFn>(b"driver\0")
            .expect("Rust library must export `driver`");
    }
}

/// Sanity check on the harness itself: the C output must be the four bytes of
/// the float in native order, lowercase hex, followed by exactly one newline.
#[test]
fn c_output_shape_is_as_expected() {
    let p = load_pair();
    let out = run_driver(&p.c, 1.0f32);
    assert_eq!(
        out,
        b"0000803f\n".to_vec(),
        "unexpected C output {:?} (native byte order assumption)",
        String::from_utf8_lossy(&out)
    );
    assert_eq!(run_driver(&p.rust, 1.0f32), out);
}

#[test]
fn representative_values() {
    let p = load_pair();
    let values: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        3.14159265,
        -3.14159265,
        1e-30,
        -1e-30,
        1e30,
        -1e30,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x0000_0001), // smallest positive subnormal
        f32::from_bits(0x8000_0001), // smallest negative subnormal
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xffbf_ffff), // negative signalling NaN
        f32::from_bits(0x7fc0_0000), // canonical quiet NaN
        f32::from_bits(0xffff_ffff),
        f32::from_bits(0x0000_0000),
        f32::from_bits(0xdead_beef),
        f32::from_bits(0x1234_5678),
        f32::from_bits(0x0a0b_0c0d),
    ];
    for v in values {
        assert_same(&p, v);
    }
}

/// Every single-bit pattern and its complement: covers each byte lane and both
/// nibbles of every byte, which is where a zero-padding or width bug would show.
#[test]
fn single_bit_patterns() {
    let p = load_pair();
    for bit in 0..32u32 {
        assert_same(&p, f32::from_bits(1u32 << bit));
        assert_same(&p, f32::from_bits(!(1u32 << bit)));
    }
}

/// All 256 byte values placed in each of the four byte positions, to confirm the
/// `%02x` conversion matches for every possible byte (notably 0x00..0x0f, which
/// must be zero-padded, and 0x80..0xff, which must not be sign-extended).
#[test]
fn all_byte_values_in_every_position() {
    let p = load_pair();
    let mut values = Vec::with_capacity(4 * 256);
    for pos in 0..4u32 {
        for byte in 0..256u32 {
            values.push(f32::from_bits(byte << (8 * pos)));
            values.push(f32::from_bits((byte << (8 * pos)) | 0x5a5a_5a5a & !(0xffu32 << (8 * pos))));
        }
    }
    assert_same_batch(&p, &values);
}

/// A strided walk across the whole 2^32 bit-pattern space (65536 samples, prime
/// stride so exponent and mantissa fields vary independently).
#[test]
fn strided_sweep_of_bit_space() {
    let p = load_pair();
    let values: Vec<f32> = (0..65_536u64)
        .map(|i| f32::from_bits(((i * 65_537) & 0xffff_ffff) as u32))
        .collect();
    assert_same_batch(&p, &values);
}

/// Exhaustive over the low 16 bits with several fixed high halves, plus
/// exhaustive over the high 16 bits with a fixed low half.
#[test]
fn exhaustive_half_word_sweeps() {
    let p = load_pair();
    for &high in &[0x0000u32, 0x3f80, 0x7f80, 0xffc0, 0x8000] {
        let values: Vec<f32> = (0..65_536u32)
            .map(|low| f32::from_bits((high << 16) | low))
            .collect();
        assert_same_batch(&p, &values);
    }
    let values: Vec<f32> = (0..65_536u32)
        .map(|high| f32::from_bits((high << 16) | 0xbeef))
        .collect();
    assert_same_batch(&p, &values);
}

#[test]
fn random_bit_patterns() {
    let p = load_pair();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let values: Vec<f32> = (0..200_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    for chunk in values.chunks(10_000) {
        assert_same_batch(&p, chunk);
    }
}

/// Repeated calls with the same argument must produce identical repeated lines
/// on both sides (no state carried between calls).
#[test]
fn repeated_calls_are_stateless() {
    let p = load_pair();
    let values = vec![-7.25f32; 64];
    assert_same_batch(&p, &values);

    let interleaved: Vec<f32> = (0..64)
        .map(|i| if i % 2 == 0 { f32::INFINITY } else { -0.0 })
        .collect();
    assert_same_batch(&p, &interleaved);
}
