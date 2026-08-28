//! Level 3: randomized differential fuzzing over the full input space
//! (arbitrary bytes, arbitrary lengths, arbitrary offsets).

mod common;

use common::Harness;

/// Small deterministic PRNG so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
}

const NUMERIC_ALPHABET: &[u8] = b"0123456789+-eE.";

#[test]
fn fuzz_numeric_alphabet() {
    let h = Harness::new();
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..40_000 {
        let len = rng.below(12);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(NUMERIC_ALPHABET[rng.below(NUMERIC_ALPHABET.len())]);
        }
        h.check(&v);
    }
}

#[test]
fn fuzz_arbitrary_bytes() {
    let h = Harness::new();
    let mut rng = Rng::new(0xDECAFBAD);
    for _ in 0..40_000 {
        let len = rng.below(20);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.next_u64() as u8);
        }
        h.check(&v);
    }
}

#[test]
fn fuzz_with_offsets_and_truncated_lengths() {
    let h = Harness::new();
    let mut rng = Rng::new(0x1234_5678_9ABC);
    for _ in 0..30_000 {
        let cap = 1 + rng.below(24);
        let mut v = Vec::with_capacity(cap);
        for _ in 0..cap {
            // Bias towards the numeric alphabet but keep some noise.
            if rng.below(4) == 0 {
                v.push(rng.next_u64() as u8);
            } else {
                v.push(NUMERIC_ALPHABET[rng.below(NUMERIC_ALPHABET.len())]);
            }
        }
        let length = rng.below(cap + 1);
        let offset = rng.below(cap + 1);
        let depth = rng.next_u64() as usize;
        h.check_raw(&v, length, offset, depth);
    }
}

#[test]
fn fuzz_realistic_json_fragments() {
    let h = Harness::new();
    let mut rng = Rng::new(0xFEED_FACE);
    let pieces: &[&str] = &[
        "0", "1", "-1", "12", "3.5", "-0.25", "1e5", "2E-3", "1.5e+2", "007", ".5", "5.",
        "2147483648", "-2147483649", "1e999", "1e-999", ",", "]", "}", " ", ":", "\"", "[", "{",
        "true", "null", "abc", "\n", "\t",
    ];
    for _ in 0..20_000 {
        let n = 1 + rng.below(5);
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(pieces[rng.below(pieces.len())]);
        }
        let bytes = s.as_bytes();
        h.check(bytes);
        if !bytes.is_empty() {
            h.check_raw(bytes, bytes.len(), rng.below(bytes.len()), 0);
        }
    }
}
