use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_write_int, trusted_utils_write_ints, trusted_utils_write_sig,
    TRUSTED_CHK_MAX_BUF_SIZE,
};

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
    state: RefCell<ParserState>,
}

struct ParserState {
    data: Vec<i32>,
    siphash: SipHash,
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
        // No special teardown needed; Rust will drop everything.
        let mut s = self.state.borrow_mut();
        s.data.clear();
    }

    pub fn output_literal_buffer(&self) {
        let mut s = self.state.borrow_mut();
        let bytes: Vec<u8> = s.data.iter().flat_map(|&i| i.to_ne_bytes()).collect();
        let len = bytes.len() as u64;
        s.siphash.siphash_update(&bytes, len);
        // Write data to f_out
        let mut f_out = unsafe {
            // SAFETY: we need a &mut File but only have &self; we use a
            // workaround via raw pointer dereference of an unsafe cell.
            let p = &self.f_out as *const File as *mut File;
            &mut *p
        };
        let data_clone: Vec<i32> = s.data.clone();
        let n = data_clone.len() as u64;
        trusted_utils_write_ints(&data_clone, n, &mut f_out);
        s.data.clear();
    }

    pub fn append_integer(&self) {
        let mut header_done = false;
        let nb_vars_to_write;
        let nb_cls_to_write;
        {
            let mut s = self.state.borrow_mut();
            if s.header {
                if s.nb_vars == -1 {
                    s.nb_vars = s.num;
                    nb_vars_to_write = Some(s.num);
                    nb_cls_to_write = None;
                } else if s.nb_cls == -1 {
                    s.nb_cls = s.num;
                    nb_cls_to_write = Some(s.num);
                    nb_vars_to_write = None;
                    header_done = true;
                } else {
                    // Invalid header state - mark input as invalid and skip
                    s.input_invalid = true;
                    nb_vars_to_write = None;
                    nb_cls_to_write = None;
                }
                s.num = 0;
                s.began_num = false;
            } else {
                let lit = s.sign * s.num;
                if lit == 0 {
                    s.nb_read_cls += 1;
                }
                s.num = 0;
                s.sign = 1;
                s.began_num = false;
                let cap_reached = s.data.len() == TRUSTED_CHK_MAX_BUF_SIZE;
                if cap_reached {
                    drop(s);
                    self.output_literal_buffer();
                    let mut s2 = self.state.borrow_mut();
                    s2.data.push(lit);
                } else {
                    s.data.push(lit);
                }
                return;
            }
        }
        if header_done {
            self.state.borrow_mut().header = false;
        }
        // Write to f_out
        let mut f_out = unsafe {
            let p = &self.f_out as *const File as *mut File;
            &mut *p
        };
        if let Some(v) = nb_vars_to_write {
            trusted_utils_write_int(v, &mut f_out);
        }
        if let Some(v) = nb_cls_to_write {
            trusted_utils_write_int(v, &mut f_out);
        }
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = match File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                eprintln!("trusted_parser: cannot open input '{}'", filename);
                std::process::exit(0);
            }
        };
        let sh = SipHash::siphash_init(&SECRET_KEY);
        TrustedParser {
            f_out: out,
            f,
            state: RefCell::new(ParserState {
                data: Vec::with_capacity(TRUSTED_CHK_MAX_BUF_SIZE),
                siphash: sh,
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
            }),
        }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            let c = match self.f.read(&mut buf) {
                Ok(0) => '\u{FFFF}', // sentinel for EOF
                Ok(_) => buf[0] as char,
                Err(_) => '\u{FFFF}',
            };
            if self.process(c) {
                break;
            }
        }
        let began = self.state.borrow().began_num;
        if began {
            self.append_integer();
        }
        let has_data = !self.state.borrow().data.is_empty();
        if has_data {
            self.output_literal_buffer();
        }
        let digest;
        {
            let mut s = self.state.borrow_mut();
            s.siphash.siphash_pad(2);
            digest = s.siphash.siphash_digest();
        }
        *sig = Some(digest.clone());
        trusted_utils_write_sig(&digest, &mut self.f_out);
        let s = self.state.borrow();
        s.input_finished && !s.input_invalid
    }

    pub fn process(&mut self, c: char) -> bool {
        let mut s = self.state.borrow_mut();
        if s.comment && c != '\n' && c != '\r' {
            return false;
        }
        // EOF sentinel
        if c == '\u{FFFF}' {
            s.input_finished = true;
            return true;
        }
        match c {
            '\n' | '\r' => {
                s.comment = false;
                if s.began_num {
                    drop(s);
                    self.append_integer();
                    return false;
                }
            }
            'p' => {
                s.header = true;
            }
            'c' => {
                if !s.header {
                    s.comment = true;
                }
            }
            ' ' => {
                if s.began_num {
                    drop(s);
                    self.append_integer();
                    return false;
                }
            }
            '-' => {
                s.sign = -1;
                s.began_num = true;
            }
            '0'..='9' => {
                let d = (c as i32) - ('0' as i32);
                s.num = s.num * 10 + d;
                s.began_num = true;
            }
            _ => {}
        }
        false
    }
}
