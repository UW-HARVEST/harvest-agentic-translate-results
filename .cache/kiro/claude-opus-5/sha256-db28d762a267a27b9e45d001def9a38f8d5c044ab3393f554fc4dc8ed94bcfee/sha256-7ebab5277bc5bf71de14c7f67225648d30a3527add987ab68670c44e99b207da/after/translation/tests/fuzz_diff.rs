//! Randomised differential sweep over `tool_basename`.
//!
//! Uses a deterministic xorshift generator so failures are reproducible without
//! pulling in an RNG dependency.

mod common;

use common::assert_same;

struct Rng(u64);

impl Rng {
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
        (self.next_u64() % n as u64) as usize
    }
}

#[test]
fn random_separator_heavy_strings() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    let alphabet = [b'/', b'\\', b'a', b'.', b':', b' ', 0x80, 0xFF];

    for _ in 0..20_000 {
        let len = rng.below(24);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(alphabet[rng.below(alphabet.len())]);
        }
        assert_same(&v);
    }
}

#[test]
fn random_arbitrary_bytes() {
    let mut rng = Rng(0x0FED_CBA9_8765_4321);

    for _ in 0..10_000 {
        let len = rng.below(64);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            // Any non-NUL byte: interior NULs would terminate the C string.
            v.push((rng.below(255) + 1) as u8);
        }
        assert_same(&v);
    }
}
