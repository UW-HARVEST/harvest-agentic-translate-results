//! Deterministic PRNG (xoshiro256**) so every randomized row is reproducible
//! from its seed.

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 to fill the state.
        let mut z = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut next = || {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.byte();
        }
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}
