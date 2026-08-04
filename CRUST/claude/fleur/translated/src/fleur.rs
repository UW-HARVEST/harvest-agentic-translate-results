use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{fmt, fs::File};

use crate::fnv;

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

// Helper: compute m given n, p (matches C: fabs(ceil(n * log(p) / log(2)^2)))
fn compute_m(n: u64, p: f64) -> u64 {
    let val = (n as f64) * p.ln() / (2f64.ln()).powi(2);
    val.ceil().abs() as u64
}

// Helper: compute k given m, n (matches C: ceil(log(2) * m / n))
fn compute_k(m: u64, n: u64) -> u64 {
    (2f64.ln() * (m as f64) / (n as f64)).ceil() as u64
}

// Helper: compute big M (number of u64s) given m (number of bits): ceil(m / 64)
fn compute_big_m(m: u64) -> u64 {
    ((m as f64) / 64.0).ceil() as u64
}

// Helper: ensures the BloomFilter is initialized (k, m, big_m, v) from h.n and h.p
// when those fields are zero. Mirrors what fleur_initialize in C does.
fn ensure_initialized(bf: &mut BloomFilter) {
    if bf.h.k == 0 || bf.h.m == 0 || bf.m == 0 || bf.v.is_empty() {
        let m_bits = compute_m(bf.h.n, bf.h.p);
        let k = compute_k(m_bits, bf.h.n);
        let big_m = compute_big_m(m_bits);
        bf.h.m = m_bits;
        bf.h.k = k;
        bf.m = big_m;
        bf.v = vec![0u64; big_m as usize];
    }
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        // Mirrors the C implementation, which uses native uint64_t wrapping
        // multiplication followed by `% m`.
        let mut tmp = Vec::with_capacity(self.h.k as usize);
        let h = fnv::fnv1(buf);
        let mut hn = h % M;
        for _ in 0..self.h.k {
            hn = hn.wrapping_mul(G) % M;
            tmp.push(hn % self.h.m);
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        ensure_initialized(self);
        if self.h.n_value + 1 <= self.h.n {
            let mut new_value = false;
            let fp = self.fingerprint(buf);
            for &fp_i in &fp {
                let k = (fp_i / 64) as usize;
                let l = fp_i % 64;
                let v = 1u64 << l;
                if self.v[k] & v == 0 {
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
        } else {
            -1
        }
    }

    pub fn join(&mut self, bf: &BloomFilter) -> i32 {
        // self == dst, bf == src (mirrors C `fleur_join(src, dst)` invocation order
        // chosen in the Rust tests via method-call-on-dst).
        let src = bf;
        let dst = self;
        if src.h.n != dst.h.n
            || src.h.p != dst.h.p
            || src.h.k != dst.h.k
            || src.h.m != dst.h.m
            || src.m != dst.m
        {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if dst.h.n_value + src.h.n_value > dst.h.n {
            eprintln!("[ERROR] Destination Filter is full.");
            return -1;
        }
        for i in 0..dst.m as usize {
            dst.v[i] |= src.v[i];
        }
        dst.h.n_value += src.h.n_value;
        1
    }

    pub fn check(&self, buf: &[u8]) -> i32 {
        if self.h.k == 0 || self.h.m == 0 || self.v.is_empty() {
            // Uninitialized filter never contains anything.
            return 0;
        }
        let fp = self.fingerprint(buf);
        for &fp_i in &fp {
            let k = (fp_i / 64) as usize;
            let l = fp_i % 64;
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
        // Mirror the C serialization layout: bf.version, then h.n, h.p, h.k, h.m,
        // h.n_value, then bit-array, then data.
        let version: u64 = 1;
        let mut ok = true;
        ok &= file.write_all(&version.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.n.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.p.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.k.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.m.to_le_bytes()).is_ok();
        ok &= file.write_all(&self.h.n_value.to_le_bytes()).is_ok();
        for &word in &self.v {
            ok &= file.write_all(&word.to_le_bytes()).is_ok();
        }
        // Data may be empty.
        let data_len = self.datasize as usize;
        let data_to_write = if data_len <= self.data.len() {
            &self.data[..data_len]
        } else {
            &self.data[..]
        };
        let _ = file.write_all(data_to_write);
        if ok {
            1
        } else {
            eprintln!("[ERROR] writing filter file.");
            0
        }
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_bits = compute_m(n, p);
        let k = compute_k(m_bits, n);
        let big_m = compute_big_m(m_bits);
        let h = Header {
            version: 1,
            n,
            p,
            k,
            m: m_bits,
            n_value: 0,
        };
        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h,
            m: big_m,
            v: vec![0u64; big_m as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }

    pub fn from_file(mut file: File) -> BloomFilter {
        let mut empty_header = Header {
            version: 0,
            n: 0,
            p: 0.0,
            k: 0,
            m: 0,
            n_value: 0,
        };
        let mut bf = BloomFilter {
            version: 0,
            datasize: 0,
            h: empty_header.clone(),
            m: 0,
            v: Vec::new(),
            data: Vec::new(),
            modified: 0,
            error: 0,
        };

        // Determine total file size.
        let file_size = match file.seek(SeekFrom::End(0)) {
            Ok(sz) => sz,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                bf.error = 1;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(0)).is_err() {
            bf.error = 1;
            return bf;
        }

        // Read header (48 bytes).
        let mut header_bytes = [0u8; 48];
        match file.read_exact(&mut header_bytes) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("[ERROR] reading filter file.");
                bf.error = 1;
                return bf;
            }
        }

        empty_header.version = u64::from_le_bytes(header_bytes[0..8].try_into().unwrap());
        empty_header.n = u64::from_le_bytes(header_bytes[8..16].try_into().unwrap());
        empty_header.p = f64::from_le_bytes(header_bytes[16..24].try_into().unwrap());
        empty_header.k = u64::from_le_bytes(header_bytes[24..32].try_into().unwrap());
        empty_header.m = u64::from_le_bytes(header_bytes[32..40].try_into().unwrap());
        empty_header.n_value = u64::from_le_bytes(header_bytes[40..48].try_into().unwrap());

        if !empty_header.check() {
            eprintln!("[ERROR] Incoherent header.");
            bf.error = 1;
            return bf;
        }

        let big_m = compute_big_m(empty_header.m);
        bf.m = big_m;

        // C uses the (somewhat odd) check `bf.M <= (size - 48)`.
        if file_size >= 48 && big_m <= file_size - 48 {
            let bitmap_bytes = big_m * 8;
            let datasize = file_size - bitmap_bytes - 48;
            bf.datasize = datasize;

            let mut v = vec![0u64; big_m as usize];
            for i in 0..big_m as usize {
                let mut word = [0u8; 8];
                if file.read_exact(&mut word).is_err() {
                    eprintln!("[ERROR] Cannot load bitarray.");
                    bf.error = 1;
                    return bf;
                }
                v[i] = u64::from_le_bytes(word);
            }
            bf.v = v;

            if datasize > 0 {
                let mut data = vec![0u8; datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
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
        bf.h = empty_header;
        bf.version = bf.h.version;
        bf
    }
}

impl Header {
    pub fn check(&self) -> bool {
        if self.version != 1 {
            eprintln!("[ERROR] Current filter version not supported.");
            return false;
        }
        // 0111111111111111111111111111111111111111111111111111111111111111b
        let maxint: u64 = 9223372036854775807;
        if self.k >= maxint {
            eprintln!("[ERROR] value of k (number of hash functions) is too high.");
            return false;
        }
        if self.p <= f64::EPSILON {
            eprintln!("[ERROR] p is too small.");
            return false;
        }
        if self.p > 1.0 {
            eprintln!("[ERROR] p is more than one.");
            return false;
        }
        if self.n_value > self.n {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_m = compute_m(self.n, self.p);
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = compute_k(self.m, self.n);
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}

// Suppress unused-const warnings for constants kept for documentation/parity.
#[allow(dead_code)]
const _UNUSED_FNV_PRIME: u64 = FNV_PRIME;
#[allow(dead_code)]
const _UNUSED_FNV_OFFSET_BASIS: u64 = FNV_OFFSET_BASIS;
