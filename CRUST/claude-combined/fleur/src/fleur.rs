use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

use crate::fnv::fnv1;

const M: u64 = 18446744073709551557;
const G: u64 = 18446744073709550147;
#[allow(dead_code)]
const FNV_PRIME: u64 = 1099511628211;
#[allow(dead_code)]
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
        let data_str = String::from_utf8_lossy(&self.data);
        write!(
            f,
            "Filter details:\n n: {} \n p: {}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut tmp = vec![0u64; self.h.k as usize];
        let h = fnv1(buf);
        // Match C semantics: uint64_t multiplication wraps mod 2^64,
        // and % m then reduces the result. Use wrapping_mul to mirror C.
        let mut hn: u64 = h % M;
        for i in 0..self.h.k as usize {
            hn = hn.wrapping_mul(G) % M;
            tmp[i] = hn % self.h.m;
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        // Lazy-initialize derived fields if the user constructed the filter
        // directly without calling `initialize` (i.e., m/k/v not yet set).
        if self.h.m == 0 || self.h.k == 0 || self.v.is_empty() {
            let m_calc = ((self.h.n as f64) * self.h.p.ln()
                / (2.0_f64.ln().powf(2.0)))
            .ceil()
            .abs() as u64;
            let k = ((2.0_f64).ln() * (m_calc as f64) / (self.h.n as f64)).ceil() as u64;
            let big_m = ((m_calc as f64) / 64.0).ceil() as u64;
            self.h.m = m_calc;
            self.h.k = k;
            self.m = big_m;
            self.v = vec![0u64; big_m as usize];
        }
        if (self.h.n_value + 1) <= self.h.n {
            let mut new_value = 0i32;
            let fp = self.fingerprint(buf);
            for i in 0..self.h.k as usize {
                let k = (fp[i] / 64) as usize;
                let l = fp[i] % 64;
                let v: u64 = 1u64 << l;
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
        // self is destination; bf is source
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

    pub fn to_file(&self, mut file: File) -> i32 {
        // Write header fields then bit array then data
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
        // bit array
        for i in 0..self.m as usize {
            if file.write_all(&self.v[i].to_le_bytes()).is_err() {
                eprintln!("[ERROR] writing filter file.");
                return 0;
            }
        }
        // data
        if !self.data.is_empty() {
            if file.write_all(&self.data).is_err() {
                eprintln!("[ERROR] writing filter file.");
                return 0;
            }
        }
        1
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_calc = ((n as f64) * p.ln() / (2.0_f64.ln().powf(2.0))).ceil().abs() as u64;
        let k = ((2.0_f64).ln() * (m_calc as f64) / (n as f64)).ceil() as u64;
        let big_m = ((m_calc as f64) / 64.0).ceil() as u64;
        let v = vec![0u64; big_m as usize];
        let h = Header {
            version: 1,
            n,
            p,
            k,
            m: m_calc,
            n_value: 0,
        };
        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h,
            m: big_m,
            v,
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }

    pub fn from_file(mut file: File) -> BloomFilter {
        // Read header (48 bytes): version (u64), n (u64), p (f64), k (u64), m (u64), n_value (u64)
        let mut header_bytes = [0u8; 48];
        let header_read = file.read_exact(&mut header_bytes).is_ok();
        if !header_read {
            eprintln!("[ERROR] reading filter file.");
            return error_bloom_filter();
        }

        let version = u64::from_le_bytes(header_bytes[0..8].try_into().unwrap());
        let n = u64::from_le_bytes(header_bytes[8..16].try_into().unwrap());
        let p = f64::from_le_bytes(header_bytes[16..24].try_into().unwrap());
        let k = u64::from_le_bytes(header_bytes[24..32].try_into().unwrap());
        let m_val = u64::from_le_bytes(header_bytes[32..40].try_into().unwrap());
        let n_value = u64::from_le_bytes(header_bytes[40..48].try_into().unwrap());

        let h = Header {
            version,
            n,
            p,
            k,
            m: m_val,
            n_value,
        };

        if !h.check() {
            eprintln!("[ERROR] Incoherent header.");
            let mut bf = error_bloom_filter();
            bf.h = h;
            return bf;
        }

        let big_m: u64 = ((m_val as f64) / 64.0).ceil() as u64;

        // Determine total file size
        let total_size = match file.seek(SeekFrom::End(0)) {
            Ok(sz) => sz as i64,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                let mut bf = error_bloom_filter();
                bf.h = h;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            let mut bf = error_bloom_filter();
            bf.h = h;
            return bf;
        }

        let mut datasize: u64 = 0;
        let mut v: Vec<u64> = Vec::new();
        let mut data: Vec<u8> = Vec::new();

        if (big_m as i64) <= (total_size - 48) {
            // bit array bytes = big_m * 8
            let bitarray_bytes = (big_m as usize) * 8;
            datasize = (total_size - (((big_m as f64) * 64.0 / 8.0).ceil() as i64) - 48) as u64;
            let mut buf = vec![0u8; bitarray_bytes];
            if file.read_exact(&mut buf).is_err() {
                eprintln!("[ERROR] Cannot load bitarray.");
                let mut bf = error_bloom_filter();
                bf.h = h;
                return bf;
            }
            v = (0..big_m as usize)
                .map(|i| {
                    let s = i * 8;
                    u64::from_le_bytes(buf[s..s + 8].try_into().unwrap())
                })
                .collect();

            if datasize > 0 {
                let mut data_buf = vec![0u8; datasize as usize];
                if file.read_exact(&mut data_buf).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    let mut bf = error_bloom_filter();
                    bf.h = h;
                    return bf;
                }
                data = data_buf;
            }
        }

        BloomFilter {
            version: 1,
            datasize,
            h,
            m: big_m,
            v,
            data,
            modified: 0,
            error: 0,
        }
    }
}

fn error_bloom_filter() -> BloomFilter {
    BloomFilter {
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
    }
}

impl Header {
    pub fn check(&self) -> bool {
        if self.version != 1 {
            eprintln!("[ERROR] Current filter version not supported.");
            return false;
        }
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
        let tmp_m = ((self.n as f64) * self.p.ln() / (2.0_f64.ln().powf(2.0)))
            .ceil()
            .abs() as u64;
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = ((2.0_f64).ln() * (self.m as f64) / (self.n as f64)).ceil() as u64;
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}
