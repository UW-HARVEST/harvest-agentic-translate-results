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

const KEY_MASK: u64 = (1u64 << 56) - 1;

#[inline]
fn rb_key(x: u64) -> u64 {
    x & KEY_MASK
}

#[inline]
fn rb_dib(x: u64) -> i32 {
    (x >> 56) as i32
}

#[inline]
fn rb_setkeydib(key: u64, dib: i32) -> u64 {
    rb_key(key) | ((dib as u64) << 56)
}

#[inline]
fn bytes_for_bits(m: usize) -> usize {
    (m + 7) / 8
}

// Implement the RHBloom struct
impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed
        // m = n * log(p) / log(1 / pow(2, log(2)))
        let denom = (1.0_f64 / (2.0_f64).powf((2.0_f64).ln())).ln();
        let m_f = (n as f64) * p.ln() / denom;
        let mut m = m_f as usize;
        if m < 8 {
            m = 8;
        }

        // Calculate the bits per key (optimal k for raw m)
        let k_f = ((m as f64) / (n as f64)) * (2.0_f64).ln();
        let mut k = k_f.round() as usize;
        if k == 0 {
            k = 1;
        }

        RHBloom {
            count: 0,
            nbuckets: 0,
            buckets: Vec::new(),
            k,
            m,
            bits: Vec::new(),
        }
    }

    pub fn memsize(&self) -> usize {
        let base = std::mem::size_of::<RHBloom>();
        if !self.bits.is_empty() {
            base + bytes_for_bits(self.m)
        } else {
            base + (self.nbuckets << 3)
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
        // We only want the 56-bit key in order to match correctly with the
        // robinhood entries, upon upgrade.
        let mut key = rb_key(key);

        let mut i: usize = 0;
        let m = self.m as u64;
        let mut j = (key % m) as usize;
        loop {
            if add {
                self.bits[j >> 3] |= 1u8 << (j & 7);
            } else if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                return false;
            }
            if i == self.k - 1 {
                break;
            }
            // Pick the next bit. Use part of the mix13 formula to help get a more
            // randomized value.
            key = key.wrapping_mul(0x94d049bb133111eb);
            key ^= key >> 31;
            j = (key % m) as usize;
            i += 1;
        }

        true
    }

    pub fn free(&mut self) {
        self.bits.clear();
        self.buckets.clear();
        self.count = 0;
        self.nbuckets = 0;
    }

    pub fn mix(mut key: u64) -> u64 {
        // hash u64 using mix13
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

        // Upgrade to bloom filter when bucket-array bytes >= bit-array bytes
        let bit_bytes = bytes_for_bits(self.m);
        if nbuckets_new * 8 >= bit_bytes {
            // Upgrade to bloom filter
            self.bits = vec![0u8; bit_bytes];
            self.count = 0;
            self.nbuckets = 0;
            // self.buckets is already empty (taken)
            for i in 0..nbuckets_old {
                if rb_dib(buckets_old[i]) != 0 {
                    self.testadd(buckets_old[i], true);
                }
            }
        } else {
            let buckets = vec![0u64; nbuckets_new];
            self.count = 0;
            self.nbuckets = nbuckets_new;
            self.buckets = buckets;
            for i in 0..nbuckets_old {
                if rb_dib(buckets_old[i]) != 0 {
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
        let mut key = rb_key(key);
        let mut dib: i32 = 1;
        let mut i = (key as usize) & (self.nbuckets - 1);
        loop {
            if rb_dib(self.buckets[i]) == 0 {
                self.buckets[i] = rb_setkeydib(key, dib);
                self.count += 1;
                return true;
            }
            if rb_key(self.buckets[i]) == key {
                return true;
            }
            if rb_dib(self.buckets[i]) < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = rb_setkeydib(key, dib);
                key = rb_key(tmp);
                dib = rb_dib(tmp);
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Inline read-only version of testadd for &self
            let mut k = rb_key(key);
            let mut i: usize = 0;
            let m = self.m as u64;
            let mut j = (k % m) as usize;
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i == self.k - 1 {
                    break;
                }
                k = k.wrapping_mul(0x94d049bb133111eb);
                k ^= k >> 31;
                j = (k % m) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = rb_key(key);
        let mut dib: i32 = 1;
        let mut i = (key as usize) & (self.nbuckets - 1);
        loop {
            let yes = rb_key(self.buckets[i]) == key;
            let no = rb_dib(self.buckets[i]) < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
}
