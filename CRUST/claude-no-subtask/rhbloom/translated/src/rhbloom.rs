// Robin hood bloom filter — Rust port of https://github.com/tidwall/rhbloom

// Define the RHBloom struct
pub struct RHBloom {
    count: usize,
    nbuckets: usize,
    buckets: Vec<u64>,
    k: usize,
    m: usize,
    bits: Vec<u8>,
}

// Mask for the lower 56 bits of a u64 (the key portion in a bucket entry).
const KEY_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

#[inline(always)]
fn entry_dib(entry: u64) -> u64 {
    entry >> 56
}

#[inline(always)]
fn entry_key(entry: u64) -> u64 {
    entry & KEY_MASK
}

#[inline(always)]
fn make_entry(key: u64, dib: u64) -> u64 {
    (key & KEY_MASK) | (dib << 56)
}

impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // m = -n * ln(p) / ln(2)^2
        let ln2 = 2.0_f64.ln();
        let denom = (1.0_f64 / 2.0_f64.powf(ln2)).ln();
        let m_f = (n as f64) * p.ln() / denom;
        let m_calc: usize = if m_f.is_finite() && m_f > 0.0 {
            m_f as usize
        } else {
            8
        };

        // k = round((m/n) * ln(2))
        let k_f = (m_calc as f64 / n as f64) * ln2;
        let mut k: usize = k_f.round() as usize;
        if k == 0 {
            k = 1;
        }

        // Round m up to a multiple of 8 (so bit array is a whole number of bytes)
        // and ensure at least 8 bits.
        let mut m = ((m_calc + 7) / 8) * 8;
        if m < 8 {
            m = 8;
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
        // We only want the 56-bit key in order to match correctly with the
        // robin hood entries, upon upgrade.
        let mut key = key & KEY_MASK;
        // Lemire's fast range reduction: ((key * m) >> 56) lies in [0, m)
        // for any 56-bit key, with low bias for our purposes.
        let m = self.m as u128;
        let k = self.k;
        let bits = self.bits.as_mut_slice();
        let mut i: usize = 0;
        let mut j = (((key as u128) * m) >> 56) as usize;
        loop {
            let byte_idx = j >> 3;
            let bit_idx = (j & 7) as u32;
            if add {
                bits[byte_idx] |= 1u8 << bit_idx;
            } else if (bits[byte_idx] >> bit_idx) & 1 == 0 {
                return false;
            }
            if i + 1 == k {
                break;
            }
            // Pick the next bit. Use part of the mix13 formula to help get a
            // more randomized value.
            key = key.wrapping_mul(0x94d049bb133111eb);
            key ^= key >> 31;
            key &= KEY_MASK;
            j = (((key as u128) * m) >> 56) as usize;
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
            let nbits_bytes = self.m >> 3;
            self.bits = vec![0u8; nbits_bytes];
            self.count = 0;
            self.nbuckets = 0;
            // self.buckets is already empty (taken)
            for &entry in buckets_old.iter() {
                if entry_dib(entry) != 0 {
                    self.testadd(entry, true);
                }
            }
        } else {
            self.count = 0;
            self.nbuckets = nbuckets_new;
            self.buckets = vec![0u64; nbuckets_new];
            for &entry in buckets_old.iter() {
                if entry_dib(entry) != 0 {
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
        let mut key = key & KEY_MASK;
        let mut dib: u64 = 1;
        let mask = (self.nbuckets - 1) as u64;
        let mut i = (key & mask) as usize;
        loop {
            let entry = self.buckets[i];
            let edib = entry_dib(entry);
            let ekey = entry_key(entry);
            if edib == 0 {
                self.buckets[i] = make_entry(key, dib);
                self.count += 1;
                return true;
            }
            if ekey == key {
                return true;
            }
            if edib < dib {
                let tmp = self.buckets[i];
                self.buckets[i] = make_entry(key, dib);
                key = entry_key(tmp);
                dib = entry_dib(tmp);
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Inlined immutable variant of testadd(key, false).
            let mut key = key & KEY_MASK;
            let m = self.m as u128;
            let k = self.k;
            let bits = self.bits.as_slice();
            let mut i: usize = 0;
            let mut j = (((key as u128) * m) >> 56) as usize;
            loop {
                if (bits[j >> 3] >> ((j & 7) as u32)) & 1 == 0 {
                    return false;
                }
                if i + 1 == k {
                    break;
                }
                key = key.wrapping_mul(0x94d049bb133111eb);
                key ^= key >> 31;
                key &= KEY_MASK;
                j = (((key as u128) * m) >> 56) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = key & KEY_MASK;
        let mut dib: u64 = 1;
        let mask = (self.nbuckets - 1) as u64;
        let mut i = (key & mask) as usize;
        loop {
            let entry = self.buckets[i];
            let edib = entry_dib(entry);
            let ekey = entry_key(entry);
            let yes = ekey == key;
            let no = edib < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
}
