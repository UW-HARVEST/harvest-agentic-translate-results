// Import necessary modules
use crate::{rhbloom};
// Define the RHBloom struct
pub struct RHBloom {
    count: usize,
    nbuckets: usize,
    buckets: Vec<u64>,
    k: usize,
    m: usize,
    bits: Vec<u8>,
}

const KEY_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF; // lower 56 bits

#[inline]
fn rhb_key(x: u64) -> u64 {
    x & KEY_MASK
}

#[inline]
fn rhb_dib(x: u64) -> u64 {
    x >> 56
}

#[inline]
fn rhb_setkeydib(key: u64, dib: u64) -> u64 {
    rhb_key(key) | ((dib & 0xff) << 56)
}

// Implement the RHBloom struct
impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed
        // m = n * log(p) / log(1 / pow(2, log(2)))
        let denom = (1.0_f64 / 2.0_f64.powf(2.0_f64.ln())).ln();
        let m_f = (n as f64) * p.ln() / denom;
        let m: usize = if m_f.is_finite() && m_f > 0.0 { m_f as usize } else { 0 };

        // Calculate the bits per key: k = round(m/n * log(2))
        let k_f = (m as f64 / n as f64) * 2.0_f64.ln();
        let k: usize = if k_f.is_finite() && k_f > 0.0 { k_f.round() as usize } else { 0 };

        // Adjust the number of bits to power of two
        let mut m0: usize = 2;
        while m0 < m {
            m0 = m0.saturating_mul(2);
            if m0 == 0 {
                break;
            }
        }

        // k0 = round(m / m0 * k)
        let k0_f = (m as f64) / (m0 as f64) * (k as f64);
        let k0: usize = if k0_f.is_finite() && k0_f > 0.0 { k0_f.round() as usize } else { 0 };

        RHBloom {
            count: 0,
            nbuckets: 0,
            buckets: Vec::new(),
            k: k0,
            m: m0,
            bits: Vec::new(),
        }
    }

    pub fn memsize(&self) -> usize {
        // Approximate the C struct size with the size of this struct
        let size = std::mem::size_of::<Self>();
        size + if !self.bits.is_empty() {
            self.m >> 3
        } else {
            self.nbuckets << 3
        }
    }

    pub fn clear(&mut self) {
        if !self.bits.is_empty() {
            for b in self.bits.iter_mut() {
                *b = 0;
            }
        } else if !self.buckets.is_empty() {
            for b in self.buckets.iter_mut() {
                *b = 0;
            }
            self.count = 0;
        }
    }

    pub fn upgraded(&self) -> bool {
        !self.bits.is_empty()
    }

    pub fn testadd(&mut self, key: u64, add: bool) -> bool {
        // Only the 56-bit key
        let mut key = rhb_key(key);

        if self.m == 0 || self.k == 0 {
            return true;
        }

        let mask = (self.m - 1) as u64;
        let mut i: usize = 0;
        let mut j: usize = (key & mask) as usize;
        loop {
            if add {
                let byte_idx = j >> 3;
                if byte_idx < self.bits.len() {
                    self.bits[byte_idx] |= 1u8 << (j & 7);
                }
            } else {
                let byte_idx = j >> 3;
                if byte_idx >= self.bits.len() {
                    return false;
                }
                if (self.bits[byte_idx] >> (j & 7)) & 1 == 0 {
                    return false;
                }
            }
            if i + 1 >= self.k {
                break;
            }
            // Pick the next bit using part of the mix13 formula
            key = key.wrapping_mul(0x94d0_49bb_1331_11eb);
            key ^= key >> 31;
            j = (key & mask) as usize;
            i += 1;
        }

        true
    }

    pub fn free(&mut self) {
        self.bits = Vec::new();
        self.buckets = Vec::new();
        self.count = 0;
        self.nbuckets = 0;
    }

    pub fn mix(key: u64) -> u64 {
        // mix13
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
        let buckets_old = std::mem::take(&mut self.buckets);
        let nbuckets_new = if nbuckets_old == 0 { 16 } else { nbuckets_old * 2 };

        if nbuckets_new * 8 >= self.m >> 3 {
            // Upgrade to bloom filter
            let bits_len = self.m >> 3;
            // ensure non-empty so upgraded() works correctly
            let bits_len = if bits_len == 0 { 1 } else { bits_len };
            self.bits = vec![0u8; bits_len];
            self.count = 0;
            self.nbuckets = 0;
            // self.buckets already taken (empty)
            for i in 0..nbuckets_old {
                if rhb_dib(buckets_old[i]) != 0 {
                    self.testadd(buckets_old[i], true);
                }
            }
        } else {
            self.count = 0;
            self.nbuckets = nbuckets_new;
            self.buckets = vec![0u64; nbuckets_new];
            for i in 0..nbuckets_old {
                if rhb_dib(buckets_old[i]) != 0 {
                    self.addkey(buckets_old[i]);
                }
            }
        }
        true
    }

    pub fn add(&mut self, key: u64) -> bool {
        let key = Self::mix(key);
        loop {
            if !self.bits.is_empty() {
                self.testadd(key, true);
                return true;
            }
            if self.count == self.nbuckets >> 1 {
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
        let mut key = rhb_key(key);
        if self.nbuckets == 0 {
            return false;
        }
        let mask = (self.nbuckets - 1) as u64;
        let mut dib: u64 = 1;
        let mut i: usize = (key & mask) as usize;
        loop {
            if rhb_dib(self.buckets[i]) == 0 {
                self.buckets[i] = rhb_setkeydib(key, dib);
                self.count += 1;
                return true;
            }
            if rhb_key(self.buckets[i]) == key {
                return true;
            }
            if rhb_dib(self.buckets[i]) < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = rhb_setkeydib(key, dib);
                key = rhb_key(tmp);
                dib = rhb_dib(tmp);
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Inline testadd check (we can't call &mut testadd from &self)
            let mut k = rhb_key(key);
            if self.m == 0 || self.k == 0 {
                return true;
            }
            let mask = (self.m - 1) as u64;
            let mut i: usize = 0;
            let mut j: usize = (k & mask) as usize;
            loop {
                let byte_idx = j >> 3;
                if byte_idx >= self.bits.len() {
                    return false;
                }
                if (self.bits[byte_idx] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i + 1 >= self.k {
                    break;
                }
                k = k.wrapping_mul(0x94d0_49bb_1331_11eb);
                k ^= k >> 31;
                j = (k & mask) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = rhb_key(key);
        let mask = (self.nbuckets - 1) as u64;
        let mut dib: u64 = 1;
        let mut i: usize = (key & mask) as usize;
        loop {
            let yes = rhb_key(self.buckets[i]) == key;
            let no = rhb_dib(self.buckets[i]) < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
}
