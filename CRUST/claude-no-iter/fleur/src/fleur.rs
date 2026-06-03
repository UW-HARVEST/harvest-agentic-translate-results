use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
const M: u64 = 18446744073709551557;
const G: u64 = 18446744073709550147;
const FNV_PRIME: u64 = 1099511628211;
const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
#[repr(C)]
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

fn compute_m(n: u64, p: f64) -> u64 {
    // m = fabs(ceil(n * log(p) / log(2)^2))
    ((n as f64) * p.ln() / 2f64.ln().powi(2)).ceil().abs() as u64
}

fn compute_k(m: u64, n: u64) -> u64 {
    // k = ceil(log(2) * m / n)
    (2f64.ln() * (m as f64) / (n as f64)).ceil() as u64
}

fn fnv1_hash(buf: &[u8]) -> u64 {
    // Match C: signed char extended through int to uint64_t
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}

impl BloomFilter {
    fn ensure_initialized(&mut self) {
        // Lazily initialize parameters/bit-array if a BloomFilter was constructed
        // by hand without going through `initialize`.
        if self.h.k == 0 || self.h.m == 0 || self.m == 0 || self.v.is_empty() {
            let n = self.h.n;
            let p = self.h.p;
            if n == 0 || p <= 0.0 {
                return;
            }
            let m = compute_m(n, p);
            let k = compute_k(m, n);
            self.h.k = k;
            self.h.m = m;
            self.m = ((m as f64) / 64.0).ceil() as u64;
            self.v = vec![0u64; self.m as usize];
        }
    }

    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut tmp = Vec::with_capacity(self.h.k as usize);
        let h = fnv1_hash(buf);
        let mut hn = h % M;
        for _ in 0..self.h.k {
            hn = hn.wrapping_mul(G) % M;
            if self.h.m == 0 {
                tmp.push(hn);
            } else {
                tmp.push(hn % self.h.m);
            }
        }
        tmp
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        self.ensure_initialized();
        if (self.h.n_value + 1) <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = 0i32;
            for &i in &fp {
                let k = (i / 64) as usize;
                let l = i % 64;
                let v = 1u64 << l;
                if k < self.v.len() {
                    if (self.v[k] & v) == 0 {
                        new_value = 1;
                    }
                    self.v[k] |= v;
                }
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
        // self is the destination filter, `bf` is the source filter.
        if self.h.n != bf.h.n {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if self.h.p != bf.h.p {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if self.h.k != bf.h.k {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if self.h.m != bf.h.m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if self.m != bf.m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if (self.h.n_value + bf.h.n_value) > self.h.n {
            eprintln!("[ERROR] Destination Filter is full.");
            return -1;
        }
        for i in 0..(self.m as usize) {
            self.v[i] |= bf.v[i];
        }
        self.h.n_value += bf.h.n_value;
        1
    }
    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for &i in &fp {
            let k = (i / 64) as usize;
            let l = i % 64;
            let v = 1u64 << l;
            if k >= self.v.len() {
                return 0;
            }
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
        let result: std::io::Result<()> = (|| {
            // Serialization version
            file.write_all(&1u64.to_le_bytes())?;
            file.write_all(&self.h.n.to_le_bytes())?;
            file.write_all(&self.h.p.to_le_bytes())?;
            file.write_all(&self.h.k.to_le_bytes())?;
            file.write_all(&self.h.m.to_le_bytes())?;
            file.write_all(&self.h.n_value.to_le_bytes())?;
            for &val in &self.v {
                file.write_all(&val.to_le_bytes())?;
            }
            let datasize = self.datasize as usize;
            if datasize > 0 {
                let n = std::cmp::min(datasize, self.data.len());
                file.write_all(&self.data[..n])?;
            }
            file.flush()?;
            Ok(())
        })();
        match result {
            Ok(()) => 1,
            Err(_) => {
                eprintln!("[ERROR] writing filter file.");
                0
            }
        }
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_bits = compute_m(n, p);
        let k = compute_k(m_bits, n);
        let big_m = ((m_bits as f64) / 64.0).ceil() as u64;
        let v = vec![0u64; big_m as usize];
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
            v,
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(mut file: File) -> BloomFilter {
        let make_empty = || BloomFilter {
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

        // Read header (48 bytes)
        let mut header_bytes = [0u8; 48];
        if file.read_exact(&mut header_bytes).is_err() {
            eprintln!("[ERROR] reading filter file.");
            return make_empty();
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
            let mut bf = make_empty();
            bf.h = h;
            return bf;
        }

        let big_m = ((h.m as f64) / 64.0).ceil() as u64;

        // Get total file size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                let mut bf = make_empty();
                bf.h = h;
                return bf;
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            let mut bf = make_empty();
            bf.h = h;
            return bf;
        }

        let mut v: Vec<u64> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        let mut datasize: u64 = 0;

        if (big_m as i64) <= (size - 48) {
            datasize = (size - (big_m as i64) * 8 - 48).max(0) as u64;

            // Load bit array
            v = vec![0u64; big_m as usize];
            for slot in v.iter_mut() {
                let mut bytes = [0u8; 8];
                if file.read_exact(&mut bytes).is_err() {
                    eprintln!("[ERROR] Cannot load bitarray.");
                    let mut bf = make_empty();
                    bf.h = h;
                    return bf;
                }
                *slot = u64::from_le_bytes(bytes);
            }

            // Load remaining data
            if datasize > 0 {
                data = vec![0u8; datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    let mut bf = make_empty();
                    bf.h = h;
                    return bf;
                }
            }
        }

        BloomFilter {
            version: h.version,
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
