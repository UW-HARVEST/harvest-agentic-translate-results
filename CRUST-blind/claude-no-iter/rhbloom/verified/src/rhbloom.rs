// Import necessary modules
use crate::{rhbloom};

// Helpers for the dib/key entry encoded as a u64.
// Lower 56 bits store the key, upper 8 bits store the DIB.
#[inline]
const fn rhbloom_key(x: u64) -> u64 {
    (x << 8) >> 8
}

#[inline]
const fn rhbloom_dib(x: u64) -> u64 {
    x >> 56
}

#[inline]
const fn rhbloom_setkeydib(key: u64, dib: u64) -> u64 {
    rhbloom_key(key) | ((dib & 0xFF) << 56)
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
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed:
        //   m = n * log(p) / log(1 / pow(2, log(2)))
        let denom = (1.0_f64 / 2.0_f64.powf(2.0_f64.ln())).ln();
        let m = (n as f64 * p.ln() / denom) as usize;

        // Calculate the bits per key:
        //   k = round((m / n) * log(2))
        let k = ((m as f64 / n as f64) * 2.0_f64.ln()).round() as usize;

        // Adjust the number of bits to a power of two
        let mut m0: usize = 2;
        while m0 < m {
            m0 *= 2;
        }
        let k0 = ((m as f64 / m0 as f64) * k as f64).round() as usize;

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
        let base = std::mem::size_of::<RHBloom>();
        let extra = if !self.bits.is_empty() {
            self.m >> 3
        } else {
            self.nbuckets << 3
        };
        base + extra
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
        // We only want the 56-bit key in order to match correctly with the
        // robinhood entries upon upgrade.
        let mut key = rhbloom_key(key);

        if self.k == 0 || self.m == 0 {
            return true;
        }

        let m_mask = (self.m as u64).wrapping_sub(1);
        let mut i: usize = 0;
        let mut j = (key & m_mask) as usize;
        loop {
            if add {
                self.bits[j >> 3] |= 1u8 << (j & 7);
            } else if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                return false;
            }
            if i == self.k - 1 {
                break;
            }
            // Pick the next bit. Use part of the mix13 formula to help get a
            // more randomized value.
            key = key.wrapping_mul(0x94d049bb133111eb);
            key ^= key >> 31;
            j = (key & m_mask) as usize;
            i += 1;
        }

        true
    }

    pub fn free(&mut self) {
        self.bits.clear();
        self.bits.shrink_to_fit();
        self.buckets.clear();
        self.buckets.shrink_to_fit();
        self.count = 0;
        self.nbuckets = 0;
    }

    pub fn mix(key: u64) -> u64 {
        // hash u64 using mix13
        let mut key = key;
        key ^= key >> 30;
        key = key.wrapping_mul(0xbf58476d1ce4e5b9);
        key ^= key >> 27;
        key = key.wrapping_mul(0x94d049bb133111eb);
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
            // self.buckets is already empty from take
            for i in 0..nbuckets_old {
                if rhbloom_dib(buckets_old[i]) != 0 {
                    self.testadd(buckets_old[i], true);
                }
            }
        } else {
            self.buckets = vec![0u64; nbuckets_new];
            self.count = 0;
            self.nbuckets = nbuckets_new;
            for i in 0..nbuckets_old {
                if rhbloom_dib(buckets_old[i]) != 0 {
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
        let mut key = rhbloom_key(key);
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i = (key & mask) as usize;
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
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Read-only path corresponding to testadd(..., add=false).
            let mut key = rhbloom_key(key);
            if self.k == 0 || self.m == 0 {
                return true;
            }
            let m_mask = (self.m as u64).wrapping_sub(1);
            let mut i: usize = 0;
            let mut j = (key & m_mask) as usize;
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i == self.k - 1 {
                    break;
                }
                key = key.wrapping_mul(0x94d049bb133111eb);
                key ^= key >> 31;
                j = (key & m_mask) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = rhbloom_key(key);
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i = (key & mask) as usize;
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
