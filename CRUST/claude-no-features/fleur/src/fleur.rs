use std::{fmt, fs::File, io::{Read, Seek, SeekFrom, Write}};

const M: u64 = 18446744073709551557;
const G: u64 = 18446744073709550147;
const FNV_PRIME: u64 = 1099511628211;
const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

#[derive(Debug, Clone)]
pub struct Header {
    pub version: u64,
    pub n: u64,
    pub p: f64,
    pub k: u64,
    pub m: u64,
    pub n_value: u64,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Header details:\n version: {}\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n",
            self.version, self.n, self.p, self.k, self.m, self.n_value
        )
    }
}

#[derive(Debug, Clone)]
pub struct BloomFilter {
    pub version: u64,
    pub datasize: u64,
    pub h: Header,
    pub m: u64,
    pub v: Vec<u64>,
    pub data: Vec<u8>,
    pub modified: i32,
    pub error: i32,
}

impl fmt::Display for BloomFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mirror fleur_print_filter formatting.
        let data_str = std::str::from_utf8(&self.data).unwrap_or("");
        write!(
            f,
            "Filter details:\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}

// FNV1 implementation matching the C `fnv1(char*, size_t)` function. Using
// signed-byte XOR to mirror the C `char` semantics on platforms where `char`
// is signed.
fn fnv1_local(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        let signed_byte = b as i8 as i64 as u64;
        h ^= signed_byte;
    }
    h
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        // The C version performs `hn = (hn * g) % m` on `uint64_t`. Because
        // both `hn` and `g` are close to 2^64, the multiplication wraps modulo
        // 2^64 *before* the % m is applied. We replicate that exact behavior.
        let mut tmp = vec![0u64; self.h.k as usize];
        let h = fnv1_local(buf);
        let mut hn: u64 = h % M;
        for i in 0..self.h.k as usize {
            hn = hn.wrapping_mul(G) % M;
            tmp[i] = hn % self.h.m;
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        // Lazily compute the filter parameters if the caller constructed
        // the BloomFilter directly with default zero values (which the test
        // helper `generate_example_filter` does). This mirrors what
        // `fleur_initialize` does in C, but allows the Rust API to be used
        // ergonomically without forcing callers to invoke `initialize`.
        if self.h.m == 0 || self.v.is_empty() {
            let ln_p = self.h.p.ln();
            let ln2 = 2f64.ln();
            let m_f = ((self.h.n as f64) * ln_p / (ln2 * ln2)).ceil().abs();
            let m_val = m_f as u64;
            let k_val = (ln2 * (m_val as f64) / (self.h.n as f64)).ceil() as u64;
            let m_words = ((m_val as f64) / 64.0).ceil() as u64;
            self.h.m = m_val;
            self.h.k = k_val;
            self.m = m_words;
            self.v = vec![0u64; m_words as usize];
        }
        if (self.h.n_value + 1) <= self.h.n {
            let mut new_value = 0;
            let fp = self.fingerprint(buf);
            for i in 0..self.h.k as usize {
                let k = (fp[i] / 64) as usize;
                let l = fp[i] % 64;
                let v = 1u64 << l;
                if (self.v[k] & v) == 0 {
                    new_value = 1;
                }
                self.v[k] |= v;
            }
            if new_value == 1 {
                self.h.n_value += 1;
                self.modified = 1;
                1
            } else {
                0
            }
        } else {
            -1
        }
    }

    pub fn join(&mut self, bf: &BloomFilter) -> i32 {
        // NOTE: in the C version, the function is `fleur_join(src, dst)` and
        // mutates `dst`. The Rust API receives `self` as `dst` and `bf` as `src`.
        // The tests are written assuming `dst.join(&src)`:
        //   `j2.join(&j1)` => src=j1, dst=j2 (where j2 is full).
        let src = bf;
        let dst = self;
        if src.h.n != dst.h.n
            || src.h.p != dst.h.p
            || src.h.k != dst.h.k
            || src.h.m != dst.h.m
            || src.m != dst.m
        {
            return 0;
        }
        if (dst.h.n_value + src.h.n_value) > dst.h.n {
            return -1;
        }
        for i in 0..dst.m as usize {
            dst.v[i] |= src.v[i];
        }
        dst.h.n_value += src.h.n_value;
        1
    }

    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for i in 0..self.h.k as usize {
            let k = (fp[i] / 64) as usize;
            let l = fp[i] % 64;
            let v = 1u64 << l;
            if (self.v[k] & v) == 0 {
                return 0;
            }
        }
        1
    }

    pub fn set_data(&mut self, buf: &[u8]) {
        self.data = buf.to_vec();
        self.datasize = buf.len() as u64;
    }

    pub fn to_file(&self, mut file: File) -> i32 {
        // Mirror fleur_bloom_filter_to_file. Always returns 1 in the Rust port
        // when writes succeed (the C version's `ret` tracking is overridden by
        // the final fwrite of Data, so the success case is when no I/O fails).
        let version: u64 = 1;
        if file.write_all(&version.to_le_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.n.to_le_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.p.to_le_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.k.to_le_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.m.to_le_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.n_value.to_le_bytes()).is_err() {
            return 0;
        }
        for &word in &self.v {
            if file.write_all(&word.to_le_bytes()).is_err() {
                return 0;
            }
        }
        if !self.data.is_empty() {
            if file.write_all(&self.data).is_err() {
                return 0;
            }
        }
        1
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        // m = |ceil(n * ln(p) / ln(2)^2)|
        let ln_p = p.ln();
        let ln2 = 2f64.ln();
        let m_f = ((n as f64) * ln_p / (ln2 * ln2)).ceil().abs();
        let m_val = m_f as u64;
        // k = ceil(ln(2) * m / n)
        let k_val = (ln2 * (m_val as f64) / (n as f64)).ceil() as u64;
        let m_words = ((m_val as f64) / 64.0).ceil() as u64;
        let v = vec![0u64; m_words as usize];

        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: Header {
                version: 1,
                n,
                p,
                k: k_val,
                m: m_val,
                n_value: 0,
            },
            m: m_words,
            v,
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }

    pub fn from_file(mut file: File) -> BloomFilter {
        // Initialize an "empty" BloomFilter we can return with error=1 on
        // failure paths.
        let mut bf = BloomFilter {
            version: 0,
            datasize: 0,
            h: Header {
                version: 0,
                n: 0,
                p: 0.0,
                k: 0,
                m: 0,
                n_value: 0,
            },
            m: 0,
            v: Vec::new(),
            data: Vec::new(),
            modified: 0,
            error: 0,
        };

        // Read the 48-byte header.
        let mut header_bytes = [0u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
            bf.error = 1;
            return bf;
        }
        let version = u64::from_le_bytes(header_bytes[0..8].try_into().unwrap());
        let n = u64::from_le_bytes(header_bytes[8..16].try_into().unwrap());
        let p = f64::from_le_bytes(header_bytes[16..24].try_into().unwrap());
        let k = u64::from_le_bytes(header_bytes[24..32].try_into().unwrap());
        let m = u64::from_le_bytes(header_bytes[32..40].try_into().unwrap());
        let n_value = u64::from_le_bytes(header_bytes[40..48].try_into().unwrap());

        let h = Header {
            version,
            n,
            p,
            k,
            m,
            n_value,
        };

        if !h.check() {
            bf.error = 1;
            return bf;
        }

        let m_words = ((h.m as f64) / 64.0).ceil() as u64;
        bf.m = m_words;

        // Determine total file size.
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => {
                bf.error = 1;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            bf.error = 1;
            return bf;
        }

        if (m_words as i64) <= (size - 48) {
            let bitarray_bytes = ((m_words * 64) as f64 / 8.0).ceil() as i64;
            let datasize = (size - bitarray_bytes - 48) as u64;
            bf.datasize = datasize;

            // Load bit array.
            let mut v = vec![0u64; m_words as usize];
            for word in v.iter_mut() {
                let mut buf8 = [0u8; 8];
                if file.read_exact(&mut buf8).is_err() {
                    bf.error = 1;
                    return bf;
                }
                *word = u64::from_le_bytes(buf8);
            }
            bf.v = v;

            // Load remaining data.
            if datasize > 0 {
                let mut data = vec![0u8; datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    bf.error = 1;
                    return bf;
                }
                bf.data = data;
            } else {
                bf.data = Vec::new();
            }
        }

        bf.modified = 0;
        bf.error = 0;
        bf.h = h;
        bf.version = version;
        bf
    }
}

impl Header {
    pub fn check(&self) -> bool {
        if self.version != 1 {
            return false;
        }
        let maxint: u64 = 9223372036854775807;
        if self.k >= maxint {
            return false;
        }
        if self.p <= f64::EPSILON {
            return false;
        }
        if self.p > 1.0 {
            return false;
        }
        if self.n_value > self.n {
            return false;
        }
        let ln_p = self.p.ln();
        let ln2 = 2f64.ln();
        let tmp_m = ((self.n as f64) * ln_p / (ln2 * ln2)).ceil().abs() as u64;
        if tmp_m != self.m {
            return false;
        }
        let tmp_k = (ln2 * (self.m as f64) / (self.n as f64)).ceil() as u64;
        if tmp_k != self.k {
            return false;
        }
        true
    }
}

// Avoid an "unused constants" warning for FNV constants that are also used
// implicitly via fnv1_local.
#[allow(dead_code)]
const _UNUSED_FNV: (u64, u64) = (FNV_PRIME, FNV_OFFSET_BASIS);
