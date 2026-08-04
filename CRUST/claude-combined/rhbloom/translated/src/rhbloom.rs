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

const MIX_MUL1: u64 = 0xbf58476d1ce4e5b9;
const MIX_MUL2: u64 = 0x94d049bb133111eb;

#[inline]
fn key_only(x: u64) -> u64 {
    // RHBLOOM_KEY: keep low 56 bits
    x << 8 >> 8
}

#[inline]
fn key_dib(x: u64) -> u64 {
    // RHBLOOM_DIB: top 8 bits
    x >> 56
}

#[inline]
fn make_keydib(key: u64, dib: u64) -> u64 {
    key_only(key) | (dib << 56)
}

// Implement the RHBloom struct
impl RHBloom {
    pub fn new(n: usize, p: f64) -> Self {
        let n = if n < 16 { 16 } else { n };

        // Calculate the total number of bits needed
        let denom = (1.0_f64 / 2.0_f64.powf(2.0_f64.ln())).ln();
        let m = (n as f64 * p.ln() / denom) as usize;

        // Compute power-of-two candidates around m
        let mut m_up: usize = 2;
        while m_up < m {
            m_up *= 2;
        }
        let m_down = if m_up > 2 { m_up / 2 } else { 2 };

        // Predicted false-positive rate for given (m, n, k)
        fn predict(m: usize, n: usize, k: usize) -> f64 {
            if k == 0 || m == 0 || n == 0 {
                return 1.0;
            }
            let exponent = -(k as f64) * (n as f64) / (m as f64);
            let inner = 1.0 - exponent.exp();
            inner.powi(k as i32)
        }

        // For each candidate m0, find the k that gives the predicted FP rate
        // closest to the target p.
        let pick_best = |m0: usize| -> (usize, f64) {
            let mut best_k = 1usize;
            let mut best_diff = f64::INFINITY;
            // Optimal k bound for this m
            let k_opt = ((m0 as f64 / n as f64) * 2.0_f64.ln()).round() as usize;
            let k_max = (k_opt + 4).max(8).min(60);
            for k_try in 1..=k_max {
                let p_pred = predict(m0, n, k_try);
                let diff = (p_pred - p).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best_k = k_try;
                }
            }
            (best_k, best_diff)
        };

        let (k_down, d_down) = pick_best(m_down);
        let (k_up, d_up) = pick_best(m_up);

        let (m0, mut k0) = if d_down <= d_up {
            (m_down, k_down)
        } else {
            (m_up, k_up)
        };
        if k0 == 0 {
            k0 = 1;
        }

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
        // Use only the 56-bit key
        let mut k = key_only(key);
        let mask = (self.m as u64).wrapping_sub(1);
        let mut i: usize = 0;
        let mut j = (k & mask) as usize;
        loop {
            if add {
                self.bits[j >> 3] |= 1u8 << (j & 7);
            } else if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                return false;
            }
            if i == self.k - 1 {
                break;
            }
            k = k.wrapping_mul(MIX_MUL2);
            k ^= k >> 31;
            j = (k & mask) as usize;
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

    pub fn mix(mut key: u64) -> u64 {
        // mix13
        key ^= key >> 30;
        key = key.wrapping_mul(MIX_MUL1);
        key ^= key >> 27;
        key = key.wrapping_mul(MIX_MUL2);
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
                if key_dib(buckets_old[i]) != 0 {
                    self.testadd(buckets_old[i], true);
                }
            }
        } else {
            self.buckets = vec![0u64; nbuckets_new];
            self.count = 0;
            self.nbuckets = nbuckets_new;
            for i in 0..nbuckets_old {
                if key_dib(buckets_old[i]) != 0 {
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
        let mut key = key_only(key);
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i = (key & mask) as usize;
        loop {
            let bucket = self.buckets[i];
            let bucket_dib = key_dib(bucket);
            if bucket_dib == 0 {
                self.buckets[i] = make_keydib(key, dib);
                self.count += 1;
                return true;
            }
            if key_only(bucket) == key {
                return true;
            }
            if bucket_dib < dib {
                let tmp = bucket;
                self.buckets[i] = make_keydib(key, dib);
                key = key_only(tmp);
                dib = key_dib(tmp);
            }
            dib += 1;
            i = ((i as u64 + 1) & mask) as usize;
        }
    }

    pub fn test(&self, key: u64) -> bool {
        let key = Self::mix(key);
        if !self.bits.is_empty() {
            // Bloom-mode test (read-only version of testadd)
            let mut k = key_only(key);
            let mask = (self.m as u64).wrapping_sub(1);
            let mut i: usize = 0;
            let mut j = (k & mask) as usize;
            loop {
                if (self.bits[j >> 3] >> (j & 7)) & 1 == 0 {
                    return false;
                }
                if i == self.k - 1 {
                    break;
                }
                k = k.wrapping_mul(MIX_MUL2);
                k ^= k >> 31;
                j = (k & mask) as usize;
                i += 1;
            }
            return true;
        }
        if self.buckets.is_empty() {
            return false;
        }
        let key = key_only(key);
        let mut dib: u64 = 1;
        let mask = (self.nbuckets as u64).wrapping_sub(1);
        let mut i = (key & mask) as usize;
        loop {
            let bucket = self.buckets[i];
            let yes = key_only(bucket) == key;
            let no = key_dib(bucket) < dib;
            if yes || no {
                return yes;
            }
            dib += 1;
            i = ((i as u64 + 1) & mask) as usize;
        }
    }
}
