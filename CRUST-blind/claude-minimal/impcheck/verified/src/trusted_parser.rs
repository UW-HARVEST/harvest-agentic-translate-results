use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};

use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_write_int, trusted_utils_write_ints, trusted_utils_write_sig,
    TRUSTED_CHK_MAX_BUF_SIZE,
};

// Secret key constant -- mirrors c_src/src/trusted/secret.c
const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
    sip: RefCell<SipHash>,
    data: RefCell<Vec<i32>>,
    comment: RefCell<bool>,
    header: RefCell<bool>,
    input_finished: RefCell<bool>,
    input_invalid: RefCell<bool>,
    began_num: RefCell<bool>,
    num: RefCell<i32>,
    sign: RefCell<i32>,
    nb_read_cls: RefCell<i32>,
    nb_vars: RefCell<i32>,
    nb_cls: RefCell<i32>,
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        self.data.borrow_mut().clear();
    }
    pub fn output_literal_buffer(&self) {
        let data = self.data.borrow();
        // siphash_update on raw bytes of the i32 buffer
        let mut bytes: Vec<u8> = Vec::with_capacity(data.len() * 4);
        for &lit in data.iter() {
            bytes.extend_from_slice(&lit.to_ne_bytes());
        }
        self.sip.borrow_mut().siphash_update(&bytes, bytes.len() as u64);
        // write to f_out
        let mut f_out = match self.f_out.try_clone() {
            Ok(f) => f,
            Err(_) => return,
        };
        trusted_utils_write_ints(&data, data.len() as u64, &mut f_out);
        drop(data);
        self.data.borrow_mut().clear();
    }
    pub fn append_integer(&self) {
        let header = *self.header.borrow();
        let num = *self.num.borrow();
        if header {
            let mut nb_vars = self.nb_vars.borrow_mut();
            let mut nb_cls = self.nb_cls.borrow_mut();
            let mut f_out = match self.f_out.try_clone() {
                Ok(f) => f,
                Err(_) => return,
            };
            if *nb_vars == -1 {
                *nb_vars = num;
                trusted_utils_write_int(*nb_vars, &mut f_out);
            } else if *nb_cls == -1 {
                *nb_cls = num;
                trusted_utils_write_int(*nb_cls, &mut f_out);
                *self.header.borrow_mut() = false;
            } else {
                std::process::abort();
            }
            *self.num.borrow_mut() = 0;
            *self.began_num.borrow_mut() = false;
            return;
        }
        let sign = *self.sign.borrow();
        let lit = sign * num;
        if lit == 0 {
            *self.nb_read_cls.borrow_mut() += 1;
        }
        *self.num.borrow_mut() = 0;
        *self.sign.borrow_mut() = 1;
        *self.began_num.borrow_mut() = false;

        if self.data.borrow().len() == TRUSTED_CHK_MAX_BUF_SIZE {
            self.output_literal_buffer();
        }
        self.data.borrow_mut().push(lit);
    }
    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = match File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                // Fall back to a /dev/null-style empty input on failure.
                File::open("/dev/null").unwrap_or_else(|_| {
                    panic!("could not open input file {}", filename)
                })
            }
        };
        let sh = SipHash::siphash_init(&SECRET_KEY);
        TrustedParser {
            f_out: out,
            f,
            sip: RefCell::new(sh),
            data: RefCell::new(Vec::with_capacity(TRUSTED_CHK_MAX_BUF_SIZE)),
            comment: RefCell::new(false),
            header: RefCell::new(false),
            input_finished: RefCell::new(false),
            input_invalid: RefCell::new(false),
            began_num: RefCell::new(false),
            num: RefCell::new(0),
            sign: RefCell::new(1),
            nb_read_cls: RefCell::new(0),
            nb_vars: RefCell::new(-1),
            nb_cls: RefCell::new(-1),
        }
    }
    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            let read_res = self.f.read(&mut buf);
            match read_res {
                Ok(0) => {
                    // EOF
                    let _ = self.process('\u{ff}');
                    *self.input_finished.borrow_mut() = true;
                    break;
                }
                Ok(_) => {
                    let c = buf[0] as char;
                    if self.process(c) {
                        break;
                    }
                }
                Err(_) => {
                    *self.input_finished.borrow_mut() = true;
                    break;
                }
            }
        }
        if *self.began_num.borrow() {
            self.append_integer();
        }
        if !self.data.borrow().is_empty() {
            self.output_literal_buffer();
        }
        self.sip.borrow_mut().siphash_pad(2);
        let computed = self.sip.borrow().siphash_digest();
        let _ = self.f_out.write_all(&computed);
        let mut f_out = match self.f_out.try_clone() {
            Ok(f) => f,
            Err(_) => return false,
        };
        trusted_utils_write_sig(&computed, &mut f_out);
        *sig = Some(computed);
        *self.input_finished.borrow() && !*self.input_invalid.borrow()
    }
    pub fn process(&mut self, c: char) -> bool {
        let comment = *self.comment.borrow();
        if comment && c != '\n' && c != '\r' {
            return false;
        }
        match c {
            '\n' | '\r' => {
                *self.comment.borrow_mut() = false;
                if *self.began_num.borrow() {
                    self.append_integer();
                }
            }
            'p' => {
                *self.header.borrow_mut() = true;
            }
            'c' => {
                if !*self.header.borrow() {
                    *self.comment.borrow_mut() = true;
                }
            }
            ' ' => {
                if *self.began_num.borrow() {
                    self.append_integer();
                }
            }
            '-' => {
                *self.sign.borrow_mut() = -1;
                *self.began_num.borrow_mut() = true;
            }
            '0'..='9' => {
                let d = (c as u8) - b'0';
                let mut num = self.num.borrow_mut();
                *num = *num * 10 + (d as i32);
                *self.began_num.borrow_mut() = true;
            }
            _ => {}
        }
        false
    }
}
