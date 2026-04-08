// Helper functions for key/dib encoding in u64 buckets
fn rhbloom_key(x: u64) -> u64 { (x << 8) >> 8 }
fn rhbloom_dib(x: u64) -> i32 { (x >> 56) as i32 }
fn rhbloom_setkeydib(key: u64, dib: i32) -> u64 { rhbloom_key(key) | ((dib as u64) << 56) }

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
        let m = (n as f64 * p.ln() / (1.0 / 2.0_f64.powf(2.0_f64.ln())).ln()) as usize;
        let k = ((m as f64 / n as f64) * 2.0_f64.ln()).round() as usize;
        let mut m0: usize = 2;
        while m0 < m { m0 *= 2; }
        let k0 = (m as f64 / m0 as f64 * k as f64).round() as usize;
        RHBloom { count: 0, nbuckets: 0, buckets: Vec::new(), k: k0, m: m0, bits: Vec::new() }
    }
    pub fn memsize(&self) -> usize {
        let base = std::mem::size_of::<RHBloom>();
        base + if !self.bits.is_empty() { self.m >> 3 } else { self.nbuckets << 3 }
    }
    pub fn clear(&mut self) {
        if !self.bits.is_empty() {
            self.bits.iter_mut().for_each(|b| *b = 0);
        } else if !self.buckets.is_empty() {
            self.buckets.iter_mut().for_each(|b| *b = 0);
            self.count = 0;
        }
    }
    pub fn upgraded(&self) -> bool {
        !self.bits.is_empty()
    }
    pub fn testadd(&mut self, key: u64, add: bool) -> bool {
        let key_init = rhbloom_key(key);
        let mut key = key_init;
        let mut i = 0usize;
        let mut j = (key as usize) & (self.m - 1);
        loop {
            if add {
                self.bits[j >> 3] |= 1 << (j & 7);
            } else if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                return false;
            }
            if i == self.k - 1 { break; }
            key = key.wrapping_mul(0x94d049bb133111eb);
            key ^= key >> 31;
            j = (key as usize) & (self.m - 1);
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
                if !self.grow() { return false; }
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
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Need mutable self for testadd, but test is &self.
            // For test (add=false), we only read bits, so replicate inline.
            let mut k = rhbloom_key(key);
            let mut i = 0usize;
            let mut j = (k as usize) & (self.m - 1);
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 { return false; }
                if i == self.k - 1 { break; }
                k = k.wrapping_mul(0x94d049bb133111eb);
                k ^= k >> 31;
                j = (k as usize) & (self.m - 1);
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() { return false; }
        let key = rhbloom_key(key);
        let mut dib = 1i32;
        let mut i = (key as usize) & (self.nbuckets - 1);
        loop {
            let yes = rhbloom_key(self.buckets[i]) == key;
            let no = rhbloom_dib(self.buckets[i]) < dib;
            if yes || no { return yes; }
            dib += 1;
            i = (i + 1) & (self.nbuckets - 1);
        }
    }
}
