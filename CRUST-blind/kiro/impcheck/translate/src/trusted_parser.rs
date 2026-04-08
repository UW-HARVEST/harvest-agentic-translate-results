use std::fs::File;
use std::io::{Read, Write};
use crate::siphash::SipHash;
use crate::secret::SECRET_KEY;
use crate::trusted_utils::{trusted_utils_write_int, trusted_utils_write_ints, trusted_utils_write_sig, TRUSTED_CHK_MAX_BUF_SIZE};

pub struct IntVec;
pub struct TrustedParser {
f_out: File,
f: File,
siphash: SipHash,
data: Vec<i32>,
comment: bool,
header: bool,
input_finished: bool,
input_invalid: bool,
began_num: bool,
num: i32,
sign: i32,
nb_read_cls: i32,
nb_vars: i32,
nb_cls: i32,
}

impl TrustedParser {
pub fn tp_end(&mut self) {
    // nothing to do in Rust, data is dropped automatically
}

pub fn output_literal_buffer(&mut self) {
    let data_bytes: Vec<u8> = self.data.iter().flat_map(|x| x.to_ne_bytes()).collect();
    self.siphash.siphash_update(&data_bytes, data_bytes.len() as u64);
    trusted_utils_write_ints(&self.data, self.data.len() as u64, &mut self.f_out);
    self.data.clear();
}

pub fn append_integer(&mut self) {
    if self.header {
        if self.nb_vars == -1 {
            self.nb_vars = self.num;
            trusted_utils_write_int(self.nb_vars, &mut self.f_out);
        } else if self.nb_cls == -1 {
            self.nb_cls = self.num;
            trusted_utils_write_int(self.nb_cls, &mut self.f_out);
            self.header = false;
        }
        self.num = 0;
        self.began_num = false;
        return;
    }
    let lit = self.sign * self.num;
    if lit == 0 { self.nb_read_cls += 1; }
    self.num = 0;
    self.sign = 1;
    self.began_num = false;
    if self.data.len() == self.data.capacity() {
        self.output_literal_buffer();
    }
    self.data.push(lit);
}

pub fn tp_init(filename: &str, out: File) -> Self {
    let siphash = SipHash::siphash_init(&SECRET_KEY);
    let f = File::open(filename).expect("Failed to open input file");
    TrustedParser {
        f_out: out,
        f,
        siphash,
        data: Vec::with_capacity(TRUSTED_CHK_MAX_BUF_SIZE),
        comment: false,
        header: false,
        input_finished: false,
        input_invalid: false,
        began_num: false,
        num: 0,
        sign: 1,
        nb_read_cls: 0,
        nb_vars: -1,
        nb_cls: -1,
    }
}

pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
    let mut buf = [0u8; 1];
    loop {
        match self.f.read(&mut buf) {
            Ok(0) => {
                // EOF
                if self.process('\xff') { break; }
                // Also try with actual EOF handling
                self.input_finished = true;
                break;
            }
            Ok(_) => {
                let c = buf[0] as char;
                if self.process(c) { break; }
            }
            Err(_) => {
                self.input_finished = true;
                break;
            }
        }
    }
    if self.began_num { self.append_integer(); }
    if !self.data.is_empty() { self.output_literal_buffer(); }
    self.siphash.siphash_pad(2);
    let digest = self.siphash.siphash_digest();
    trusted_utils_write_sig(&digest, &mut self.f_out);
    *sig = Some(digest);
    self.input_finished && !self.input_invalid
}

pub fn process(&mut self, c: char) -> bool {
    if self.comment && c != '\n' && c != '\r' { return false; }
    let uc = c as u8 as i8;
    match uc {
        -1 => { // EOF
            self.input_finished = true;
            return true;
        }
        b'\n' as i8 | b'\r' as i8 => {
            self.comment = false;
            if self.began_num { self.append_integer(); }
        }
        b'p' as i8 => {
            self.header = true;
        }
        b'c' as i8 => {
            if !self.header { self.comment = true; }
        }
        b' ' as i8 => {
            if self.began_num { self.append_integer(); }
        }
        b'-' as i8 => {
            self.sign = -1;
            self.began_num = true;
        }
        b'0' as i8..=b'9' as i8 => {
            self.num = self.num * 10 + (c as i32 - '0' as i32);
            self.began_num = true;
        }
        _ => {}
    }
    false
}
}
