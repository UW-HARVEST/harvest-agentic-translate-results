use std::fs::File;
use std::io::{Read, Write};

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
}

const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

#[inline]
fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, 32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, 32);
}

fn u8to64_le(p: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = p.len().min(8);
    buf[..n].copy_from_slice(&p[..n]);
    u64::from_le_bytes(buf)
}

fn siphash_full(data: &[u8]) -> [u8; 16] {
    let k0 = u8to64_le(&SECRET_KEY[0..8]);
    let k1 = u8to64_le(&SECRET_KEY[8..16]);
    let mut v0 = 0x736f6d6570736575u64 ^ k0;
    let mut v1 = (0x646f72616e646f6du64 ^ k1) ^ 0xee;
    let mut v2 = 0x6c7967656e657261u64 ^ k0;
    let mut v3 = 0x7465646279746573u64 ^ k1;

    let inlen = data.len() as u64;
    let mut idx = 0usize;
    while idx + 8 <= data.len() {
        let m = u8to64_le(&data[idx..idx + 8]);
        v3 ^= m;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
        idx += 8;
    }
    let left = data.len() - idx;
    let mut b: u64 = inlen << 56;
    for i in 0..left {
        b |= (data[idx + i] as u64) << (8 * i);
    }

    v3 ^= b;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= b;
    v2 ^= 0xee;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let mut out = [0u8; 16];
    let bb = v0 ^ v1 ^ v2 ^ v3;
    out[0..8].copy_from_slice(&bb.to_le_bytes());
    v1 ^= 0xdd;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let bb = v0 ^ v1 ^ v2 ^ v3;
    out[8..16].copy_from_slice(&bb.to_le_bytes());
    out
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        let _ = self.f_out.flush();
    }

    pub fn output_literal_buffer(&self) {
        // No-op: state is local to tp_parse in this Rust port.
    }

    pub fn append_integer(&self) {
        // No-op: state is local to tp_parse in this Rust port.
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = match File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                crate::trusted_utils::trusted_utils_exit_eof();
                unreachable!()
            }
        };
        TrustedParser { f_out: out, f }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        let mut comment = false;
        let mut header = false;
        let mut input_finished = false;
        let input_invalid = false;
        let mut began_num = false;

        let mut num: i32 = 0;
        let mut sign_v: i32 = 1;
        let mut nb_read_cls: i32 = 0;

        let mut nb_vars: i32 = -1;
        let mut nb_cls: i32 = -1;

        let mut data: Vec<i32> = Vec::with_capacity(1 << 14);
        let max_buf_size: usize = 1 << 14;
        // Collect all integers' little-endian bytes for hashing.
        let mut hash_bytes: Vec<u8> = Vec::new();

        // Read all input
        let mut content = Vec::new();
        if self.f.read_to_end(&mut content).is_err() {
            // ignored
        }

        let flush_buffer =
            |data: &mut Vec<i32>, hash_bytes: &mut Vec<u8>, f_out: &mut File| {
                for v in data.iter() {
                    hash_bytes.extend_from_slice(&v.to_le_bytes());
                    let _ = f_out.write_all(&v.to_le_bytes());
                }
                data.clear();
            };

        let append_int =
            |num: &mut i32,
             sign_v: &mut i32,
             nb_read_cls: &mut i32,
             nb_vars: &mut i32,
             nb_cls: &mut i32,
             header: &mut bool,
             began_num: &mut bool,
             data: &mut Vec<i32>,
             hash_bytes: &mut Vec<u8>,
             f_out: &mut File| {
                if *header {
                    if *nb_vars == -1 {
                        *nb_vars = *num;
                        let _ = f_out.write_all(&nb_vars.to_le_bytes());
                    } else if *nb_cls == -1 {
                        *nb_cls = *num;
                        let _ = f_out.write_all(&nb_cls.to_le_bytes());
                        *header = false;
                    }
                    *num = 0;
                    *began_num = false;
                    return;
                }
                let lit = *sign_v * *num;
                if lit == 0 {
                    *nb_read_cls += 1;
                }
                *num = 0;
                *sign_v = 1;
                *began_num = false;
                if data.len() >= max_buf_size {
                    flush_buffer(data, hash_bytes, f_out);
                }
                data.push(lit);
            };

        for &c in content.iter() {
            if comment && c != b'\n' && c != b'\r' {
                continue;
            }
            match c {
                b'\n' | b'\r' => {
                    comment = false;
                    if began_num {
                        append_int(
                            &mut num,
                            &mut sign_v,
                            &mut nb_read_cls,
                            &mut nb_vars,
                            &mut nb_cls,
                            &mut header,
                            &mut began_num,
                            &mut data,
                            &mut hash_bytes,
                            &mut self.f_out,
                        );
                    }
                }
                b'p' => {
                    header = true;
                }
                b'c' => {
                    if !header {
                        comment = true;
                    }
                }
                b' ' => {
                    if began_num {
                        append_int(
                            &mut num,
                            &mut sign_v,
                            &mut nb_read_cls,
                            &mut nb_vars,
                            &mut nb_cls,
                            &mut header,
                            &mut began_num,
                            &mut data,
                            &mut hash_bytes,
                            &mut self.f_out,
                        );
                    }
                }
                b'-' => {
                    sign_v = -1;
                    began_num = true;
                }
                b'0'..=b'9' => {
                    num = num * 10 + (c - b'0') as i32;
                    began_num = true;
                }
                _ => {}
            }
        }
        // EOF reached
        input_finished = true;

        if began_num {
            append_int(
                &mut num,
                &mut sign_v,
                &mut nb_read_cls,
                &mut nb_vars,
                &mut nb_cls,
                &mut header,
                &mut began_num,
                &mut data,
                &mut hash_bytes,
                &mut self.f_out,
            );
        }
        if !data.is_empty() {
            flush_buffer(&mut data, &mut hash_bytes, &mut self.f_out);
        }
        // Two-byte padding
        hash_bytes.push(0);
        hash_bytes.push(0);

        let computed = siphash_full(&hash_bytes);
        // Write the signature to f_out
        let _ = self.f_out.write_all(&computed);
        *sig = Some(computed.to_vec());
        input_finished && !input_invalid
    }

    pub fn process(&mut self, _c: char) -> bool {
        // Mirror C process(c): returns whether to break out of the parse loop (EOF).
        // In this Rust port, parsing is performed in tp_parse directly.
        false
    }
}
