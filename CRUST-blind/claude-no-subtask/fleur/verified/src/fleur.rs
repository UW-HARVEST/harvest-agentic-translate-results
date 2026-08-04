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
        // Mirror the C printf in fleur_print_filter; data printed as a string.
        let data_str = String::from_utf8_lossy(&self.data);
        write!(
            f,
            "Filter details:\n n: {} \n p: {:.6}\n k: {} \n m: {} \n N: {} \n M: {}\n Data: {}.\n",
            self.h.n, self.h.p, self.h.k, self.h.m, self.h.n_value, self.m, data_str
        )
    }
}

// Internal helper: FNV-1 hash mirroring the C fnv1, with sign-extension of bytes
fn fnv1_internal(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}

impl BloomFilter {
    pub fn fingerprint(&self, buf: &[u8]) -> Vec<u64> {
        let mut tmp: Vec<u64> = Vec::with_capacity(self.h.k as usize);
        let h = fnv1_internal(buf);
        // C: hn = h % m; then hn = (hn * g) % m; tmp[i] = hn % bf->h.m
        // Need to use 128-bit arithmetic to avoid overflow on multiplication.
        let mut hn: u64 = h % M;
        for _ in 0..self.h.k {
            let prod = (hn as u128) * (G as u128);
            hn = (prod % (M as u128)) as u64;
            tmp.push(hn % self.h.m);
        }
        tmp
    }

    pub fn add(&mut self, buf: &[u8]) -> i32 {
        if (self.h.n_value + 1) <= self.h.n {
            let mut new_value = 0;
            let fp = self.fingerprint(buf);
            for i in 0..(self.h.k as usize) {
                let k = (fp[i] / 64) as usize;
                let l = fp[i] % 64;
                let v = 1u64 << l;
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
        // self is dst, bf is src (mirrors C signature fleur_join(src, dst))
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
        for i in 0..(self.m as usize) {
            self.v[i] |= bf.v[i];
        }
        self.h.n_value += bf.h.n_value;
        1
    }

    pub fn check(&self, buf: &[u8]) -> i32 {
        let fp = self.fingerprint(buf);
        for i in 0..(self.h.k as usize) {
            let k = (fp[i] / 64) as usize;
            let l = fp[i] % 64;
            let v = 1u64 << l;
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
        // Write header in same on-disk format as C struct header:
        //   version (u64 LE), n (u64 LE), p (f64 LE), k (u64 LE), m (u64 LE), N (u64 LE)
        // Then bitarray v (M * u64 LE), then data bytes.
        let mut version_field: u64 = 1;
        // The C function sets bf->version = 1 before writing. The header.version
        // is what is written, but the C code writes bf->version (which is the outer
        // BloomFilter struct's version field, NOT bf->h.version). Mirror that.
        // (However, since C's check_header expects header.version == 1, the on-disk
        // header version must be 1.)
        let _ = &mut version_field;
        let res: std::io::Result<()> = (|| {
            file.write_all(&1u64.to_le_bytes())?;
            file.write_all(&self.h.n.to_le_bytes())?;
            file.write_all(&self.h.p.to_le_bytes())?;
            file.write_all(&self.h.k.to_le_bytes())?;
            file.write_all(&self.h.m.to_le_bytes())?;
            file.write_all(&self.h.n_value.to_le_bytes())?;
            for &val in &self.v {
                file.write_all(&val.to_le_bytes())?;
            }
            if self.datasize > 0 {
                let len = self.datasize as usize;
                let take = len.min(self.data.len());
                file.write_all(&self.data[..take])?;
            }
            Ok(())
        })();
        match res {
            Ok(()) => 1,
            Err(_) => {
                eprintln!("[ERROR] writing filter file.");
                0
            }
        }
    }

    pub fn initialize(n: u64, p: f64, buf: &[u8]) -> BloomFilter {
        // m = abs(ceil(n * ln(p) / (ln 2)^2))
        let m_f = ((n as f64) * p.ln() / 2.0_f64.ln().powi(2)).ceil().abs();
        let m_val = m_f as u64;
        let k_val = (2.0_f64.ln() * (m_val as f64) / (n as f64)).ceil() as u64;
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
        // Default error filter
        let make_error = || BloomFilter {
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
        let mut hdr_buf = [0u8; 48];
        if file.read_exact(&mut hdr_buf).is_err() {
            eprintln!("[ERROR] reading filter file.");
            return make_error();
        }
        let h = Header {
            version: u64::from_le_bytes(hdr_buf[0..8].try_into().unwrap()),
            n: u64::from_le_bytes(hdr_buf[8..16].try_into().unwrap()),
            p: f64::from_le_bytes(hdr_buf[16..24].try_into().unwrap()),
            k: u64::from_le_bytes(hdr_buf[24..32].try_into().unwrap()),
            m: u64::from_le_bytes(hdr_buf[32..40].try_into().unwrap()),
            n_value: u64::from_le_bytes(hdr_buf[40..48].try_into().unwrap()),
        };

        if !h.check() {
            eprintln!("[ERROR] Incoherent header.");
            return make_error();
        }

        let big_m = ((h.m as f64) / 64.0).ceil() as u64;

        // get total size
        let size = match file.seek(SeekFrom::End(0)) {
            Ok(s) => s as i64,
            Err(_) => {
                eprintln!("[ERROR] Cannot seek in binary file.");
                return make_error();
            }
        };
        if file.seek(SeekFrom::Start(48)).is_err() {
            eprintln!("[ERROR] Cannot seek in binary file.");
            return make_error();
        }

        let mut v = Vec::new();
        let mut data = Vec::new();
        let mut datasize: u64 = 0;

        if (big_m as i64) <= (size - 48) {
            // datasize = size - ceil(M*64 / 8) - 48 = size - M*8 - 48
            let bitarray_bytes = (big_m * 8) as i64;
            datasize = (size - bitarray_bytes - 48) as u64;

            // load bitarray
            v = vec![0u64; big_m as usize];
            for slot in v.iter_mut() {
                let mut buf = [0u8; 8];
                if file.read_exact(&mut buf).is_err() {
                    eprintln!("[ERROR] Cannot load bitarray.");
                    return make_error();
                }
                *slot = u64::from_le_bytes(buf);
            }

            if datasize > 0 {
                data = vec![0u8; datasize as usize];
                if file.read_exact(&mut data).is_err() {
                    eprintln!("[ERROR] Cannot load bloom filter metadata.");
                    return make_error();
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
        let tmp_m_f = ((self.n as f64) * self.p.ln() / 2.0_f64.ln().powi(2))
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
