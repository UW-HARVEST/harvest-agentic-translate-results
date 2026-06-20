use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

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
            "Header details:\n version: {}\n n: {} \n p: {}\n k: {} \n m: {} \n N: {} \n",
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
        write!(
            f,
            "Filter details:\n n: {} \n p: {}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n,
            self.h.p,
            self.h.k,
            self.h.m,
            self.h.n_value,
            self.m,
            String::from_utf8_lossy(&self.data)
        )
    }
}
impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut h = FNV_OFFSET_BASIS;
        for &byte in buf {
            h = h.wrapping_mul(FNV_PRIME);
            h ^= u64::from(byte);
        }

        let mut fingerprint = Vec::with_capacity(self.h.k as usize);
        let mut hn = h % M;
        for _ in 0..self.h.k {
            hn = hn.wrapping_mul(G) % M;
            fingerprint.push(hn % self.h.m);
        }
        fingerprint
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        if self.h.n_value.saturating_add(1) > self.h.n {
            return -1;
        }

        let mut new_value = false;
        for bit in self.fingerprint(buf) {
            let word_index = (bit / 64) as usize;
            let bit_index = bit % 64;
            let mask = 1u64 << bit_index;
            if (self.v[word_index] & mask) == 0 {
                new_value = true;
            }
            self.v[word_index] |= mask;
        }

        if new_value {
            self.h.n_value += 1;
            self.modified = 1;
            1
        } else {
            0
        }
    }
    pub fn join(&mut self, bf: &BloomFilter) -> i32 {
        if self.h.n != bf.h.n
            || self.h.p != bf.h.p
            || self.h.k != bf.h.k
            || self.h.m != bf.h.m
            || self.m != bf.m
        {
            return 0;
        }

        if self.h.n_value.saturating_add(bf.h.n_value) > self.h.n {
            return -1;
        }

        for (dst, src) in self.v.iter_mut().zip(&bf.v) {
            *dst |= *src;
        }
        self.h.n_value += bf.h.n_value;
        1
    }
    pub fn check(&self, buf: &[u8]) -> i32 {
        for bit in self.fingerprint(buf) {
            let word_index = (bit / 64) as usize;
            let bit_index = bit % 64;
            let mask = 1u64 << bit_index;
            if (self.v[word_index] & mask) == 0 {
                return 0;
            }
        }
        1
    }
    pub fn set_data(&mut self, buf: &[u8]) {
        self.data = buf.to_vec();
        self.datasize = buf.len() as u64;
    }
    pub fn to_file(&self, file: File) -> i32 {
        let mut file = file;
        let version = 1u64;

        if file.write_all(&version.to_le_bytes()).is_err()
            || file.write_all(&self.h.n.to_le_bytes()).is_err()
            || file.write_all(&self.h.p.to_le_bytes()).is_err()
            || file.write_all(&self.h.k.to_le_bytes()).is_err()
            || file.write_all(&self.h.m.to_le_bytes()).is_err()
            || file.write_all(&self.h.n_value.to_le_bytes()).is_err()
        {
            return 0;
        }

        for word in &self.v {
            if file.write_all(&word.to_le_bytes()).is_err() {
                return 0;
            }
        }

        if file.write_all(&self.data).is_err() {
            return 0;
        }

        1
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m = ((n as f64 * p.ln()) / (2.0f64.ln().powi(2))).ceil().abs() as u64;
        let k = ((2.0f64.ln() * m as f64) / n as f64).ceil() as u64;
        let words = (m as f64 / 64.0).ceil() as u64;
        let data = buf.to_vec();

        BloomFilter {
            version: 1,
            datasize: data.len() as u64,
            h: Header {
                version: 1,
                n,
                p,
                k,
                m,
                n_value: 0,
            },
            m: words,
            v: vec![0; words as usize],
            data,
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(file: File) -> BloomFilter {
        let mut file = file;

        let read_u64 = |file: &mut File| -> Option<u64> {
            let mut buf = [0u8; 8];
            file.read_exact(&mut buf).ok()?;
            Some(u64::from_le_bytes(buf))
        };
        let read_f64 = |file: &mut File| -> Option<f64> {
            let mut buf = [0u8; 8];
            file.read_exact(&mut buf).ok()?;
            Some(f64::from_le_bytes(buf))
        };

        let header = match (
            read_u64(&mut file),
            read_u64(&mut file),
            read_f64(&mut file),
            read_u64(&mut file),
            read_u64(&mut file),
            read_u64(&mut file),
        ) {
            (Some(version), Some(n), Some(p), Some(k), Some(m), Some(n_value)) => Header {
                version,
                n,
                p,
                k,
                m,
                n_value,
            },
            _ => {
                return BloomFilter {
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
                    error: 1,
                };
            }
        };

        if !header.check() {
            return BloomFilter {
                version: header.version,
                datasize: 0,
                h: header,
                m: 0,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        let words = (header.m as f64 / 64.0).ceil() as u64;

        let file_size = match file.seek(SeekFrom::End(0)) {
            Ok(size) => size,
            Err(_) => {
                return BloomFilter {
                    version: header.version,
                    datasize: 0,
                    h: header,
                    m: words,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                };
            }
        };

        if file.seek(SeekFrom::Start(48)).is_err() {
            return BloomFilter {
                version: header.version,
                datasize: 0,
                h: header,
                m: words,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        let bitmap_bytes = words.saturating_mul(8);
        let datasize = file_size.saturating_sub(bitmap_bytes + 48);

        let mut v = vec![0u64; words as usize];
        for word in &mut v {
            let mut buf = [0u8; 8];
            if file.read_exact(&mut buf).is_err() {
                return BloomFilter {
                    version: header.version,
                    datasize,
                    h: header,
                    m: words,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                };
            }
            *word = u64::from_le_bytes(buf);
        }

        let mut data = vec![0u8; datasize as usize];
        if datasize > 0 && file.read_exact(&mut data).is_err() {
            return BloomFilter {
                version: header.version,
                datasize,
                h: header,
                m: words,
                v,
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        BloomFilter {
            version: header.version,
            datasize,
            h: header,
            m: words,
            v,
            data,
            modified: 0,
            error: 0,
        }
    }
}
impl Header {
    pub fn check(&self) -> bool {
        if self.version != 1 {
            return false;
        }

        let maxint = 9_223_372_036_854_775_807u64;
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

        let tmp_m = ((self.n as f64 * self.p.ln()) / (2.0f64.ln().powi(2)))
            .ceil()
            .abs() as u64;
        if tmp_m != self.m {
            return false;
        }

        let tmp_k = ((2.0f64.ln() * self.m as f64) / self.n as f64).ceil() as u64;
        tmp_k == self.k
    }
}
