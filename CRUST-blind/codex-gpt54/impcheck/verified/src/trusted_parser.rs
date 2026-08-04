use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::cell::RefCell;
use std::collections::HashMap;
pub struct IntVec;

struct ParserState {
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
    hasher: crate::siphash::SipHash,
}

thread_local! {
    static PARSER_STATES: RefCell<HashMap<i32, ParserState>> = RefCell::new(HashMap::new());
}

pub struct TrustedParser {
    f_out: File,
    f: File,
}
impl TrustedParser {
fn id(&self) -> i32 {
    self.f.as_raw_fd()
}
pub fn tp_end(&mut self) {
    PARSER_STATES.with(|states| {
        states.borrow_mut().remove(&self.id());
    });
}
pub fn output_literal_buffer(&self) {
    PARSER_STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(&self.id()) {
            let mut bytes = Vec::with_capacity(state.data.len() * std::mem::size_of::<i32>());
            for lit in &state.data {
                bytes.extend_from_slice(&lit.to_ne_bytes());
            }
            state.hasher.siphash_update(&bytes, bytes.len() as u64);
        }
    });
    let mut out = self.f_out.try_clone().unwrap();
    PARSER_STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(&self.id()) {
            crate::trusted_utils::trusted_utils_write_ints(&state.data, state.data.len() as u64, &mut out);
            state.data.clear();
        }
    });
}
pub fn append_integer(&self) {
    PARSER_STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(&self.id()) {
            if state.header {
                if state.nb_vars == -1 {
                    state.nb_vars = state.num;
                    let mut out = self.f_out.try_clone().unwrap();
                    crate::trusted_utils::trusted_utils_write_int(state.nb_vars, &mut out);
                } else if state.nb_cls == -1 {
                    state.nb_cls = state.num;
                    let mut out = self.f_out.try_clone().unwrap();
                    crate::trusted_utils::trusted_utils_write_int(state.nb_cls, &mut out);
                    state.header = false;
                }
                state.num = 0;
                state.began_num = false;
                return;
            }

            let lit = state.sign * state.num;
            if lit == 0 {
                state.nb_read_cls += 1;
            }
            state.num = 0;
            state.sign = 1;
            state.began_num = false;
            if state.data.len() == crate::trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE {
                drop(states);
                self.output_literal_buffer();
                PARSER_STATES.with(|states| {
                    if let Some(state) = states.borrow_mut().get_mut(&self.id()) {
                        state.data.push(lit);
                    }
                });
                return;
            }
            state.data.push(lit);
        }
    });
}
pub fn tp_init(filename: &str, out: File) -> Self {
    let input = File::open(filename).unwrap_or_else(|_| crate::trusted_utils::trusted_utils_exit_eof());
    let parser = Self { f_out: out, f: input };
    let key = [
        86_u8, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    PARSER_STATES.with(|states| {
        states.borrow_mut().insert(
            parser.id(),
            ParserState {
                data: Vec::with_capacity(crate::trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE),
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
                hasher: crate::siphash::SipHash::siphash_init(&key),
            },
        );
    });
    parser
}
pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
    let mut buf = [0_u8; 1];
    loop {
        match self.f.read(&mut buf) {
            Ok(0) => {
                if self.process(char::from_u32(255).unwrap_or('\0')) {
                    break;
                }
            }
            Ok(_) => {
                if self.process(buf[0] as char) {
                    break;
                }
            }
            Err(_) => crate::trusted_utils::trusted_utils_exit_eof(),
        }
    }
    let began_num = PARSER_STATES.with(|states| states.borrow().get(&self.id()).map(|s| s.began_num).unwrap_or(false));
    if began_num {
        self.append_integer();
    }
    let has_data = PARSER_STATES.with(|states| states.borrow().get(&self.id()).map(|s| !s.data.is_empty()).unwrap_or(false));
    if has_data {
        self.output_literal_buffer();
    }
    let digest = PARSER_STATES.with(|states| {
        let mut states = states.borrow_mut();
        let state = states.get_mut(&self.id()).unwrap();
        state.hasher.siphash_pad(2);
        state.hasher.siphash_digest()
    });
    *sig = Some(digest.clone());
    crate::trusted_utils::trusted_utils_write_sig(&digest, &mut self.f_out);
    PARSER_STATES.with(|states| {
        states
            .borrow()
            .get(&self.id())
            .map(|s| s.input_finished && !s.input_invalid)
            .unwrap_or(false)
    })
}
pub fn process(&mut self, c: char) -> bool {
    let id = self.id();
    let eof = c == char::from_u32(255).unwrap_or('\0');
    PARSER_STATES.with(|states| {
        let mut states = states.borrow_mut();
        let state = states.get_mut(&id).unwrap();
        if state.comment && c != '\n' && c != '\r' && !eof {
            return false;
        }
        match c {
            _ if eof => {
                state.input_finished = true;
                true
            }
            '\n' | '\r' => {
                state.comment = false;
                let should_append = state.began_num;
                drop(states);
                if should_append {
                    self.append_integer();
                }
                false
            }
            'p' => {
                state.header = true;
                false
            }
            'c' => {
                if !state.header {
                    state.comment = true;
                }
                false
            }
            ' ' => {
                let should_append = state.began_num;
                drop(states);
                if should_append {
                    self.append_integer();
                }
                false
            }
            '-' => {
                state.sign = -1;
                state.began_num = true;
                false
            }
            '0'..='9' => {
                state.num = state.num.saturating_mul(10).saturating_add(c as i32 - '0' as i32);
                state.began_num = true;
                false
            }
            _ => false,
        }
    })
}
}
