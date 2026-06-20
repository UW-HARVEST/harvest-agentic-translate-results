use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

use crate::checker_interface::{
    TRUSTED_CHK_CLS_DELETE, TRUSTED_CHK_CLS_IMPORT, TRUSTED_CHK_CLS_PRODUCE, TRUSTED_CHK_END_LOAD,
    TRUSTED_CHK_INIT, TRUSTED_CHK_LOAD, TRUSTED_CHK_RES_ACCEPT, TRUSTED_CHK_RES_ERROR,
    TRUSTED_CHK_TERMINATE, TRUSTED_CHK_VALIDATE_SAT, TRUSTED_CHK_VALIDATE_UNSAT,
};
use crate::top_check::{
    top_check_commit_formula_sig, top_check_delete, top_check_end_load, top_check_import,
    top_check_init, top_check_load, top_check_produce, top_check_valid, top_check_validate_sat,
    top_check_validate_unsat,
};
use crate::trusted_utils::{
    trusted_utils_get_msg, trusted_utils_log, trusted_utils_log_err, trusted_utils_read_bool,
    trusted_utils_read_char, trusted_utils_read_int, trusted_utils_read_ints, trusted_utils_read_sig,
    trusted_utils_read_ul, trusted_utils_read_uls, trusted_utils_write_char, trusted_utils_write_sig,
    SIG_SIZE_BYTES,
};

thread_local! {
    static STATE: RefCell<Option<CheckerState>> = const { RefCell::new(None) };
}

struct CheckerState {
    input: File,
    output: File,
    nb_vars: i32,
    formula_sig: [u8; SIG_SIZE_BYTES],
    buf_sig: [u8; SIG_SIZE_BYTES],
    buf_lits: Vec<i32>,
    buf_hints: Vec<u64>,
}

pub struct TrustedChecker {
    input: File,
    output: File,
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        let start = Instant::now();
        let mut nb_produced = 0u64;
        let mut nb_imported = 0u64;
        let mut nb_deleted = 0u64;
        let mut reported_error = false;

        loop {
            let c = STATE.with(|state| {
                let mut state = state.borrow_mut();
                trusted_utils_read_char(&mut state.as_mut().unwrap().input)
            });

            if c == TRUSTED_CHK_CLS_PRODUCE as i32 {
                let (id, nb_lits, nb_hints, share, lits, hints) = STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let id = trusted_utils_read_ul(&mut state.input);
                    let nb_lits = trusted_utils_read_int(&mut state.input);
                    state.buf_lits.resize(nb_lits as usize, 0);
                    trusted_utils_read_ints(&mut state.buf_lits, nb_lits as u64, &mut state.input);
                    let nb_hints = trusted_utils_read_int(&mut state.input);
                    state.buf_hints.resize(nb_hints as usize, 0);
                    trusted_utils_read_uls(&mut state.buf_hints, nb_hints as u64, &mut state.input);
                    let share = trusted_utils_read_bool(&mut state.input);
                    (
                        id,
                        nb_lits,
                        nb_hints,
                        share,
                        state.buf_lits.clone(),
                        state.buf_hints.clone(),
                    )
                });
                let mut out_sig = if share { Some(vec![0u8; SIG_SIZE_BYTES]) } else { None };
                let res = top_check_produce(id, &lits, nb_lits, &hints, nb_hints, &mut out_sig);
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                    if let Some(sig) = out_sig {
                        trusted_utils_write_sig(&sig, &mut state.output);
                    }
                    state.output.flush().ok();
                });
                nb_produced += 1;
            } else if c == TRUSTED_CHK_CLS_IMPORT as i32 {
                let (id, nb_lits, lits, sig) = STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let id = trusted_utils_read_ul(&mut state.input);
                    let nb_lits = trusted_utils_read_int(&mut state.input);
                    state.buf_lits.resize(nb_lits as usize, 0);
                    trusted_utils_read_ints(&mut state.buf_lits, nb_lits as u64, &mut state.input);
                    trusted_utils_read_sig(&mut state.buf_sig, &mut state.input);
                    (id, nb_lits, state.buf_lits.clone(), state.buf_sig)
                });
                let res = top_check_import(id, &lits, nb_lits, &sig);
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                });
                nb_imported += 1;
            } else if c == TRUSTED_CHK_CLS_DELETE as i32 {
                let (nb_hints, hints) = STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let nb_hints = trusted_utils_read_int(&mut state.input);
                    state.buf_hints.resize(nb_hints as usize, 0);
                    trusted_utils_read_uls(&mut state.buf_hints, nb_hints as u64, &mut state.input);
                    (nb_hints, state.buf_hints.clone())
                });
                let res = top_check_delete(&hints, nb_hints);
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                });
                nb_deleted += nb_hints as u64;
            } else if c == TRUSTED_CHK_LOAD as i32 {
                let lits = STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let nb_lits = trusted_utils_read_int(&mut state.input);
                    state.buf_lits.resize(nb_lits as usize, 0);
                    trusted_utils_read_ints(&mut state.buf_lits, nb_lits as u64, &mut state.input);
                    state.buf_lits.clone()
                });
                for lit in lits {
                    top_check_load(lit);
                }
            } else if c == TRUSTED_CHK_INIT as i32 {
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    state.nb_vars = trusted_utils_read_int(&mut state.input);
                    top_check_init(state.nb_vars, check_model, lenient);
                    trusted_utils_read_sig(&mut state.formula_sig, &mut state.input);
                    top_check_commit_formula_sig(&state.formula_sig);
                    trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, &mut state.output);
                    state.output.flush().ok();
                });
            } else if c == TRUSTED_CHK_END_LOAD as i32 {
                let res = top_check_end_load();
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                    state.output.flush().ok();
                });
            } else if c == TRUSTED_CHK_VALIDATE_UNSAT as i32 {
                let mut out_sig = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check_validate_unsat(&mut out_sig);
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                    trusted_utils_write_sig(out_sig.as_ref().unwrap(), &mut state.output);
                    state.output.flush().ok();
                });
                if res {
                    trusted_utils_log("UNSAT validated");
                }
            } else if c == TRUSTED_CHK_VALIDATE_SAT as i32 {
                let model = STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let model_size = trusted_utils_read_int(&mut state.input);
                    let mut model = vec![0i32; model_size as usize];
                    trusted_utils_read_ints(&mut model, model_size as u64, &mut state.input);
                    model
                });
                let mut out_sig = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check_validate_sat(&model, model.len() as u64, &mut out_sig);
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(
                        if res {
                            TRUSTED_CHK_RES_ACCEPT
                        } else {
                            TRUSTED_CHK_RES_ERROR
                        },
                        &mut state.output,
                    );
                    trusted_utils_write_sig(out_sig.as_ref().unwrap(), &mut state.output);
                    state.output.flush().ok();
                });
                if res {
                    trusted_utils_log("SAT validated");
                }
            } else if c == TRUSTED_CHK_TERMINATE as i32 {
                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap();
                    trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, &mut state.output);
                    state.output.flush().ok();
                });
                break;
            } else {
                trusted_utils_log_err("Invalid directive!");
                break;
            }

            if !top_check_valid() && !reported_error {
                trusted_utils_log_err(&trusted_utils_get_msg());
                reported_error = true;
            }
        }

        trusted_utils_log(&format!(
            "cpu:{:.3} prod:{} imp:{} del:{}",
            start.elapsed().as_secs_f32(),
            nb_produced,
            nb_imported,
            nb_deleted
        ));
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).unwrap();
        let output = File::create(fifo_out).unwrap();
        let state_input = input.try_clone().unwrap();
        let state_output = output.try_clone().unwrap();
        STATE.with(|state| {
            *state.borrow_mut() = Some(CheckerState {
                input: state_input,
                output: state_output,
                nb_vars: 0,
                formula_sig: [0; SIG_SIZE_BYTES],
                buf_sig: [0; SIG_SIZE_BYTES],
                buf_lits: Vec::with_capacity(1 << 14),
                buf_hints: Vec::with_capacity(1 << 14),
            });
        });
        Self { input, output }
    }

    pub fn tc_end(&mut self) {
        STATE.with(|state| {
            *state.borrow_mut() = None;
        });
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        let mut lits = vec![0i32; nb_lits as usize];
        trusted_utils_read_ints(&mut lits, nb_lits as u64, &mut self.input);
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        let mut hints = vec![0u64; nb_hints as usize];
        trusted_utils_read_uls(&mut hints, nb_hints as u64, &mut self.input);
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        self.say(ok);
        self.output.flush().ok();
    }

    pub fn say(&mut self, ok: bool) {
        trusted_utils_write_char(
            if ok {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            },
            &mut self.output,
        );
    }
}
