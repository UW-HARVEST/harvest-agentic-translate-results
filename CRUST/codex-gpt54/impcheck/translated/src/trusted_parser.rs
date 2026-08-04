use std::cell::RefCell;
use std::fs::File;
use std::io::Read;

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_exit_eof, trusted_utils_set_msg, trusted_utils_write_int,
    trusted_utils_write_ints, trusted_utils_write_sig, TRUSTED_CHK_MAX_BUF_SIZE,
};

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
    hasher: SipHash,
}

thread_local! {
    static STATE: RefCell<Option<ParserState>> = const { RefCell::new(None) };
}

pub struct TrustedParser {
    f_out: File,
    f: File,
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        STATE.with(|state| {
            *state.borrow_mut() = None;
        });
    }

    pub fn output_literal_buffer(&mut self) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut().unwrap();
            let bytes = ints_to_bytes(&state.data);
            state.hasher.siphash_update(&bytes, bytes.len() as u64);
            trusted_utils_write_ints(&state.data, state.data.len() as u64, &mut self.f_out);
            state.data.clear();
        });
    }

    pub fn append_integer(&mut self) {
        let mut flush_then_push = None;
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut().unwrap();
            if state.header {
                if state.nb_vars == -1 {
                    state.nb_vars = state.num;
                    trusted_utils_write_int(state.nb_vars, &mut self.f_out);
                } else if state.nb_cls == -1 {
                    state.nb_cls = state.num;
                    trusted_utils_write_int(state.nb_cls, &mut self.f_out);
                    state.header = false;
                } else {
                    trusted_utils_set_msg("Header parse error");
                    state.input_invalid = true;
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

            if state.data.len() == state.data.capacity() {
                flush_then_push = Some(lit);
                return;
            }
            state.data.push(lit);
        });
        if let Some(lit) = flush_then_push {
            self.output_literal_buffer();
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.as_mut().unwrap().data.push(lit);
            });
        }
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = match File::open(filename) {
            Ok(file) => file,
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!();
            }
        };
        STATE.with(|state| {
            *state.borrow_mut() = Some(ParserState {
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
                hasher: SipHash::siphash_init(&SECRET_KEY),
            });
        });
        Self { f_out: out, f }
    }

    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        loop {
            let mut buf = [0u8; 1];
            match self.f.read(&mut buf) {
                Ok(0) => {
                    STATE.with(|state| {
                        if let Some(state) = state.borrow_mut().as_mut() {
                            state.input_finished = true;
                        }
                    });
                    break;
                }
                Ok(_) => {
                    if self.process(buf[0] as char) {
                        break;
                    }
                }
                Err(_) => trusted_utils_exit_eof(),
            }
        }

        let needs_append = STATE.with(|state| {
            state.borrow().as_ref().map(|state| state.began_num).unwrap_or(false)
        });
        if needs_append {
            self.append_integer();
        }
        let has_buffer = STATE.with(|state| {
            state.borrow().as_ref().map(|state| !state.data.is_empty()).unwrap_or(false)
        });
        if has_buffer {
            self.output_literal_buffer();
        }
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut().unwrap();
            state.hasher.siphash_pad(2);
            let digest = state.hasher.siphash_digest();
            trusted_utils_write_sig(&digest, &mut self.f_out);
            *sig = Some(digest);
            state.input_finished && !state.input_invalid
        })
    }

    pub fn process(&mut self, c: char) -> bool {
        enum Action {
            None,
            Append,
        }

        let action = STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut().unwrap();

            if state.comment && c != '\n' && c != '\r' {
                return Action::None;
            }

            match c {
                '\n' | '\r' => {
                    state.comment = false;
                    return if state.began_num { Action::Append } else { Action::None };
                }
                'p' => state.header = true,
                'c' => {
                    if !state.header {
                        state.comment = true;
                    }
                }
                ' ' => {
                    return if state.began_num { Action::Append } else { Action::None };
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
            Action::None
        });

        if matches!(action, Action::Append) {
            self.append_integer();
        }
        false
    }
}

fn ints_to_bytes(values: &[i32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<i32>());
    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    bytes
}
