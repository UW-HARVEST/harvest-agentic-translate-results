use crate::secret::SECRET_KEY;
use crate::siphash_global;
use crate::trusted_utils::{
    trusted_utils_write_int, trusted_utils_write_ints, trusted_utils_write_sig,
    TRUSTED_CHK_MAX_BUF_SIZE,
};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
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
}

thread_local! {
    static STATE: RefCell<Option<ParserState>> = RefCell::new(None);
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        STATE.with(|s| {
            *s.borrow_mut() = None;
        });
        let _ = self.f.read(&mut [0u8; 0]);
    }

    pub fn output_literal_buffer(&self) {
        let bytes_and_len = STATE.with(|s| {
            let sr = s.borrow();
            let st = sr.as_ref().expect("parser not initialized");
            let mut bytes = Vec::with_capacity(st.data.len() * 4);
            for &v in st.data.iter() {
                bytes.extend_from_slice(&v.to_ne_bytes());
            }
            (bytes, st.data.clone())
        });
        siphash_global::with_global_siphash(|sh| {
            sh.siphash_update(&bytes_and_len.0, bytes_and_len.0.len() as u64);
        });
        let mut f_out_clone: File = self.f_out.try_clone().expect("clone f_out");
        trusted_utils_write_ints(&bytes_and_len.1, bytes_and_len.1.len() as u64, &mut f_out_clone);
        STATE.with(|s| {
            let mut sr = s.borrow_mut();
            sr.as_mut().unwrap().data.clear();
        });
    }

    pub fn append_integer(&self) {
        let action = STATE.with(|s| {
            let mut state_ref = s.borrow_mut();
            let st = state_ref.as_mut().expect("parser not initialized");
            if st.header {
                if st.nb_vars == -1 {
                    st.nb_vars = st.num;
                    let v = st.nb_vars;
                    st.num = 0;
                    st.began_num = false;
                    return Some(("int", v, 0));
                } else if st.nb_cls == -1 {
                    st.nb_cls = st.num;
                    let v = st.nb_cls;
                    st.header = false;
                    st.num = 0;
                    st.began_num = false;
                    return Some(("int", v, 0));
                } else {
                    st.input_invalid = true;
                    return None;
                }
            }
            let lit = st.sign * st.num;
            if lit == 0 {
                st.nb_read_cls += 1;
            }
            st.num = 0;
            st.sign = 1;
            st.began_num = false;
            if st.data.len() == st.capacity {
                Some(("flush_then_push", lit, 1))
            } else {
                st.data.push(lit);
                None
            }
        });
        if let Some((kind, v, _)) = action {
            if kind == "int" {
                let mut f_out_clone: File = self.f_out.try_clone().expect("clone f_out");
                trusted_utils_write_int(v, &mut f_out_clone);
            } else if kind == "flush_then_push" {
                self.output_literal_buffer();
                STATE.with(|s| {
                    let mut sr = s.borrow_mut();
                    sr.as_mut().unwrap().data.push(v);
                });
            }
        }
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        siphash_global::init(&SECRET_KEY);
        let f = File::open(filename).expect("tp_init: failed to open input file");
        let f_out = out.try_clone().expect("tp_init: clone out file");
        let st = ParserState {
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
        };
        STATE.with(|s| {
            *s.borrow_mut() = Some(st);
        });
        let _ = f_out;
        Self { f_out: out, f }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            let n = self.f.read(&mut buf).unwrap_or(0);
            let c = if n == 0 { -1i32 } else { buf[0] as i32 };
            if self.process_int(c) {
                break;
            }
        }
        let began_num = STATE.with(|s| s.borrow().as_ref().map(|st| st.began_num).unwrap_or(false));
        if began_num {
            self.append_integer();
        }
        let data_size = STATE.with(|s| s.borrow().as_ref().map(|st| st.data.len()).unwrap_or(0));
        if data_size > 0 {
            self.output_literal_buffer();
        }
        siphash_global::with_global_siphash(|sh| {
            sh.siphash_pad(2);
        });
        let digest = siphash_global::with_global_siphash(|sh| sh.siphash_digest());
        trusted_utils_write_sig(&digest, &mut self.f_out);
        *sig = Some(digest);
        let _ = self.f_out.flush();
        let (input_finished, input_invalid) = STATE.with(|s| {
            let sr = s.borrow();
            let st = sr.as_ref().unwrap();
            (st.input_finished, st.input_invalid)
        });
        input_finished && !input_invalid
    }

    fn process_int(&mut self, c_int: i32) -> bool {
        let comment = STATE.with(|s| s.borrow().as_ref().unwrap().comment);
        if c_int == -1 {
            STATE.with(|s| {
                let mut sr = s.borrow_mut();
                sr.as_mut().unwrap().input_finished = true;
            });
            return true;
        }
        let c = c_int as u8 as char;
        if comment && c != '\n' && c != '\r' {
            return false;
        }

        match c {
            '\n' | '\r' => {
                let began_num = STATE.with(|s| {
                    let mut sr = s.borrow_mut();
                    let st = sr.as_mut().unwrap();
                    st.comment = false;
                    st.began_num
                });
                if began_num {
                    self.append_integer();
                }
            }
            'p' => {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().header = true);
            }
            'c' => {
                STATE.with(|s| {
                    let mut sr = s.borrow_mut();
                    let st = sr.as_mut().unwrap();
                    if !st.header {
                        st.comment = true;
                    }
                });
            }
            ' ' => {
                let began_num = STATE.with(|s| s.borrow().as_ref().unwrap().began_num);
                if began_num {
                    self.append_integer();
                }
            }
            '-' => {
                STATE.with(|s| {
                    let mut sr = s.borrow_mut();
                    let st = sr.as_mut().unwrap();
                    st.sign = -1;
                    st.began_num = true;
                });
            }
            '0'..='9' => {
                STATE.with(|s| {
                    let mut sr = s.borrow_mut();
                    let st = sr.as_mut().unwrap();
                    st.num = st.num * 10 + (c as i32 - '0' as i32);
                    st.began_num = true;
                });
            }
            _ => {}
        }
        false
    }

    pub fn process(&mut self, c: char) -> bool {
        self.process_int(c as i32)
    }
}
