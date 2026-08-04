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
        let mut tmp = vec![0u64; self.h.k as usize];
        let h = fnv1_local(buf);
        let mut hn = h % M;
        for i in 0..self.h.k as usize {
            // C uses wrapping 64-bit multiplication, then takes the result mod M.
            hn = hn.wrapping_mul(G) % M;
            tmp[i] = hn % self.h.m;
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        if self.h.n_value + 1 <= self.h.n {
            let mut new_value = 0;
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
        // self == dst, bf == src
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
        let mut buf = Vec::with_capacity(self.v.len() * 8);
        for &x in &self.v {
            buf.extend_from_slice(&x.to_le_bytes());
        }
        if file.write_all(&buf).is_err() {
            eprintln!("[ERROR] writing filter file.");
            return 0;
        }
        // Data can be empty
        if self.datasize > 0 {
            let _ = file.write_all(&self.data[..self.datasize as usize]);
        }
        1
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_calc = (((n as f64) * p.ln() / (2.0_f64.ln().powi(2))).ceil()).abs() as u64;
        let k_calc = ((2.0_f64.ln() * (m_calc as f64) / (n as f64)).ceil()) as u64;
        let m_size = ((m_calc as f64) / 64.0).ceil() as u64;
        let v = vec![0u64; m_size as usize];
        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: Header {
                version: 1,
                n,
                p,
                k: k_calc,
                m: m_calc,
                n_value: 0,
            },
            m: m_size,
            v,
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

        // Read 48 bytes for header
        let mut header_buf = [0u8; 48];
        if file.read_exact(&mut header_buf).is_err() {
            eprintln!("[ERROR] reading filter file.");
            bf.error = 1;
            return bf;
        }
        let h = Header {
            version: u64::from_le_bytes(header_buf[0..8].try_into().unwrap()),
            n: u64::from_le_bytes(header_buf[8..16].try_into().unwrap()),
            p: f64::from_le_bytes(header_buf[16..24].try_into().unwrap()),
            k: u64::from_le_bytes(header_buf[24..32].try_into().unwrap()),
            m: u64::from_le_bytes(header_buf[32..40].try_into().unwrap()),
            n_value: u64::from_le_bytes(header_buf[40..48].try_into().unwrap()),
        };

        if !h.check() {
            eprintln!("[ERROR] Incoherent header.");
            bf.error = 1;
            bf.h = h;
            return bf;
        }

        bf.m = ((h.m as f64) / 64.0).ceil() as u64;

        // Get total file size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                bf.error = 1;
                bf.h = h;
                return bf;
            }
        };
        let _ = file.seek(SeekFrom::Start(48));

        if bf.m as i64 <= (size as i64 - 48) {
            let bits_bytes = (((bf.m * 64) as f64) / 8.0).ceil() as u64;
            let datasize = (size as i64) - (bits_bytes as i64) - 48;
            let datasize = if datasize < 0 { 0 } else { datasize as u64 };
            bf.datasize = datasize;

            // Load bit array
            let mut v = vec![0u64; bf.m as usize];
            let mut bytes = vec![0u8; bf.m as usize * 8];
            if file.read_exact(&mut bytes).is_err() {
                eprintln!("[ERROR] Cannot load bitarray.");
                bf.error = 1;
                bf.h = h;
                return bf;
            }
            for i in 0..bf.m as usize {
                v[i] = u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap());
            }
            bf.v = v;

            if bf.datasize > 0 {
                let mut data = vec![0u8; bf.datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    bf.error = 1;
                    bf.h = h;
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
        let tmp_m = ((((self.n as f64) * self.p.ln() / (2.0_f64.ln().powi(2))).ceil()).abs()) as u64;
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = ((2.0_f64.ln() * (self.m as f64) / (self.n as f64)).ceil()) as u64;
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}

// Local FNV-1 implementation (matches the C version with signed char promotion)
fn fnv1_local(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        let signed_b = b as i8 as i64 as u64;
        h ^= signed_b;
    }
    h
}

