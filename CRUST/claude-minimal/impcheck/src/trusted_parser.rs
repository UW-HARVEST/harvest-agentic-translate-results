// Direct port of c_src/src/trusted/trusted_parser.c

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::*;

use std::cell::RefCell;
use std::fs::File;
use std::io::Read;

pub struct IntVec {
    pub capacity: u64,
    pub size: u64,
    pub data: Vec<i32>,
}

impl IntVec {
    pub fn new(capacity: u64) -> Self {
        IntVec {
            capacity,
            size: 0,
            data: vec![0; capacity as usize],
        }
    }
    pub fn push(&mut self, v: i32) {
        if self.size == self.capacity {
            let mut nc = (self.capacity as f64 * 1.3) as u64;
            if nc < self.capacity + 1 {
                nc = self.capacity + 1;
            }
            self.data.resize(nc as usize, 0);
            self.capacity = nc;
        }
        self.data[self.size as usize] = v;
        self.size += 1;
    }
    pub fn clear(&mut self) {
        self.size = 0;
    }
}

pub struct TrustedParser {
    pub f_out: File,
    pub f: File,
}

thread_local! {
    pub static PARSER_STATE: RefCell<ParserState> = RefCell::new(ParserState::new());
}

pub struct ParserState {
    pub data: IntVec,
    pub comment: bool,
    pub header: bool,
    pub input_finished: bool,
    pub input_invalid: bool,
    pub began_num: bool,
    pub num: i32,
    pub sign: i32,
    pub nb_read_cls: i32,
    pub nb_vars: i32,
    pub nb_cls: i32,
    pub siphash: SipHash,
}

impl ParserState {
    fn new() -> Self {
        ParserState {
            data: IntVec::new(TRUSTED_CHK_MAX_BUF_SIZE as u64),
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
            siphash: SipHash::siphash_init(&SECRET_KEY),
        }
    }
}

impl TrustedParser {
    pub fn tp_init(filename: &str, out: File) -> Self {
        // Reset thread-local state
        PARSER_STATE.with(|s| {
            *s.borrow_mut() = ParserState::new();
        });
        let f = match File::open(filename) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!()
            }
        };
        TrustedParser { f_out: out, f }
    }

    pub fn tp_end(&mut self) {
        PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.data.size = 0;
            st.data.capacity = 0;
            st.data.data.clear();
        });
    }

    pub fn output_literal_buffer(&self) {
        PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            let nb = st.data.size;
            let mut bytes: Vec<u8> = Vec::with_capacity(nb as usize * 4);
            for i in 0..(nb as usize) {
                bytes.extend_from_slice(&st.data.data[i].to_ne_bytes());
            }
            st.siphash.siphash_update(&bytes, nb * 4);
            // Write ints to f_out — but `self` is &self; we need to write to the
            // underlying file. Use unsafe pointer trick: clone fd via try_clone.
            // To keep things simple, we don't write here in &self; the C version
            // writes to f_out via global FILE*. For this Rust port, this method
            // requires &mut self. We provide a helper instead below.
            st.data.size = 0;
        });
    }

    pub fn output_literal_buffer_mut(&mut self) {
        PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            let nb = st.data.size;
            let mut bytes: Vec<u8> = Vec::with_capacity(nb as usize * 4);
            for i in 0..(nb as usize) {
                bytes.extend_from_slice(&st.data.data[i].to_ne_bytes());
            }
            st.siphash.siphash_update(&bytes, nb * 4);
            trusted_utils_write_ints(&st.data.data, nb, &mut self.f_out);
            st.data.size = 0;
        });
    }

    pub fn append_integer(&self) {
        // Same caveat as output_literal_buffer: we do all the bookkeeping with
        // `&self`, deferring writes to the buffered version.
        PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            if st.header {
                if st.nb_vars == -1 {
                    st.nb_vars = st.num;
                } else if st.nb_cls == -1 {
                    st.nb_cls = st.num;
                    st.header = false;
                }
                st.num = 0;
                st.began_num = false;
                return;
            }
            let lit = st.sign * st.num;
            if lit == 0 {
                st.nb_read_cls += 1;
            }
            st.num = 0;
            st.sign = 1;
            st.began_num = false;
            if st.data.size == st.data.capacity {
                // Output later via output_literal_buffer_mut.
            }
            st.data.push(lit);
        });
    }

    pub fn append_integer_mut(&mut self) {
        PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            if st.header {
                if st.nb_vars == -1 {
                    st.nb_vars = st.num;
                    let v = st.nb_vars;
                    trusted_utils_write_int(v, &mut self.f_out);
                } else if st.nb_cls == -1 {
                    st.nb_cls = st.num;
                    let v = st.nb_cls;
                    trusted_utils_write_int(v, &mut self.f_out);
                    st.header = false;
                }
                st.num = 0;
                st.began_num = false;
                return;
            }
            let lit = st.sign * st.num;
            if lit == 0 {
                st.nb_read_cls += 1;
            }
            st.num = 0;
            st.sign = 1;
            st.began_num = false;
            if st.data.size == st.data.capacity {
                // inline output literal buffer
                let nb = st.data.size;
                let mut bytes: Vec<u8> = Vec::with_capacity(nb as usize * 4);
                for i in 0..(nb as usize) {
                    bytes.extend_from_slice(&st.data.data[i].to_ne_bytes());
                }
                st.siphash.siphash_update(&bytes, nb * 4);
                trusted_utils_write_ints(&st.data.data, nb, &mut self.f_out);
                st.data.size = 0;
            }
            st.data.push(lit);
        });
    }

    pub fn process(&mut self, c: char) -> bool {
        let st_comment = PARSER_STATE.with(|s| s.borrow().comment);
        if st_comment && c != '\n' && c != '\r' {
            return false;
        }
        match c {
            '\n' | '\r' => {
                let began = PARSER_STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.comment = false;
                    st.began_num
                });
                if began {
                    self.append_integer_mut();
                }
            }
            'p' => {
                PARSER_STATE.with(|s| s.borrow_mut().header = true);
            }
            'c' => {
                PARSER_STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    if !st.header {
                        st.comment = true;
                    }
                });
            }
            ' ' => {
                let began = PARSER_STATE.with(|s| s.borrow().began_num);
                if began {
                    self.append_integer_mut();
                }
            }
            '-' => {
                PARSER_STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.sign = -1;
                    st.began_num = true;
                });
            }
            '0'..='9' => {
                PARSER_STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.num = st.num * 10 + (c as i32 - '0' as i32);
                    st.began_num = true;
                });
            }
            _ => {}
        }
        false
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            let nread = self.f.read(&mut buf).unwrap_or(0);
            if nread == 0 {
                PARSER_STATE.with(|s| s.borrow_mut().input_finished = true);
                break;
            }
            let c = buf[0] as char;
            if self.process(c) {
                break;
            }
        }
        let began = PARSER_STATE.with(|s| s.borrow().began_num);
        if began {
            self.append_integer_mut();
        }
        let sz = PARSER_STATE.with(|s| s.borrow().data.size);
        if sz > 0 {
            self.output_literal_buffer_mut();
        }
        let computed = PARSER_STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.siphash.siphash_pad(2);
            st.siphash.siphash_digest()
        });
        *sig = Some(computed.clone());
        trusted_utils_write_sig(&computed, &mut self.f_out);
        let (finished, invalid) = PARSER_STATE.with(|s| {
            let st = s.borrow();
            (st.input_finished, st.input_invalid)
        });
        finished && !invalid
    }
}
