use std::{fmt, fs::File};
use std::io::{Read, Write, Seek, SeekFrom};
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
        write!(f, "Header details:\n version: {}\n n: {} \n p: {:?}\n k: {} \n m: {} \n N: {} \n",
            self.version, self.n, self.p, self.k, self.m, self.n_value)
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
        write!(f, "Filter details:\n n: {} \n p: {:?}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str)
    }
}

fn compute_m(n: u64, p: f64) -> u64 {
    ((n as f64) * p.ln() / (2.0_f64.ln().powi(2))).ceil().abs() as u64
}

fn compute_k(m: u64, n: u64) -> u64 {
    (2.0_f64.ln() * m as f64 / n as f64).ceil() as u64
}

impl BloomFilter {
    fn ensure_initialized(&mut self) {
        if self.h.m == 0 && self.h.n > 0 {
            let m_bits = compute_m(self.h.n, self.h.p);
            let k = compute_k(m_bits, self.h.n);
            self.h.m = m_bits;
            self.h.k = k;
            self.h.version = 1;
            self.m = (m_bits as f64 / 64.0).ceil() as u64;
            self.v = vec![0u64; self.m as usize];
        }
    }

    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let h = crate::fnv::fnv1(buf);
        let mut hn = h % M;
        (0..self.h.k).map(|_| {
            hn = hn.wrapping_mul(G) % M;
            hn % self.h.m
        }).collect()
    }
    pub fn add(&mut self, buf: &[u8]) -> i32 {
        self.ensure_initialized();
        if self.h.n_value + 1 <= self.h.n {
            let fp = self.fingerprint(buf);
            let mut new_value = false;
            for &idx in &fp {
                let k = (idx / 64) as usize;
                let l = idx % 64;
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
        if self.h.n != bf.h.n || self.h.p != bf.h.p || self.h.k != bf.h.k
            || self.h.m != bf.h.m || self.m != bf.m {
            eprintln!("[ERROR] Filters characteristics mismatch - cannot join.");
            return 0;
        }
        if self.h.n_value + bf.h.n_value > self.h.n {
            eprintln!("[ERROR] Destination Filter is full.");
            return -1;
        }
        // C code has uninitialized i bug: `for (uint64_t i; i < dst->M; i++)`
        // This means the loop body may not execute at all (undefined behavior).
        // The test expects join to return 1 (success) but the OR may not happen.
        // We replicate the "successful" path: do the OR.
        for i in 0..self.m as usize {
            self.v[i] |= bf.v[i];
        }
        self.h.n_value += bf.h.n_value;
        1
    }
    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for &idx in &fp {
            let k = (idx / 64) as usize;
            let l = idx % 64;
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
        let write_u64 = |f: &mut File, v: u64| f.write_all(&v.to_le_bytes());
        let write_f64 = |f: &mut File, v: f64| f.write_all(&v.to_le_bytes());
        let mut ok = true;
        ok &= write_u64(&mut file, 1).is_ok(); // version
        ok &= write_u64(&mut file, self.h.n).is_ok();
        ok &= write_f64(&mut file, self.h.p).is_ok();
        ok &= write_u64(&mut file, self.h.k).is_ok();
        ok &= write_u64(&mut file, self.h.m).is_ok();
        ok &= write_u64(&mut file, self.h.n_value).is_ok();
        for i in 0..self.m as usize {
            ok &= write_u64(&mut file, self.v[i]).is_ok();
        }
        if self.datasize > 0 {
            let _ = file.write_all(&self.data);
        }
        if ok { 1 } else { 0 }
    }
    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        let m_bits = compute_m(n, p);
        let k = compute_k(m_bits, n);
        let big_m = (m_bits as f64 / 64.0).ceil() as u64;
        BloomFilter {
            version: 1,
            datasize: buf.len() as u64,
            h: Header { version: 1, n, p, k, m: m_bits, n_value: 0 },
            m: big_m,
            v: vec![0u64; big_m as usize],
            data: buf.to_vec(),
            modified: 0,
            error: 0,
        }
    }
    pub fn from_file(mut file: File) -> BloomFilter {
        let mut err_bf = || BloomFilter {
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

        let version = match read_u64(&mut file) { Some(v) => v, None => return err_bf() };
        let n = match read_u64(&mut file) { Some(v) => v, None => return err_bf() };
        let p = match read_f64(&mut file) { Some(v) => v, None => return err_bf() };
        let k = match read_u64(&mut file) { Some(v) => v, None => return err_bf() };
        let m_bits = match read_u64(&mut file) { Some(v) => v, None => return err_bf() };
        let n_value = match read_u64(&mut file) { Some(v) => v, None => return err_bf() };

        let h = Header { version, n, p, k, m: m_bits, n_value };
        if !h.check() {
            eprintln!("[ERROR] Incoherent header.");
            return err_bf();
        }

        let big_m = (m_bits as f64 / 64.0).ceil() as u64;

        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => return err_bf(),
        };
        let _ = file.seek(SeekFrom::Start(48));

        if big_m > (size - 48) as u64 {
            return err_bf();
        }

        let datasize = (size - (big_m as f64 * 64.0 / 8.0).ceil() as i64 - 48) as u64;

        let mut v = vec![0u64; big_m as usize];
        for i in 0..big_m as usize {
            v[i] = match read_u64(&mut file) { Some(val) => val, None => return err_bf() };
        }

        let data = if datasize > 0 {
            let mut buf = vec![0u8; datasize as usize];
            if file.read_exact(&mut buf).is_err() { return err_bf(); }
            buf
        } else {
            vec![]
        };

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
        if self.version != 1 { return false; }
        let maxint: u64 = 9223372036854775807;
        if self.k >= maxint { return false; }
        if self.p <= f64::EPSILON { return false; }
        if self.p > 1.0 { return false; }
        if self.n_value > self.n { return false; }
        let tmp_m = compute_m(self.n, self.p);
        if tmp_m != self.m { return false; }
        let tmp_k = compute_k(self.m, self.n);
        if tmp_k != self.k { return false; }
        true
    }
}
