use crate::siphash::SipHash;
use crate::trusted_utils;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Mutex;

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
    siphash: SipHash,
}

impl ParserState {
    fn new() -> Self {
        let key: [u8; 16] = [
            86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
        ];
        ParserState {
            data: Vec::with_capacity(trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE),
            capacity: trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE,
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
            siphash: SipHash::siphash_init(&key),
        }
    }
}

fn pstate() -> &'static Mutex<ParserState> {
    use std::sync::OnceLock;
    static S: OnceLock<Mutex<ParserState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(ParserState::new()))
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        // Reset state
        let mut s = pstate().lock().unwrap();
        *s = ParserState::new();
        let _ = self.f_out.flush();
    }

    pub fn output_literal_buffer(&self) {
        let mut s = pstate().lock().unwrap();
        let bytes_len = s.data.len() * std::mem::size_of::<i32>();
        let mut bytes = Vec::with_capacity(bytes_len);
        for &v in &s.data {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        s.siphash.siphash_update(&bytes, bytes_len as u64);
        // Write ints to output
        let data = s.data.clone();
        let n = data.len() as u64;
        drop(s);
        let mut out = (&self.f_out).try_clone().unwrap();
        trusted_utils::trusted_utils_write_ints(&data, n, &mut out);
        let mut s = pstate().lock().unwrap();
        s.data.clear();
    }

    pub fn append_integer(&self) {
        let mut s = pstate().lock().unwrap();
        if s.header {
            if s.nb_vars == -1 {
                s.nb_vars = s.num;
                let nb = s.nb_vars;
                drop(s);
                let mut out = (&self.f_out).try_clone().unwrap();
                trusted_utils::trusted_utils_write_int(nb, &mut out);
                let mut s2 = pstate().lock().unwrap();
                s2.num = 0;
                s2.began_num = false;
                return;
            } else if s.nb_cls == -1 {
                s.nb_cls = s.num;
                let nb = s.nb_cls;
                s.header = false;
                drop(s);
                let mut out = (&self.f_out).try_clone().unwrap();
                trusted_utils::trusted_utils_write_int(nb, &mut out);
                let mut s2 = pstate().lock().unwrap();
                s2.num = 0;
                s2.began_num = false;
                return;
            } else {
                // abort()
                std::process::exit(1);
            }
        }
        let lit = s.sign * s.num;
        if lit == 0 {
            s.nb_read_cls += 1;
        }
        s.num = 0;
        s.sign = 1;
        s.began_num = false;
        let cap_full = s.data.len() == s.capacity;
        if cap_full {
            drop(s);
            self.output_literal_buffer();
            let mut s2 = pstate().lock().unwrap();
            s2.data.push(lit);
        } else {
            s.data.push(lit);
        }
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        // Reset global parser state.
        {
            let mut s = pstate().lock().unwrap();
            *s = ParserState::new();
        }
        let f = File::open(filename).expect("open input formula file");
        TrustedParser { f_out: out, f }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            let n = self.f.read(&mut buf).unwrap_or(0);
            let c = if n == 0 { -1i32 } else { buf[0] as i32 };
            // Process character
            if self.process_eof_or_char(c) {
                break;
            }
        }
        let began_num = pstate().lock().unwrap().began_num;
        if began_num {
            self.append_integer();
        }
        let data_size = pstate().lock().unwrap().data.len();
        if data_size > 0 {
            self.output_literal_buffer();
        }
        let mut s = pstate().lock().unwrap();
        s.siphash.siphash_pad(2);
        let digest = s.siphash.siphash_digest();
        *sig = Some(digest.clone());
        let input_finished = s.input_finished;
        let input_invalid = s.input_invalid;
        drop(s);
        let mut out = (&self.f_out).try_clone().unwrap();
        trusted_utils::trusted_utils_write_sig(&digest, &mut out);
        input_finished && !input_invalid
    }

    fn process_eof_or_char(&self, c: i32) -> bool {
        if c < 0 {
            // EOF
            let mut s = pstate().lock().unwrap();
            s.input_finished = true;
            return true;
        }
        let ch = (c & 0xff) as u8 as char;
        self.process(ch)
    }

    pub fn process(&self, c: char) -> bool {
        // Check comment state, header state etc.
        let comment = pstate().lock().unwrap().comment;
        if comment && c != '\n' && c != '\r' {
            return false;
        }
        match c {
            '\n' | '\r' => {
                let began_num = {
                    let mut s = pstate().lock().unwrap();
                    s.comment = false;
                    s.began_num
                };
                if began_num {
                    self.append_integer();
                }
            }
            'p' => {
                let mut s = pstate().lock().unwrap();
                s.header = true;
            }
            'c' => {
                let mut s = pstate().lock().unwrap();
                if !s.header {
                    s.comment = true;
                }
            }
            ' ' => {
                let began_num = pstate().lock().unwrap().began_num;
                if began_num {
                    self.append_integer();
                }
            }
            '-' => {
                let mut s = pstate().lock().unwrap();
                s.sign = -1;
                s.began_num = true;
            }
            '0'..='9' => {
                let mut s = pstate().lock().unwrap();
                s.num = s.num * 10 + (c as i32 - '0' as i32);
                s.began_num = true;
            }
            _ => {}
        }
        false
    }
}
