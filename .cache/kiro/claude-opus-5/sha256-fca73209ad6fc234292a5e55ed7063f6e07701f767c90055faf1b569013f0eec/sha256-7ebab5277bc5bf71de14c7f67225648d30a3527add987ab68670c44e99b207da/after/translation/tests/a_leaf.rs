//! Level 1: leaf functions with no dependency on the module-level node state.
mod harness;

use harness::impls;
use std::ffi::{c_char, c_double, c_int};

/// Interesting doubles: exact int bounds, just inside/outside them, subnormals,
/// negative zero, infinities and both NaN signs.
fn double_cases() -> Vec<c_double> {
    let mut v: Vec<c_double> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        // exact int bounds and their neighbourhood
        c_int::MAX as c_double,
        c_int::MAX as c_double - 1.0,
        c_int::MAX as c_double - 0.5,
        c_int::MAX as c_double + 0.5,
        c_int::MAX as c_double + 1.0,
        c_int::MAX as c_double + 2.0,
        c_int::MIN as c_double,
        c_int::MIN as c_double + 1.0,
        c_int::MIN as c_double + 0.5,
        c_int::MIN as c_double - 0.5,
        c_int::MIN as c_double - 1.0,
        c_int::MIN as c_double - 2.0,
        // far outside
        1e18,
        -1e18,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        // tiny
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::EPSILON,
        // non-finite
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0abc), // quiet NaN, payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001),
    ];
    // the six node values used by maxnmin, and their subtree sums
    v.extend_from_slice(&[10.5, 20.7, 15.3, 5.9, 8.2, 12.4, 73.0, 34.8, 27.7]);
    // a deterministic spread of magnitudes
    let mut x: u64 = 0x1234_5678_9abc_def0;
    for _ in 0..400 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mant = ((x >> 11) as f64) / ((1u64 << 53) as f64); // [0,1)
        let exp = ((x & 0x3f) as i32) - 32; // 2^-32 .. 2^31
        let sign = if x & 0x40 != 0 { -1.0 } else { 1.0 };
        v.push(sign * mant * (2.0f64).powi(exp) * 3.0e9);
    }
    v
}

#[test]
fn safe_double_to_int_matches() {
    let i = impls();
    for d in double_cases() {
        let expected = unsafe { (i.c.safe_double_to_int)(d) };
        for r in &i.rust {
            let got = unsafe { (r.safe_double_to_int)(d) };
            assert_eq!(
                expected, got,
                "safe_double_to_int({d:?} bits={:#018x}) C={expected} {}={got}",
                d.to_bits(),
                r.label
            );
        }
    }
}

fn string_cases() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"root".to_vec(),
        b"child1".to_vec(),
        b"grandchild1".to_vec(),
        b"Hello, World!".to_vec(),
        b" \t\n\r\x0b\x0c".to_vec(),
        b"0123456789".to_vec(),
        vec![0x01],
        vec![0x7f],
        vec![0x80],          // sign-extends to a negative int on x86-64
        vec![0xff],          // -> -1
        vec![0x80, 0x80],    // -> -256
        vec![0xff; 32],
        vec![0x80; 64],
        (1u8..=255).collect(), // every non-NUL byte value
        (1u8..=255).rev().collect(),
        vec![b'x'; 49],
        vec![b'x'; 50],
        vec![b'x'; 200],
        "héllo wörld ✓".as_bytes().to_vec(),
    ];
    // deterministic random byte strings (never containing an interior NUL)
    let mut x: u64 = 0xdead_beef_cafe_1234;
    for len in 0..64usize {
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            let b = ((x >> 33) & 0xff) as u8;
            s.push(if b == 0 { 1 } else { b });
        }
        v.push(s);
    }
    v
}

#[test]
fn process_string_matches() {
    let i = impls();
    for s in string_cases() {
        // separate NUL-terminated buffers per call: the C signature is
        // `char *`, so nothing is shared between implementations.
        let mut cbuf: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
        cbuf.push(0);
        let expected = unsafe { (i.c.process_string)(cbuf.as_mut_ptr()) };

        for r in &i.rust {
            let mut rbuf: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
            rbuf.push(0);
            let got = unsafe { (r.process_string)(rbuf.as_mut_ptr()) };
            assert_eq!(
                expected,
                got,
                "process_string({:?}) C={expected} {}={got}",
                String::from_utf8_lossy(&s),
                r.label
            );
        }

        // the buffer must be left untouched by either implementation
        let mut orig: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
        orig.push(0);
        assert_eq!(cbuf, orig, "C mutated its input buffer");
    }
}

/// An interior NUL terminates the walk; bytes past it are ignored.
#[test]
fn process_string_stops_at_interior_nul() {
    let i = impls();
    let raw: [c_char; 8] = [b'a' as c_char, b'b' as c_char, 0, 127, 127, 127, 127, 0];
    let mut cbuf = raw;
    let expected = unsafe { (i.c.process_string)(cbuf.as_mut_ptr()) };
    assert_eq!(expected, (b'a' + b'b') as c_int);
    for r in &i.rust {
        let mut rbuf = raw;
        let got = unsafe { (r.process_string)(rbuf.as_mut_ptr()) };
        assert_eq!(expected, got, "{}", r.label);
    }
}
