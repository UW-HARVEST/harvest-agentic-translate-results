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
// Implement the RHBloom struct
impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed
        // m = n * log(p) / log(1 / pow(2, log(2)))
        let denom = (1.0_f64 / (2.0_f64).powf((2.0_f64).ln())).ln();
        let m_f = (n as f64) * p.ln() / denom;
        let m: usize = m_f as usize;

        // Calculate the bits per key
        // k = round((m/n) * log(2))
        let k_f = ((m as f64) / (n as f64)) * (2.0_f64).ln();
        let k: usize = k_f.round() as usize;

        // Adjust the number of bits to power of two
        let mut m0: usize = 2;
        while m0 < m {
            m0 = m0.saturating_mul(2);
            if m0 == 0 {
                break;
            }
        }

        // k0 = round((m/m0) * k)
        let k0_f = (m as f64) / (m0 as f64) * (k as f64);
        let k0: usize = k0_f.round() as usize;

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
        let mut size = std::mem::size_of::<Self>();
        if !self.bits.is_empty() {
            size += self.m >> 3;
        } else {
            size += self.nbuckets << 3;
        }
        size
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
        // We only want the 56-bit key to match correctly with robinhood entries
        // upon upgrade.
        let mut key = (key << 8) >> 8;

        let mut i: usize = 0;
        let m_mask = (self.m as u64).wrapping_sub(1);
        let mut j: usize = (key & m_mask) as usize;
        loop {
            if add {
                self.bits[j >> 3] |= 1u8 << (j & 7);
            } else if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                return false;
            }
            if i == self.k.wrapping_sub(1) {
                break;
            }
            // Pick the next bit.
            key = key.wrapping_mul(0x94d049bb133111eb_u64);
            key ^= key >> 31;
            j = (key & m_mask) as usize;
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
        // mix13: https://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html
        let mut key = key;
        key ^= key >> 30;
        key = key.wrapping_mul(0xbf58476d1ce4e5b9_u64);
        key ^= key >> 27;
        key = key.wrapping_mul(0x94d049bb133111eb_u64);
        key ^= key >> 31;
        key
    }

    pub fn grow(&mut self) -> bool {
        let nbuckets_old = self.nbuckets;
        let buckets_old = std::mem::take(&mut self.buckets);
        let nbuckets_new = if nbuckets_old == 0 { 16 } else { nbuckets_old * 2 };

        if nbuckets_new * 8 >= self.m >> 3 {
            // Upgrade to bloom filter
            self.bits = vec![0u8; self.m >> 3];
            self.count = 0;
            self.nbuckets = 0;
            self.buckets = Vec::new();
            for i in 0..nbuckets_old {
                let entry = buckets_old[i];
                if (entry >> 56) != 0 {
                    self.testadd(entry, true);
                }
            }
        } else {
            self.count = 0;
            self.nbuckets = nbuckets_new;
            self.buckets = vec![0u64; nbuckets_new];
            for i in 0..nbuckets_old {
                let entry = buckets_old[i];
                if (entry >> 56) != 0 {
                    self.addkey(entry);
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
        let mut key = (key << 8) >> 8;
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i: usize = (key & mask) as usize;
        loop {
            let bucket = self.buckets[i];
            let bucket_dib = bucket >> 56;
            let bucket_key = (bucket << 8) >> 8;
            if bucket_dib == 0 {
                self.buckets[i] = key | (dib << 56);
                self.count += 1;
                return true;
            }
            if bucket_key == key {
                return true;
            }
            if bucket_dib < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = key | (dib << 56);
                key = (tmp << 8) >> 8;
                dib = tmp >> 56;
            }
            dib += 1;
            i = (i + 1) & (mask as usize);
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Inline testadd(key, false)
            let mut k = (key << 8) >> 8;
            let mut i: usize = 0;
            let m_mask = (self.m as u64).wrapping_sub(1);
            let mut j: usize = (k & m_mask) as usize;
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i == self.k.wrapping_sub(1) {
                    break;
                }
                k = k.wrapping_mul(0x94d049bb133111eb_u64);
                k ^= k >> 31;
                j = (k & m_mask) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = (key << 8) >> 8;
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i: usize = (key & mask) as usize;
        loop {
            let bucket = self.buckets[i];
            let bucket_dib = bucket >> 56;
            let bucket_key = (bucket << 8) >> 8;
            let yes = bucket_key == key;
            let no = bucket_dib < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (mask as usize);
        }
    }
}
