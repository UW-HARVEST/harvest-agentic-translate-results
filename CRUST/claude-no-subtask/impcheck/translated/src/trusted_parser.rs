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
    siphash: RefCell<SipHash>,
}

struct ParserState {
    data: Vec<i32>,
    capacity: u64,
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

impl ParserState {
    fn new(capacity: u64) -> Self {
        Self {
            data: Vec::with_capacity(capacity as usize),
            capacity,
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
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        // The data Vec frees automatically when the struct is dropped.
        let mut st = self.state.borrow_mut();
        st.data.clear();
    }

    pub fn output_literal_buffer(&self) {
        let mut st = self.state.borrow_mut();
        let mut sh = self.siphash.borrow_mut();
        // Compose bytes from the data
        let n = st.data.len();
        let mut bytes = Vec::with_capacity(n * 4);
        for v in &st.data {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        sh.siphash_update(&bytes, bytes.len() as u64);
        // Write to f_out
        let f_out_ptr = &self.f_out as *const File as *mut File;
        let data_clone = st.data.clone();
        let size = data_clone.len() as u64;
        unsafe {
            trusted_utils_write_ints(&data_clone, size, &mut *f_out_ptr);
        }
        st.data.clear();
    }

    pub fn append_integer(&self) {
        let mut st = self.state.borrow_mut();
        if st.header {
            if st.nb_vars == -1 {
                st.nb_vars = st.num;
                let nb_vars = st.nb_vars;
                drop(st);
                let f_out_ptr = &self.f_out as *const File as *mut File;
                unsafe {
                    trusted_utils_write_int(nb_vars, &mut *f_out_ptr);
                }
                let mut st = self.state.borrow_mut();
                st.num = 0;
                st.began_num = false;
                return;
            } else if st.nb_cls == -1 {
                st.nb_cls = st.num;
                let nb_cls = st.nb_cls;
                drop(st);
                let f_out_ptr = &self.f_out as *const File as *mut File;
                unsafe {
                    trusted_utils_write_int(nb_cls, &mut *f_out_ptr);
                }
                let mut st = self.state.borrow_mut();
                st.header = false;
                st.num = 0;
                st.began_num = false;
                return;
            } else {
                // C code aborts here; mirror by marking input invalid
                // and returning so the caller fails gracefully.
                st.input_invalid = true;
                st.input_finished = true;
                return;
            }
        }

        let lit = st.sign * st.num;
        if lit == 0 {
            st.nb_read_cls += 1;
        }
        st.num = 0;
        st.sign = 1;
        st.began_num = false;

        if st.data.len() as u64 == st.capacity {
            drop(st);
            self.output_literal_buffer();
            let mut st = self.state.borrow_mut();
            st.data.push(lit);
        } else {
            st.data.push(lit);
        }
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = match File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                crate::trusted_utils::trusted_utils_exit_eof();
                unreachable!()
            }
        };
        Self {
            f_out: out,
            f,
            state: RefCell::new(ParserState::new(TRUSTED_CHK_MAX_BUF_SIZE as u64)),
            siphash: RefCell::new(SipHash::siphash_init(&SECRET_KEY)),
        }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        // Read the file character by character
        let mut buf = [0u8; 4096];
        loop {
            let n = match self.f.read(&mut buf) {
                Ok(0) => {
                    // EOF
                    let _ = self.process('\u{FFFF}'); // signal EOF via process - but use a non-byte char
                    self.state.borrow_mut().input_finished = true;
                    break;
                }
                Ok(n) => n,
                Err(_) => {
                    self.state.borrow_mut().input_finished = true;
                    break;
                }
            };
            let mut done = false;
            for i in 0..n {
                let c = buf[i] as char;
                if self.process(c) {
                    done = true;
                    break;
                }
            }
            if done {
                break;
            }
        }
        // Trailing began_num handling
        let needs_append = {
            let st = self.state.borrow();
            st.began_num
        };
        if needs_append {
            self.append_integer();
        }
        let has_data = self.state.borrow().data.len() > 0;
        if has_data {
            self.output_literal_buffer();
        }
        // Two-byte padding for formula signature input
        {
            let mut sh = self.siphash.borrow_mut();
            sh.siphash_pad(2);
            let digest = sh.siphash_digest();
            *sig = Some(digest.clone());
            // Write signature to f_out
            trusted_utils_write_sig(&digest, &mut self.f_out);
            // Flush f_out
            let _ = self.f_out.flush();
        }
        let st = self.state.borrow();
        st.input_finished && !st.input_invalid
    }

    pub fn process(&mut self, c: char) -> bool {
        let mut st = self.state.borrow_mut();
        if st.comment && c != '\n' && c != '\r' {
            return false;
        }
        match c {
            '\u{FFFF}' => {
                // EOF marker we use internally
                st.input_finished = true;
                return true;
            }
            '\n' | '\r' => {
                st.comment = false;
                if st.began_num {
                    drop(st);
                    self.append_integer();
                    return false;
                }
            }
            'p' => {
                st.header = true;
            }
            'c' => {
                if !st.header {
                    st.comment = true;
                }
            }
            ' ' | '\t' => {
                if st.began_num {
                    drop(st);
                    self.append_integer();
                    return false;
                }
            }
            '-' => {
                st.sign = -1;
                st.began_num = true;
            }
            '0'..='9' => {
                st.num = st.num * 10 + ((c as i32) - ('0' as i32));
                st.began_num = true;
            }
            _ => {}
        }
        false
    }
}
