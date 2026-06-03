use std::io::{Read, Seek, SeekFrom, Write};
use std::{fmt, fs::File};
const M: u64 = 18446744073709551557;
const G: u64 = 18446744073709550147;
const FNV_PRIME: u64 = 1099511628211;
const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
#[derive(Debug, Clone)]
#[repr(C)]
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
        let h = fnv1_local(buf);
        let mut hn = h % M;
        let mut tmp = Vec::with_capacity(self.h.k as usize);
        for _ in 0..self.h.k {
            // Match C's u64 wrapping multiplication, then reduce mod M.
            hn = hn.wrapping_mul(G) % M;
            tmp.push(hn % self.h.m);
        }
        tmp
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        // Lazy initialization: if the filter was constructed manually with
        // k/m/v unset (matching the test fixture pattern), derive them now.
        if self.h.k == 0 && self.h.m == 0 && self.v.is_empty() && self.h.n > 0 {
            let n_f = self.h.n as f64;
            let m_bits =
                (n_f * self.h.p.ln() / 2.0_f64.ln().powi(2)).ceil().abs() as u64;
            let k = (2.0_f64.ln() * m_bits as f64 / n_f).ceil() as u64;
            self.h.m = m_bits;
            self.h.k = k;
            self.m = ((m_bits as f64) / 64.0).ceil() as u64;
            self.v = vec![0u64; self.m as usize];
        }
        if (self.h.n_value + 1) <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = 0;
            for i in 0..self.h.k as usize {
                let k = (fp[i] / 64) as usize;
                let l = fp[i] % 64;
                let v = 1u64 << l;
                if self.v[k] & v == 0 {
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
        if bf.h.n != self.h.n {
            return 0;
        }
        if bf.h.p != self.h.p {
            return 0;
        }
        if bf.h.k != self.h.k {
            return 0;
        }
        if bf.h.m != self.h.m {
            return 0;
        }
        if bf.m != self.m {
            return 0;
        }
        if (self.h.n_value + bf.h.n_value) > self.h.n {
            return -1;
        }
        for i in 0..self.m as usize {
            self.v[i] |= bf.v[i];
        }
        self.h.n_value += bf.h.n_value;
        1
    }
    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for i in 0..self.h.k as usize {
            let k = (fp[i] / 64) as usize;
            let l = fp[i] % 64;
            let v = 1u64 << l;
            if self.v[k] & v == 0 {
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
        let version: u64 = 1;
        if file.write_all(&version.to_ne_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.n.to_ne_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.p.to_ne_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.k.to_ne_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.m.to_ne_bytes()).is_err() {
            return 0;
        }
        if file.write_all(&self.h.n_value.to_ne_bytes()).is_err() {
            return 0;
        }
        for &v in &self.v {
            if file.write_all(&v.to_ne_bytes()).is_err() {
                return 0;
            }
        }
        if !self.data.is_empty() {
            let _ = file.write_all(&self.data);
        }
        1
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_bits = ((n as f64) * p.ln() / 2.0_f64.ln().powi(2))
            .ceil()
            .abs() as u64;
        let k = (2.0_f64.ln() * m_bits as f64 / n as f64).ceil() as u64;
        let m_count = ((m_bits as f64) / 64.0).ceil() as u64;
        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: Header {
                version: 1,
                n,
                p,
                k,
                m: m_bits,
                n_value: 0,
            },
            m: m_count,
            v: vec![0u64; m_count as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(mut file: File) -> BloomFilter {
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
        let mut header_bytes = [0u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
            bf.error = 1;
            return bf;
        }
        let h = unsafe { std::ptr::read(header_bytes.as_ptr() as *const Header) };
        if !h.check() {
            bf.error = 1;
            return bf;
        }
        let m_count = ((h.m as f64) / 64.0).ceil() as u64;
        bf.m = m_count;
        let total_size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s,
            Err(_) => {
                bf.error = 1;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            bf.error = 1;
            return bf;
        }
        if total_size >= 48 && m_count <= total_size - 48 {
            let v_bytes_total = ((m_count as f64) * 64.0 / 8.0).ceil() as u64;
            let datasize = total_size - v_bytes_total - 48;
            bf.datasize = datasize;
            let mut v_bytes = vec![0u8; (m_count * 8) as usize];
            if file.read_exact(&mut v_bytes).is_err() {
                bf.error = 1;
                return bf;
            }
            let mut v = Vec::with_capacity(m_count as usize);
            for chunk in v_bytes.chunks_exact(8) {
                v.push(u64::from_ne_bytes(chunk.try_into().unwrap()));
            }
            bf.v = v;
            if datasize > 0 {
                let mut data = vec![0u8; datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    bf.error = 1;
                    return bf;
                }
                bf.data = data;
            }
        }
        bf.modified = 0;
        bf.error = 0;
        bf.h = h;
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
        let tmp_m = ((self.n as f64) * self.p.ln() / 2.0_f64.ln().powi(2))
            .ceil()
            .abs() as u64;
        if tmp_m != self.m {
            return false;
        }
        let tmp_k = (2.0_f64.ln() * self.m as f64 / self.n as f64).ceil() as u64;
        if tmp_k != self.k {
            return false;
        }
        true
    }
}

// Local FNV1 implementation matching c_src/fnv/fnv.c. We replicate it here
// instead of relying on `crate::fnv::fnv1` so that internal callers don't
// depend on cross-module visibility ordering.
fn fnv1_local(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        h ^= b as u64;
    }
    h
}
