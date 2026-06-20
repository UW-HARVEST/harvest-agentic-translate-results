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
        write!(
            f,
            "Filter details:\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
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
    fn derive_parameters(&mut self) {
        if self.h.k != 0 && self.h.m != 0 && self.m != 0 && !self.v.is_empty() {
            return;
        }

        let bit_count = ((self.h.n as f64 * self.h.p.ln()) / f64::ln(2.0).powi(2))
            .ceil()
            .abs() as u64;
        let hash_count = (f64::ln(2.0) * bit_count as f64 / self.h.n as f64).ceil() as u64;
        let word_count = (bit_count as f64 / 64.0).ceil() as u64;

        self.h.k = hash_count;
        self.h.m = bit_count;
        self.m = word_count;
        if self.v.len() != word_count as usize {
            self.v = vec![0; word_count as usize];
        }
    }

    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.h.k as usize);
        let h = {
            let mut h = FNV_OFFSET_BASIS;
            for &byte in buf {
                h = h.wrapping_mul(FNV_PRIME);
                h ^= (byte as i8 as i64) as u64;
            }
            h
        };
        let mut hn = h % M;
        for _ in 0..self.h.k {
            hn = hn.wrapping_mul(G) % M;
            out.push(hn % self.h.m);
        }
        out
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        self.derive_parameters();
        if self.h.n_value + 1 > self.h.n {
            return -1;
        }

        let mut new_value = false;
        for fp in self.fingerprint(buf) {
            let k = (fp / 64) as usize;
            let l = fp % 64;
            let v = 1_u64 << l;
            if (self.v[k] & v) == 0 {
                new_value = true;
            }
            self.v[k] |= v;
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
        if self.h.n_value + bf.h.n_value > self.h.n {
            return -1;
        }
        for (dst, src) in self.v.iter_mut().zip(&bf.v) {
            *dst |= *src;
        }
        self.h.n_value += bf.h.n_value;
        1
    }
    pub fn check(&self, buf: &[u8]) -> i32 {
        if self.h.k == 0 || self.h.m == 0 || self.m == 0 || self.v.is_empty() {
            return 0;
        }
        for fp in self.fingerprint(buf) {
            let k = (fp / 64) as usize;
            let l = fp % 64;
            let v = 1_u64 << l;
            if (self.v[k] & v) == 0 {
                return 0;
            }
        }
        1
    }
    pub fn set_data(&mut self, buf: &[u8]) {
        self.data = buf.to_vec();
        self.datasize = self.data.len() as u64;
    }
    pub fn to_file(&self, file: File) -> i32 {
        let mut file = file;
        let version = 1_u64;
        let data_len = self.datasize.min(self.data.len() as u64) as usize;

        let mut ok = file.write_all(&version.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.n.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.p.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.k.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.m.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.n_value.to_le_bytes()).is_ok();

        for word in &self.v {
            ok &= file.write_all(&word.to_le_bytes()).is_ok();
        }

        if data_len > 0 {
            ok &= file.write_all(&self.data[..data_len]).is_ok();
        }

        if ok { 1 } else { 0 }
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let bit_count = ((n as f64 * p.ln()) / f64::ln(2.0).powi(2)).ceil().abs() as u64;
        let hash_count = (f64::ln(2.0) * bit_count as f64 / n as f64).ceil() as u64;
        let word_count = (bit_count as f64 / 64.0).ceil() as u64;

        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: Header {
                version: 1,
                n,
                p,
                k: hash_count,
                m: bit_count,
                n_value: 0,
            },
            m: word_count,
            v: vec![0; word_count as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(file: File) -> BloomFilter {
        let mut file = file;
        let mut header_bytes = [0_u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
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

        let h = Header {
            version: u64::from_le_bytes(header_bytes[0..8].try_into().unwrap()),
            n: u64::from_le_bytes(header_bytes[8..16].try_into().unwrap()),
            p: f64::from_le_bytes(header_bytes[16..24].try_into().unwrap()),
            k: u64::from_le_bytes(header_bytes[24..32].try_into().unwrap()),
            m: u64::from_le_bytes(header_bytes[32..40].try_into().unwrap()),
            n_value: u64::from_le_bytes(header_bytes[40..48].try_into().unwrap()),
        };

        if !h.check() {
            return BloomFilter {
                version: 0,
                datasize: 0,
                h,
                m: 0,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        let word_count = (h.m as f64 / 64.0).ceil() as u64;
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(size) => size,
            Err(_) => {
                return BloomFilter {
                    version: 0,
                    datasize: 0,
                    h,
                    m: word_count,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                }
            }
        };

        let bitarray_bytes = word_count.saturating_mul(8);
        if size < 48 + bitarray_bytes {
            return BloomFilter {
                version: 0,
                datasize: 0,
                h,
                m: word_count,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        if file.seek(SeekFrom::Start(48)).is_err() {
            return BloomFilter {
                version: 0,
                datasize: 0,
                h,
                m: word_count,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        let mut v = vec![0_u64; word_count as usize];
        for word in &mut v {
            let mut bytes = [0_u8; 8];
            if file.read_exact(&mut bytes).is_err() {
                return BloomFilter {
                    version: 0,
                    datasize: 0,
                    h,
                    m: word_count,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                };
            }
            *word = u64::from_le_bytes(bytes);
        }

        let datasize = size - 48 - bitarray_bytes;
        let mut data = vec![0_u8; datasize as usize];
        if datasize > 0 && file.read_exact(&mut data).is_err() {
            return BloomFilter {
                version: 0,
                datasize: 0,
                h,
                m: word_count,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        BloomFilter {
            version: h.version,
            datasize,
            h,
            m: word_count,
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
        if self.k >= i64::MAX as u64 {
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

        let tmp_m = ((self.n as f64 * self.p.ln()) / f64::ln(2.0).powi(2))
            .ceil()
            .abs() as u64;
        if tmp_m != self.m {
            return false;
        }

        let tmp_k = (f64::ln(2.0) * self.m as f64 / self.n as f64).ceil() as u64;
        tmp_k == self.k
    }
}
