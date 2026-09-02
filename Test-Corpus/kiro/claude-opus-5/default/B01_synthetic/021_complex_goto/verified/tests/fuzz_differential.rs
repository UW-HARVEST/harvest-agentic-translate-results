//! Randomised differential fuzzing over the same two subprocesses.
//!
//! Deterministic (fixed seed, hand-rolled LCG, no dev-dependencies) so a
//! failure is always reproducible. Two generators are used:
//!
//! 1. structured `"<x><sep><y>"` inputs drawn from the interesting integer
//!    boundaries, and
//! 2. free-form byte soup from the alphabet `%d` actually branches on, which
//!    lands on the `scanf` failure paths far more often than a shaped input.
//!
//! Because a random input can hit the class where the C program never
//! terminates, comparisons go through `assert_compatible`, which compares
//! everything observable up to a stdout cap and the exit status too whenever
//! the C program finished inside that cap.

mod harness;

use harness::assert_compatible;

const CAP: usize = 1 << 20;

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes' 64-bit LCG constants.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }

    fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u32) as i64
    }
}

#[test]
fn fuzz_structured_pairs() {
    let mut rng = Lcg(0x5eed_1234_9abc_def0);
    let boundaries: [i64; 12] = [
        0,
        1,
        2,
        3,
        4,
        -1,
        2147483647,
        -2147483648,
        2147483648,
        4294967296,
        4294967295,
        -4294967295,
    ];
    let seps: [&[u8]; 8] = [
        b" ", b"\n", b"\t", b"  \n\t ", b"\r\n", b"\x0b", b"\x0c", b"    ",
    ];
    let prefixes: [&[u8]; 4] = [b"", b" ", b"\n\n", b"\t"];
    let suffixes: [&[u8]; 5] = [b"", b"\n", b" junk", b"\x00", b"  "];

    for i in 0..400 {
        let x = match rng.below(4) {
            0 => *rng.pick(&boundaries),
            1 => rng.in_range(-4, 4),
            _ => rng.in_range(-10, 60),
        };
        let y = match rng.below(4) {
            0 => *rng.pick(&boundaries),
            1 => rng.in_range(-4, 4),
            _ => rng.in_range(-10, 60),
        };

        let mut input = Vec::new();
        input.extend_from_slice(rng.pick(&prefixes));
        input.extend_from_slice(x.to_string().as_bytes());
        input.extend_from_slice(rng.pick(&seps));
        input.extend_from_slice(y.to_string().as_bytes());
        input.extend_from_slice(rng.pick(&suffixes));

        assert_compatible(&format!("fuzz-pair #{i} x={x} y={y}"), &input, CAP);
    }
}

#[test]
fn fuzz_free_form_bytes() {
    let mut rng = Lcg(0xdead_beef_cafe_0001);
    // Every byte class the C program's input handling distinguishes: digits,
    // signs, the whitespace set, non-numeric bytes, NUL and a high byte.
    let alphabet: &[u8] = b"0123456789012 \t\n\r\x0b\x0c+-abcXx.,\x00\xff";

    for i in 0..500 {
        let len = rng.below(11) as usize;
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(alphabet)).collect();
        assert_compatible(&format!("fuzz-bytes #{i}"), &input, CAP);
    }
}

#[test]
fn fuzz_long_digit_runs() {
    // Long digit strings drive the overflow / saturation / truncation path.
    let mut rng = Lcg(0x0123_4567_89ab_cdef);
    for i in 0..80 {
        let len = 1 + rng.below(28) as usize;
        let mut input = Vec::new();
        if rng.below(2) == 0 {
            input.push(if rng.below(2) == 0 { b'-' } else { b'+' });
        }
        for j in 0..len {
            // Avoid a value that merely happens to be small; keep the leading
            // digit non-zero half the time.
            let d = if j == 0 && rng.below(2) == 0 {
                b'1' + rng.below(9) as u8
            } else {
                b'0' + rng.below(10) as u8
            };
            input.push(d);
        }
        input.extend_from_slice(b" 0");
        assert_compatible(&format!("fuzz-digits #{i}"), &input, CAP);
    }
}
