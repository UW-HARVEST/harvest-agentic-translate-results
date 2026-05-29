use std::io::{Read, Seek, SeekFrom, Write};
use std::{fmt, fs::File};
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
        // Match C's print_filter using the bytes of `data` as a string
        let data_str = String::from_utf8_lossy(&self.data);
        write!(
            f,
            "Filter details:\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}
impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut tmp: Vec<u64> = vec![0u64; self.h.k as usize];
        let h = fnv1(buf);
        let mut hn: u64 = h % M;
        for i in 0..self.h.k as usize {
            hn = hn.wrapping_mul(G) % M;
            tmp[i] = hn % self.h.m;
        }
        tmp
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
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
        // Match C's serialization order: version, h.n, h.p, h.k, h.m, h.N, then v array, then Data.
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
        // Bit array
        let mut buf: Vec<u8> = Vec::with_capacity(self.m as usize * 8);
        for &word in &self.v {
            buf.extend_from_slice(&word.to_le_bytes());
        }
        if file.write_all(&buf).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        // Data (may be empty)
        if self.datasize > 0 {
            let n = self.datasize as usize;
            let slice = if self.data.len() >= n {
                &self.data[..n]
            } else {
                &self.data[..]
            };
            if file.write_all(slice).is_err() {
                eprintln!("[ERROR] writing filter file.");
                return 0;
            }
        }
        1
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        // m = |ceil(n * ln(p) / ln(2)^2)|
        let m_calc = ((n as f64) * p.ln() / (2.0_f64.ln().powi(2))).ceil().abs();
        let m_bits = m_calc as u64;
        let k = (2.0_f64.ln() * (m_bits as f64) / (n as f64)).ceil() as u64;
        let m_words = (m_bits as f64 / 64.0).ceil() as u64;

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
            m: m_words,
            v: vec![0u64; m_words as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(mut file: File) -> BloomFilter {
        // Read 48-byte header
        let mut header_bytes = [0u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
            eprintln!("[ERROR] reading filter file.");
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
            eprintln!("[ERROR] Incoherent header.");
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

        let m_words = (h.m as f64 / 64.0).ceil() as u64;

        // Get total file size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                return BloomFilter {
                    version: 0,
                    datasize: 0,
                    h,
                    m: m_words,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                };
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            return BloomFilter {
                version: 0,
                datasize: 0,
                h,
                m: m_words,
                v: Vec::new(),
                data: Vec::new(),
                modified: 0,
                error: 1,
            };
        }

        let size_i = size as i64;
        let mut datasize: u64 = 0;
        let mut v_vec: Vec<u64> = Vec::new();
        let mut data: Vec<u8> = Vec::new();

        if (m_words as i64) <= (size_i - 48) {
            datasize = (size_i - (m_words as i64) * 8 - 48).max(0) as u64;

            // Load bit array
            let mut v_bytes = vec![0u8; m_words as usize * 8];
            if file.read_exact(&mut v_bytes).is_err() && m_words > 0 {
                eprintln!("[ERROR] Cannot load bitarray.");
                return BloomFilter {
                    version: 0,
                    datasize: 0,
                    h,
                    m: m_words,
                    v: Vec::new(),
                    data: Vec::new(),
                    modified: 0,
                    error: 1,
                };
            }
            v_vec = Vec::with_capacity(m_words as usize);
            for chunk in v_bytes.chunks_exact(8) {
                v_vec.push(u64::from_le_bytes(chunk.try_into().unwrap()));
            }

            if datasize > 0 {
                let mut data_bytes = vec![0u8; datasize as usize];
                if file.read_exact(&mut data_bytes).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    return BloomFilter {
                        version: 0,
                        datasize: 0,
                        h,
                        m: m_words,
                        v: Vec::new(),
                        data: Vec::new(),
                        modified: 0,
                        error: 1,
                    };
                }
                data = data_bytes;
            }
        }

        BloomFilter {
            version: 1,
            datasize,
            h,
            m: m_words,
            v: v_vec,
            data,
            modified: 0,
            error: 0,
        }
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
        let tmp_m_f = ((self.n as f64) * self.p.ln() / (2.0_f64.ln().powi(2)))
            .ceil()
            .abs();
        let tmp_m = tmp_m_f as u64;
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

fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // Match C's `h ^= buf[i]` where buf is char* (signed on most platforms),
        // sign-extended to uint64_t before XOR.
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}
