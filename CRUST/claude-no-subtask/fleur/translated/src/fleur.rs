use std::{fmt, fs::File};
use std::io::{Read, Seek, SeekFrom, Write};

use crate::fnv;

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
        // Mirror C's fleur_print_filter: data is printed as a C string.
        let data_str = match std::str::from_utf8(&self.data) {
            Ok(s) => s.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:?}", self.data),
        };
        write!(
            f,
            "Filter details:\n n: {} \n p: {}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let h = fnv::fnv1(buf);
        let mut hn = h % M;
        let mut tmp = Vec::with_capacity(self.h.k as usize);
        for _ in 0..self.h.k {
            hn = hn.wrapping_mul(G) % M;
            tmp.push(hn % self.h.m);
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        // Lazy initialization: if the filter has not been initialized
        // (k=0, m=0, or v is empty) but n and p are set, compute the
        // derived parameters as `initialize` would.
        if (self.h.k == 0 || self.h.m == 0 || self.v.is_empty())
            && self.h.n > 0
            && self.h.p > 0.0
        {
            let ln_p = self.h.p.ln();
            let ln_2 = 2.0f64.ln();
            let m_f = ((self.h.n as f64) * ln_p / ln_2.powi(2)).ceil().abs();
            let m_val = m_f as u64;
            let k_val = ((ln_2 * (m_val as f64)) / (self.h.n as f64)).ceil() as u64;
            let big_m = ((m_val as f64) / 64.0).ceil() as u64;
            self.h.m = m_val;
            self.h.k = k_val;
            self.m = big_m;
            self.v = vec![0u64; big_m as usize];
        }

        if (self.h.n_value + 1) <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = false;
            for &val in &fp {
                let k = (val / 64) as usize;
                let l = val % 64;
                let v_bit: u64 = 1u64 << l;
                if (self.v[k] & v_bit) == 0 {
                    new_value = true;
                }
                self.v[k] |= v_bit;
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
        // self is dst, bf is src
        if bf.h.n != self.h.n {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if bf.h.p != self.h.p {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if bf.h.k != self.h.k {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if bf.h.m != self.h.m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if bf.m != self.m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if (self.h.n_value + bf.h.n_value) > self.h.n {
            eprintln!("[ERROR] Destination Filter is full.");
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
        for &val in &fp {
            let k = (val / 64) as usize;
            let l = val % 64;
            let v_bit: u64 = 1u64 << l;
            if (self.v[k] & v_bit) == 0 {
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
        // Mirror fleur_bloom_filter_to_file: write version=1, then header
        // fields (n, p, k, m, N), then v[], then data[].
        let version: u64 = 1;
        if file.write_all(&version.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        if file.write_all(&self.h.n.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        if file.write_all(&self.h.p.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        if file.write_all(&self.h.k.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        if file.write_all(&self.h.m.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        if file.write_all(&self.h.n_value.to_le_bytes()).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        // Write the bit array. C writes bf->M * sizeof(uint64_t) in one chunk.
        let mut v_bytes: Vec<u8> = Vec::with_capacity(self.v.len() * 8);
        for &word in &self.v {
            v_bytes.extend_from_slice(&word.to_le_bytes());
        }
        if file.write_all(&v_bytes).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        // Data, may be empty
        let _ = file.write_all(&self.data);
        1
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let ln_p = p.ln();
        let ln_2 = 2.0f64.ln();
        let m_f = ((n as f64) * ln_p / ln_2.powi(2)).ceil().abs();
        let m_val = m_f as u64;
        let k_val = ((ln_2 * (m_val as f64)) / (n as f64)).ceil() as u64;

        let big_m = ((m_val as f64) / 64.0).ceil() as u64;
        let v = vec![0u64; big_m as usize];

        let header = Header {
            version: 1,
            n,
            p,
            k: k_val,
            m: m_val,
            n_value: 0,
        };

        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: header,
            m: big_m,
            v,
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }

    pub fn from_file(mut file: File) -> BloomFilter {
        // Allocate a default-error filter to return on failure paths.
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

        // Read header (48 bytes).
        let mut header_bytes = [0u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
            eprintln!("[ERROR] reading filter file.");
            bf.error = 1;
            return bf;
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
            eprintln!("[ERROR] Incoherent header.");
            bf.error = 1;
            return bf;
        }

        let big_m: u64 = ((h.m as f64) / 64.0).ceil() as u64;
        bf.m = big_m;

        // Get total file size by seeking.
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                bf.error = 1;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            bf.error = 1;
            return bf;
        }

        // Mirror the (somewhat suspicious) C check.
        if (big_m as i64) <= (size - 48) {
            // datasize = size - ceil((M*64)/8) - 48 = size - M*8 - 48
            bf.datasize =
                ((size as i64) - (((big_m as f64 * 64.0) / 8.0).ceil() as i64) - 48) as u64;

            // Load bitarray
            let bit_bytes_len = (big_m as usize) * 8;
            let mut bit_bytes = vec![0u8; bit_bytes_len];
            if file.read_exact(&mut bit_bytes).is_err() {
                eprintln!("[ERROR] Cannot load bitarray.");
                bf.error = 1;
                return bf;
            }
            let mut v: Vec<u64> = Vec::with_capacity(big_m as usize);
            for i in 0..(big_m as usize) {
                v.push(u64::from_le_bytes(
                    bit_bytes[i * 8..i * 8 + 8].try_into().unwrap(),
                ));
            }
            bf.v = v;

            // Load remaining data
            if bf.datasize > 0 {
                let mut data_buf = vec![0u8; bf.datasize as usize];
                if file.read_exact(&mut data_buf).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    bf.error = 1;
                    return bf;
                }
                bf.data = data_buf;
            } else {
                bf.data = Vec::new();
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
            eprintln!("[ERROR] Current filter version not supported.");
            return false;
        }
        // 0x7FFFFFFFFFFFFFFF
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

        let ln_2 = 2.0f64.ln();
        let tmp_m_f = ((self.n as f64) * self.p.ln() / ln_2.powi(2)).ceil().abs();
        let tmp_m = tmp_m_f as u64;
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = ((ln_2 * (self.m as f64)) / (self.n as f64)).ceil() as u64;
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}
