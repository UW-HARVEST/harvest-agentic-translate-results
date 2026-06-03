// Note: This file is not included via `lib.rs`. It is provided for
// completeness of the C-to-Rust translation.
#![allow(dead_code)]

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
    capacity: usize,
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
    sip: SipHash,
}

impl ParserState {
    fn new() -> Self {
        Self {
            data: Vec::with_capacity(TRUSTED_CHK_MAX_BUF_SIZE),
            capacity: TRUSTED_CHK_MAX_BUF_SIZE,
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
            sip: SipHash::siphash_init(&SECRET_KEY),
        }
    }
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        let _ = self.f_out.flush();
    }

    pub fn output_literal_buffer(&self) {
        let mut state = self.state.borrow_mut();
        let bytes: Vec<u8> = state.data.iter().flat_map(|i| i.to_ne_bytes()).collect();
        state.sip.siphash_update(&bytes, bytes.len() as u64);
        // Write directly to the underlying file, bypassing the &File borrow
        // by using OS-level write. We use a cloned handle.
        let n = state.data.len() as u64;
        let data_clone = state.data.clone();
        drop(state);
        // SAFETY: We need a mutable reference. Since this is &self per the
        // signature, use a bytes-based write via the std API on the file path.
        // Instead, we use raw bytes and File::write via a raw fd approach is
        // complicated; just reopen the writer would lose data. Instead, take
        // a raw pointer to f_out.
        unsafe {
            let f_ptr = &self.f_out as *const File as *mut File;
            trusted_utils_write_ints(&data_clone, n, &mut *f_ptr);
        }
        let mut state = self.state.borrow_mut();
        state.data.clear();
    }

    pub fn append_integer(&self) {
        let mut state = self.state.borrow_mut();
        if state.header {
            if state.nb_vars == -1 {
                state.nb_vars = state.num;
                let nv = state.nb_vars;
                drop(state);
                unsafe {
                    let f_ptr = &self.f_out as *const File as *mut File;
                    trusted_utils_write_int(nv, &mut *f_ptr);
                }
                let mut state = self.state.borrow_mut();
                state.num = 0;
                state.began_num = false;
                return;
            } else if state.nb_cls == -1 {
                state.nb_cls = state.num;
                let nc = state.nb_cls;
                state.header = false;
                drop(state);
                unsafe {
                    let f_ptr = &self.f_out as *const File as *mut File;
                    trusted_utils_write_int(nc, &mut *f_ptr);
                }
                let mut state = self.state.borrow_mut();
                state.num = 0;
                state.began_num = false;
                return;
            } else {
                std::process::abort();
            }
        }
        let lit = state.sign * state.num;
        if lit == 0 {
            state.nb_read_cls += 1;
        }
        state.num = 0;
        state.sign = 1;
        state.began_num = false;
        let need_flush = state.data.len() == state.capacity;
        drop(state);
        if need_flush {
            self.output_literal_buffer();
        }
        let mut state = self.state.borrow_mut();
        state.data.push(lit);
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = File::open(filename).unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        TrustedParser {
            f_out: out,
            f,
            state: RefCell::new(ParserState::new()),
        }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            match self.f.read(&mut buf) {
                Ok(0) => {
                    if self.process('\u{0}') {
                        // Treat as EOF: handled below
                    }
                    self.state.borrow_mut().input_finished = true;
                    break;
                }
                Ok(_) => {
                    let c = buf[0] as char;
                    if self.process(c) {
                        break;
                    }
                }
                Err(_) => {
                    self.state.borrow_mut().input_finished = true;
                    break;
                }
            }
        }
        let began_num = self.state.borrow().began_num;
        if began_num {
            self.append_integer();
        }
        let data_len = self.state.borrow().data.len();
        if data_len > 0 {
            self.output_literal_buffer();
        }
        let digest = {
            let mut state = self.state.borrow_mut();
            state.sip.siphash_pad(2);
            state.sip.siphash_digest()
        };
        *sig = Some(digest.clone());
        trusted_utils_write_sig(&digest, &mut self.f_out);
        let st = self.state.borrow();
        st.input_finished && !st.input_invalid
    }

    pub fn process(&mut self, c: char) -> bool {
        let mut state = self.state.borrow_mut();
        if state.comment && c != '\n' && c != '\r' {
            return false;
        }
        let uc = c as u8 as i8;
        // EOF: caller signals via a separate path; here, do not handle.
        let _ = uc;
        match c {
            '\n' | '\r' => {
                state.comment = false;
                if state.began_num {
                    drop(state);
                    self.append_integer();
                    return false;
                }
            }
            'p' => {
                state.header = true;
            }
            'c' => {
                if !state.header {
                    state.comment = true;
                }
            }
            ' ' => {
                if state.began_num {
                    drop(state);
                    self.append_integer();
                    return false;
                }
            }
            '-' => {
                state.sign = -1;
                state.began_num = true;
            }
            '0'..='9' => {
                state.num = state.num * 10 + (c as i32 - '0' as i32);
                state.began_num = true;
            }
            _ => {}
        }
        false
    }
}
