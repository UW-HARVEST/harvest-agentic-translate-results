// Define the RHBloom struct
pub struct RHBloom {
    count: usize,
    nbuckets: usize,
    buckets: Vec<u64>,
    k: usize,
    m: usize,
    bits: Vec<u8>,
}

// Helpers matching the C macros
const KEY_MASK: u64 = (1u64 << 56) - 1;

#[inline]
fn rhbloom_key(x: u64) -> u64 {
    x & KEY_MASK
}

#[inline]
fn rhbloom_dib(x: u64) -> u64 {
    x >> 56
}

#[inline]
fn rhbloom_setkeydib(key: u64, dib: u64) -> u64 {
    rhbloom_key(key) | ((dib & 0xff) << 56)
}

// Implement the RHBloom struct
impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed
        // m = n * log(p) / log(1 / 2^log(2)), where log is natural log.
        let denom = (1.0_f64 / (2.0_f64).powf((2.0_f64).ln())).ln();
        let m_f = (n as f64) * p.ln() / denom;
        let m = m_f as usize;

        // Calculate the bits per key
        let k = ((m as f64 / n as f64) * (2.0_f64).ln()).round() as usize;

        // Adjust the number of bits to power of two
        let mut m0: usize = 2;
        while m0 < m {
            m0 *= 2;
        }
        let k0 = ((m as f64 / m0 as f64) * (k as f64)).round() as usize;

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
        let mut size = std::mem::size_of::<RHBloom>();
        size += if !self.bits.is_empty() {
            self.m >> 3
        } else {
            self.nbuckets << 3
        };
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
        // We only want the 56-bit key in order to match correctly with the
        // robinhood entries, upon upgrade.
        let mut key = rhbloom_key(key);

        // Add or check each bit
        let mut i: usize = 0;
        let mask = (self.m as u64).wrapping_sub(1);
        let mut j = (key & mask) as usize;
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
        // https://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html
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
            let bucket = self.buckets[i];
            if rhbloom_dib(bucket) == 0 {
                self.buckets[i] = rhbloom_setkeydib(key, dib);
                self.count += 1;
                return true;
            }
            if rhbloom_key(bucket) == key {
                return true;
            }
            if rhbloom_dib(bucket) < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = rhbloom_setkeydib(key, dib);
                key = rhbloom_key(tmp);
                dib = rhbloom_dib(tmp);
            }
            dib += 1;
            i = (i + 1) & (mask as usize);
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Inline read-only version of testadd
            let mut key = rhbloom_key(key);
            let mut i: usize = 0;
            let mask = (self.m as u64).wrapping_sub(1);
            let mut j = (key & mask) as usize;
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i == self.k - 1 {
                    break;
                }
                key = key.wrapping_mul(0x94d049bb133111eb);
                key ^= key >> 31;
                j = (key & mask) as usize;
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
            let bucket = self.buckets[i];
            let yes = rhbloom_key(bucket) == key;
            let no = rhbloom_dib(bucket) < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (mask as usize);
        }
    }
}
