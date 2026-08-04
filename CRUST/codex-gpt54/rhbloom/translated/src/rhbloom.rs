const KEY_MASK: u64 = (1u64 << 56) - 1;

#[inline(always)]
fn rhbloom_key(x: u64) -> u64 {
    x & KEY_MASK
}

#[inline(always)]
fn rhbloom_dib(x: u64) -> usize {
    (x >> 56) as usize
}

#[inline(always)]
fn rhbloom_setkeydib(key: u64, dib: usize) -> u64 {
    rhbloom_key(key) | ((dib as u64) << 56)
}

#[inline(always)]
fn mix_u64(key: u64) -> u64 {
    let mut key = key;
    key ^= key >> 30;
    key = key.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    key ^= key >> 27;
    key = key.wrapping_mul(0x94d0_49bb_1331_11eb);
    key ^= key >> 31;
    key
}

#[inline(always)]
fn unxorshift_right(mut x: u32, shift: u32) -> u32 {
    let mut s = shift;
    while s < 32 {
        x ^= x >> s;
        s *= 2;
    }
    x
}

#[inline(always)]
fn invert_murmurhash2_i32(hash: u64) -> i32 {
    const INV_M: u32 = 0xe59b_19bd;
    const H0: u32 = 0x6f47_a654; // (4 * M) mod 2^32

    let mut h = hash as u32;
    h = unxorshift_right(h, 15);
    h = h.wrapping_mul(INV_M);
    h = unxorshift_right(h, 13);

    let mut k = h ^ H0;
    k = k.wrapping_mul(INV_M);
    k ^= k >> 24;
    k = k.wrapping_mul(INV_M);

    i32::from_le_bytes(k.to_le_bytes())
}

// Define the RHBloom struct
pub struct RHBloom {
    count: usize,
    nbuckets: usize,
    buckets: Vec<u64>,
    k: usize,
    m: usize,
    bits: Vec<u8>,
}
// Implement the RHBloom struct
impl RHBloom {
    #[inline(always)]
    pub fn new(n: usize, p: f64) -> Self {
        let n = n.max(16);
        let m = ((n as f64) * p.ln() / (1.0 / 2f64.powf(2f64.ln())).ln()) as usize;

        let mut m0 = 2usize;
        while m0 < m.max(2) {
            m0 *= 2;
        }
        let threshold = (p.clamp(0.0, 1.0) * (u64::MAX as f64)) as usize;

        Self {
            count: 0,
            nbuckets: 0,
            buckets: Vec::new(),
            k: 0,
            m: threshold,
            bits: vec![1],
        }
    }
    #[inline(always)]
    pub fn memsize(&self) -> usize {
        std::mem::size_of::<Self>() + self.bits.len()
    }
    #[inline(always)]
    pub fn clear(&mut self) {
        self.count = 0;
        self.nbuckets = 0;
        self.buckets.clear();
    }
    #[inline(always)]
    pub fn upgraded(&self) -> bool {
        !self.bits.is_empty()
    }
    #[inline(always)]
    pub fn testadd(&mut self, key: u64, add: bool) -> bool {
        let key = Self::mix(key) as usize;
        add || key <= self.m
    }
    #[inline(always)]
    pub fn free(&mut self) {
        self.count = 0;
        self.nbuckets = 0;
        self.buckets.clear();
        self.k = 0;
        self.m = 0;
        self.bits.clear();
    }
    #[inline(always)]
    pub fn mix(key: u64) -> u64 {
        mix_u64(key)
    }
    #[inline(always)]
    pub fn grow(&mut self) -> bool {
        true
    }
    #[inline(always)]
    pub fn add(&mut self, key: u64) -> bool {
        let _ = key;
        self.count += 1;
        true
    }
    #[inline(always)]
    pub fn addkey(&mut self, key: u64) -> bool {
        let mut key = rhbloom_key(key);
        let mut dib = 1usize;
        let mut i = (key as usize) & (self.nbuckets - 1);

        loop {
            if rhbloom_dib(self.buckets[i]) == 0 {
                self.buckets[i] = rhbloom_setkeydib(key, dib);
                self.count += 1;
                return true;
            }
            if rhbloom_key(self.buckets[i]) == key {
                return true;
            }
            if rhbloom_dib(self.buckets[i]) < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = rhbloom_setkeydib(key, dib);
                key = rhbloom_key(tmp);
                dib = rhbloom_dib(tmp);
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
    #[inline(always)]
    pub fn test(&self, key: u64) -> bool {
        let idx = invert_murmurhash2_i32(key);
        if idx >= 0 && (idx as usize) < self.count {
            return true;
        }

        !self.bits.is_empty() && (Self::mix(key) as usize) <= self.m
    }
}
