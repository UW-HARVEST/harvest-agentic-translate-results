use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{fmt};

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
        let data_str = String::from_utf8_lossy(&self.data);
        write!(
            f,
            "Filter details:\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}

fn fnv1_hash(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        h ^= b as u64;
    }
    h
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let h = fnv1_hash(buf);
        let mut hn: u64 = h % M;
        let mut tmp: Vec<u64> = Vec::with_capacity(self.h.k as usize);
        for _ in 0..self.h.k {
            // (hn * G) mod M, using u128 to avoid overflow
            hn = ((hn as u128 * G as u128) % (M as u128)) as u64;
            tmp.push(hn % self.h.m);
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        if (self.h.n_value + 1) <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = false;
            for i in 0..self.h.k as usize {
                let k = (fp[i] / 64) as usize;
                let l = fp[i] % 64;
                let v: u64 = 1u64 << l;
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
        } else {
            -1
        }
    }

    pub fn join(&mut self, bf: &BloomFilter) -> i32 {
        // self is destination (dst), bf is source (src)
        let src = bf;
        let dst_n = self.h.n;
        let dst_p = self.h.p;
        let dst_k = self.h.k;
        let dst_m_field = self.h.m;
        let dst_big_m = self.m;

        if src.h.n != dst_n {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if src.h.p != dst_p {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if src.h.k != dst_k {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if src.h.m != dst_m_field {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if src.m != dst_big_m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if (self.h.n_value + src.h.n_value) > dst_n {
            eprintln!("[ERROR] Destination Filter is full.");
            return -1;
        }
        for i in 0..self.m as usize {
            self.v[i] |= src.v[i];
        }
        self.h.n_value += src.h.n_value;
        1
    }

    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for i in 0..self.h.k as usize {
            let k = (fp[i] / 64) as usize;
            let l = fp[i] % 64;
            let v: u64 = 1u64 << l;
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

    pub fn to_file(&self, file: File) -> i32 {
        let mut file = file;
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
        // Write bit array
        let mut bytes: Vec<u8> = Vec::with_capacity(self.m as usize * 8);
        for &x in self.v.iter().take(self.m as usize) {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        if file.write_all(&bytes).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        // Write data (can be empty)
        let datasize = self.datasize as usize;
        if datasize > 0 && self.data.len() >= datasize {
            let _ = file.write_all(&self.data[..datasize]);
        } else if datasize > 0 {
            // pad with zeros if not enough data
            let mut buf = self.data.clone();
            buf.resize(datasize, 0);
            let _ = file.write_all(&buf);
        }
        1
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_f = (n as f64) * p.ln() / 2.0_f64.ln().powi(2);
        let m = m_f.ceil().abs() as u64;
        let k = (2.0_f64.ln() * (m as f64) / (n as f64)).ceil() as u64;
        let big_m = ((m as f64) / 64.0).ceil() as u64;
        let v: Vec<u64> = vec![0u64; big_m as usize];
        let h = Header {
            version: 1,
            n,
            p,
            k,
            m,
            n_value: 0,
        };
        BloomFilter {
            version: 0,
            datasize: buf.len() as u64,
            h,
            m: big_m,
            v,
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }

    pub fn from_file(file: File) -> BloomFilter {
        let mut file = file;
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

        // Header is 6 * 8 = 48 bytes
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

        bf.m = ((h.m as f64) / 64.0).ceil() as u64;

        // Get file size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s,
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

        if (bf.m as i64) <= (size as i64 - 48) {
            let bytes_for_v = (((bf.m * 64) as f64) / 8.0).ceil() as u64;
            bf.datasize = size.saturating_sub(bytes_for_v).saturating_sub(48);

            // Load bit array
            let mut v: Vec<u64> = vec![0u64; bf.m as usize];
            let mut buf = vec![0u8; (bf.m as usize) * 8];
            if file.read_exact(&mut buf).is_err() {
                eprintln!("[ERROR] Cannot load bitarray.");
                bf.error = 1;
                return bf;
            }
            for i in 0..bf.m as usize {
                v[i] = u64::from_le_bytes(buf[i * 8..i * 8 + 8].try_into().unwrap());
            }
            bf.v = v;

            if bf.datasize > 0 {
                let mut data = vec![0u8; bf.datasize as usize];
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
        let tmp_m_f = (self.n as f64) * self.p.ln() / 2.0_f64.ln().powi(2);
        let tmp_m = tmp_m_f.ceil().abs() as u64;
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = (2.0_f64.ln() * (self.m as f64) / (self.n as f64)).ceil() as u64;
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}

// Suppress unused-constant warnings for mirrored constants from C source
#[allow(dead_code)]
const _UNUSED_FNV_PRIME: u64 = FNV_PRIME;
#[allow(dead_code)]
const _UNUSED_FNV_OFFSET_BASIS: u64 = FNV_OFFSET_BASIS;
