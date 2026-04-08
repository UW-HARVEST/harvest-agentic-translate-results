use std::{fmt, fs::File};
use std::io::{Read, Seek, SeekFrom, Write};
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
            "Header details:\n version: {}\n n: {} \n p: {:?}\n k: {} \n m: {} \n N: {} \n",
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
            "Filter details:\n n: {} \n p: {:?}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}
impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut tmp = vec![0u64; self.h.k as usize];
        let h = fnv1(buf);
        let mut hn = h % M;
        for i in 0..self.h.k as usize {
            hn = hn.wrapping_mul(G) % M;
            tmp[i] = hn % self.h.m;
        }
        tmp
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        if (self.h.n_value + 1) <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = false;
            for i in 0..self.h.k as usize {
                let k = fp[i] / 64;
                let l = fp[i] % 64;
                let v = 1u64 << l;
                if (self.v[k as usize] & v) == 0 {
                    new_value = true;
                }
                self.v[k as usize] |= v;
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
        if bf.h.n != self.h.n
            || bf.h.p != self.h.p
            || bf.h.k != self.h.k
            || bf.h.m != self.h.m
            || bf.m != self.m
        {
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
            let k = fp[i] / 64;
            let l = fp[i] % 64;
            let v = 1u64 << l;
            if (self.v[k as usize] & v) == 0 {
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
        let mut ok = true;
        ok = ok && file.write_all(&version.to_le_bytes()).is_ok();
        ok = ok && file.write_all(&self.h.n.to_le_bytes()).is_ok();
        ok = ok && file.write_all(&self.h.p.to_le_bytes()).is_ok();
        ok = ok && file.write_all(&self.h.k.to_le_bytes()).is_ok();
        ok = ok && file.write_all(&self.h.m.to_le_bytes()).is_ok();
        ok = ok && file.write_all(&self.h.n_value.to_le_bytes()).is_ok();
        for val in &self.v {
            ok = ok && file.write_all(&val.to_le_bytes()).is_ok();
        }
        let _ = file.write_all(&self.data);
        if !ok {
            eprintln!("[ERROR] writing filter file.");
            0
        } else {
            1
        }
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_bits = f64::abs(f64::ceil((n as f64) * f64::ln(p) / f64::powi(f64::ln(2.0), 2))) as u64;
        let k = f64::ceil(f64::ln(2.0) * m_bits as f64 / n as f64) as u64;
        let big_m = f64::ceil(m_bits as f64 / 64.0) as u64;
        BloomFilter {
            version: 1,
            datasize: 0,
            h: Header { version: 1, n, p, k, m: m_bits, n_value: 0 },
            m: big_m,
            v: vec![0u64; big_m as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(mut file: File) -> BloomFilter {
        let err_bf = || BloomFilter {
            version: 0, datasize: 0,
            h: Header { version: 0, n: 0, p: 0.0, k: 0, m: 0, n_value: 0 },
            m: 0, v: vec![], data: vec![], modified: 0, error: 1,
        };

        let read_u64 = |f: &mut File| -> Option<u64> {
            let mut buf = [0u8; 8];
            f.read_exact(&mut buf).ok()?;
            Some(u64::from_le_bytes(buf))
        };
        let read_f64 = |f: &mut File| -> Option<f64> {
            let mut buf = [0u8; 8];
            f.read_exact(&mut buf).ok()?;
            Some(f64::from_le_bytes(buf))
        };

        // Read header (48 bytes: 6 x u64, with p as f64)
        let version = match read_u64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };
        let n = match read_u64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };
        let p = match read_f64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };
        let k = match read_u64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };
        let m_bits = match read_u64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };
        let n_value = match read_u64(&mut file) { Some(v) => v, None => { eprintln!("[ERROR] reading filter file."); return err_bf(); } };

        let h = Header { version, n, p, k, m: m_bits, n_value };

        if !h.check() {
            eprintln!("[ERROR] Incoherent header.");
            return err_bf();
        }

        let big_m = f64::ceil(m_bits as f64 / 64.0) as u64;

        // Get file size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => { eprintln!("[ERROR] Cannot seek in binary file."); return err_bf(); }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            return err_bf();
        }

        if big_m <= (size - 48) as u64 {
            let datasize = (size - (f64::ceil((big_m as f64 * 64.0) / 8.0) as i64) - 48) as u64;

            // Read bit array
            let mut v = vec![0u64; big_m as usize];
            for val in v.iter_mut() {
                let mut buf = [0u8; 8];
                if file.read_exact(&mut buf).is_err() {
                    eprintln!("[ERROR] Cannot load bitarray.");
                    return err_bf();
                }
                *val = u64::from_le_bytes(buf);
            }

            // Read remaining data
            let data = if datasize > 0 {
                let mut d = vec![0u8; datasize as usize];
                if file.read_exact(&mut d).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    return err_bf();
                }
                d
            } else {
                vec![]
            };

            BloomFilter {
                version,
                datasize,
                h,
                m: big_m,
                v,
                data,
                modified: 0,
                error: 0,
            }
        } else {
            err_bf()
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
        let tmp_m = f64::abs(f64::ceil((self.n as f64) * f64::ln(self.p) / f64::powi(f64::ln(2.0), 2))) as u64;
        if tmp_m != self.m {
            eprintln!("[ERROR] incoherent filter.");
            return false;
        }
        let tmp_k = f64::ceil(f64::ln(2.0) * self.m as f64 / self.n as f64) as u64;
        if tmp_k != self.k {
            eprintln!("[ERROR] incoherent filter values.");
            return false;
        }
        true
    }
}

fn fnv1(buf: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        h ^= b as u64;
    }
    h
}
