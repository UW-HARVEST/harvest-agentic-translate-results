// Import necessary modules
use std::mem;

const KEY_MASK: u64 = (1u64 << 56) - 1;

fn rhbloom_key(x: u64) -> u64 {
    x & KEY_MASK
}

fn rhbloom_dib(x: u64) -> i32 {
    (x >> 56) as i32
}

fn rhbloom_setkeydib(key: u64, dib: i32) -> u64 {
    rhbloom_key(key) | ((dib as u64) << 56)
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
    pub fn new(n: usize, p: f64) -> Self {
        let n = n.max(16);
        let m = (n as f64 * p.ln() / (1.0 / 2f64.powf(2f64.ln())).ln()) as usize;
        let k = (((m as f64) / (n as f64)) * 2f64.ln()).round() as usize;

        let mut m0 = 2usize;
        while m0 < m {
            m0 *= 2;
        }
        let k0 = ((m as f64) / (m0 as f64) * (k as f64)).round() as usize;

        Self {
            count: 0,
            nbuckets: 0,
            buckets: Vec::new(),
            k: k0,
            m: m0,
            bits: Vec::new(),
        }
    }
    pub fn memsize(&self) -> usize {
        std::mem::size_of::<Self>()
            + if self.upgraded() {
                self.m >> 3
            } else {
                self.nbuckets << 3
            }
    }
    pub fn clear(&mut self) {
        if self.upgraded() {
            self.bits.fill(0);
        } else if !self.buckets.is_empty() {
            self.buckets.fill(0);
            self.count = 0;
        }
    }
    pub fn upgraded(&self) -> bool {
        !self.bits.is_empty()
    }
    pub fn testadd(&mut self, key: u64, add: bool) -> bool {
        let mut key = rhbloom_key(key);
        let mut i = 0usize;
        let mut j = (key as usize) & (self.m - 1);

        loop {
            let byte_index = j >> 3;
            let bit = 1u8 << (j & 7);
            if add {
                self.bits[byte_index] |= bit;
            } else if self.bits[byte_index] & bit == 0 {
                return false;
            }
            if i == self.k - 1 {
                break;
            }
            key = key.wrapping_mul(0x94d0_49bb_1331_11eb);
            key ^= key >> 31;
            j = (key as usize) & (self.m - 1);
            i += 1;
        }

        true
    }
    pub fn free(&mut self) {
        self.count = 0;
        self.nbuckets = 0;
        self.buckets = Vec::new();
        self.bits = Vec::new();
    }
    pub fn mix(key: u64) -> u64 {
        let mut key = key;
        key ^= key >> 30;
        key = key.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        key ^= key >> 27;
        key = key.wrapping_mul(0x94d0_49bb_1331_11eb);
        key ^= key >> 31;
        key
    }
    pub fn grow(&mut self) -> bool {
        let nbuckets_old = self.nbuckets;
        let buckets_old = mem::take(&mut self.buckets);
        let nbuckets_new = if nbuckets_old == 0 {
            16
        } else {
            nbuckets_old * 2
        };

        if nbuckets_new * 8 >= self.m >> 3 {
            let bit_bytes = self.m >> 3;
            let mut bits = Vec::new();
            if bits.try_reserve_exact(bit_bytes).is_err() {
                self.buckets = buckets_old;
                return false;
            }
            bits.resize(bit_bytes, 0);

            self.bits = bits;
            self.count = 0;
            self.nbuckets = 0;
            for bucket in buckets_old {
                if rhbloom_dib(bucket) != 0 {
                    self.testadd(bucket, true);
                }
            }
        } else {
            let mut buckets = Vec::new();
            if buckets.try_reserve_exact(nbuckets_new).is_err() {
                self.buckets = buckets_old;
                return false;
            }
            buckets.resize(nbuckets_new, 0);

            self.count = 0;
            self.nbuckets = nbuckets_new;
            self.buckets = buckets;
            for bucket in buckets_old {
                if rhbloom_dib(bucket) != 0 {
                    self.addkey(bucket);
                }
            }
        }

        true
    }
    pub fn add(&mut self, key: u64) -> bool {
        let key = Self::mix(key);
        loop {
            if self.upgraded() {
                self.testadd(key, true);
                return true;
            }
            if self.count == (self.nbuckets >> 1) {
                if !self.grow() {
                    return false;
                }
                continue;
            }
            break;
        }
        self.addkey(key)
    }
    pub fn addkey(&mut self, key: u64) -> bool {
        let mut key = rhbloom_key(key);
        let mut dib = 1i32;
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
    pub fn test(&self, key: u64) -> bool {
        let mut key = Self::mix(key);
        if self.upgraded() {
            key = rhbloom_key(key);
            let mut i = 0usize;
            let mut j = (key as usize) & (self.m - 1);
            loop {
                let byte_index = j >> 3;
                let bit = 1u8 << (j & 7);
                if self.bits[byte_index] & bit == 0 {
                    return false;
                }
                if i == self.k - 1 {
                    break;
                }
                key = key.wrapping_mul(0x94d0_49bb_1331_11eb);
                key ^= key >> 31;
                j = (key as usize) & (self.m - 1);
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }

        key = rhbloom_key(key);
        let mut dib = 1i32;
        let mut i = (key as usize) & (self.nbuckets - 1);
        loop {
            let yes = rhbloom_key(self.buckets[i]) == key;
            let no = rhbloom_dib(self.buckets[i]) < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
}
