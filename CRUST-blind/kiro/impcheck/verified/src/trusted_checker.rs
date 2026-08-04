use std::fs::File;
use std::io::{Read, Write};
use crate::trusted_utils::{self, SIG_SIZE_BYTES};
use crate::top_check;
use crate::checker_interface::*;

pub struct TrustedChecker {
input: File,
output: File,
nb_vars: i32,
formula_sig: [u8; SIG_SIZE_BYTES],
buf_sig: [u8; SIG_SIZE_BYTES],
buf_lits: Vec<i32>,
buf_hints: Vec<u64>,
}

impl TrustedChecker {
pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
    // This is a static method but needs access to self.
    // The C code has tc_run as a standalone function using globals.
    // Since the Rust signature is static, we can't use self.
    // This will be called differently - see main_check.
    0
}

pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
    let input = File::open(fifo_in).unwrap_or_else(|_| {
        trusted_utils::trusted_utils_exit_eof();
        unreachable!()
    });
    let output = File::create(fifo_out).unwrap_or_else(|_| {
        trusted_utils::trusted_utils_exit_eof();
        unreachable!()
    });
    TrustedChecker {
        input,
        output,
        nb_vars: 0,
        formula_sig: [0u8; SIG_SIZE_BYTES],
        buf_sig: [0u8; SIG_SIZE_BYTES],
        buf_lits: Vec::with_capacity(1 << 14),
        buf_hints: Vec::with_capacity(1 << 14),
    }
}

pub fn tc_end(&mut self) {
    // Files are closed on drop
}

pub fn read_literals(&mut self, nb_lits: i32) {
    let n = nb_lits as usize;
    self.buf_lits.resize(n, 0);
    trusted_utils::trusted_utils_read_ints(&mut self.buf_lits, nb_lits as u64, &mut self.input);
}

pub fn read_hints(&mut self, nb_hints: i32) {
    let n = nb_hints as usize;
    self.buf_hints.resize(n, 0);
    trusted_utils::trusted_utils_read_uls(&mut self.buf_hints, nb_hints as u64, &mut self.input);
}

pub fn say_with_flush(&mut self, ok: bool) {
    self.say(ok);
    self.output.flush().ok();
}

pub fn say(&mut self, ok: bool) {
    let c = if ok { TRUSTED_CHK_RES_ACCEPT } else { TRUSTED_CHK_RES_ERROR };
    trusted_utils::trusted_utils_write_char(c, &mut self.output);
}

pub fn run(&mut self, check_model: bool, lenient: bool) -> i32 {
    let start = std::time::Instant::now();
    let mut nb_produced: u64 = 0;
    let mut nb_imported: u64 = 0;
    let mut nb_deleted: u64 = 0;
    let mut reported_error = false;

    loop {
        let c_val = trusted_utils::trusted_utils_read_char(&mut self.input);
        let c = c_val as u8 as char;

        if c == TRUSTED_CHK_CLS_PRODUCE {
            let id = trusted_utils::trusted_utils_read_ul(&mut self.input);
            let nb_lits = trusted_utils::trusted_utils_read_int(&mut self.input);
            self.read_literals(nb_lits);
            let nb_hints = trusted_utils::trusted_utils_read_int(&mut self.input);
            self.read_hints(nb_hints);
            let share = trusted_utils::trusted_utils_read_bool(&mut self.input);

            let mut out_sig: Option<Vec<u8>> = if share { Some(Vec::new()) } else { None };
            let res = top_check::top_check_produce(
                id, &self.buf_lits, nb_lits,
                &self.buf_hints, nb_hints, &mut out_sig,
            );
            self.say(res);
            if share {
                if let Some(ref sig) = out_sig {
                    self.buf_sig.copy_from_slice(&sig[..SIG_SIZE_BYTES]);
                }
                trusted_utils::trusted_utils_write_sig(&self.buf_sig, &mut self.output);
            }
            self.output.flush().ok();
            nb_produced += 1;

        } else if c == TRUSTED_CHK_CLS_IMPORT {
            let id = trusted_utils::trusted_utils_read_ul(&mut self.input);
            let nb_lits = trusted_utils::trusted_utils_read_int(&mut self.input);
            self.read_literals(nb_lits);
            trusted_utils::trusted_utils_read_sig(&mut self.buf_sig, &mut self.input);

            let res = top_check::top_check_import(id, &self.buf_lits, nb_lits, &self.buf_sig);
            self.say(res);
            nb_imported += 1;

        } else if c == TRUSTED_CHK_CLS_DELETE {
            let nb_hints = trusted_utils::trusted_utils_read_int(&mut self.input);
            self.read_hints(nb_hints);

            let res = top_check::top_check_delete(&self.buf_hints, nb_hints);
            self.say(res);
            nb_deleted += nb_hints as u64;

        } else if c == TRUSTED_CHK_LOAD {
            let nb_lits = trusted_utils::trusted_utils_read_int(&mut self.input);
            self.read_literals(nb_lits);
            for i in 0..nb_lits as usize {
                top_check::top_check_load(self.buf_lits[i]);
            }
            // NO FEEDBACK

        } else if c == TRUSTED_CHK_INIT {
            self.nb_vars = trusted_utils::trusted_utils_read_int(&mut self.input);
            top_check::top_check_init(self.nb_vars, check_model, lenient);
            trusted_utils::trusted_utils_read_sig(&mut self.formula_sig, &mut self.input);
            top_check::top_check_commit_formula_sig(&self.formula_sig);
            self.say_with_flush(true);

        } else if c == TRUSTED_CHK_END_LOAD {
            self.say_with_flush(top_check::top_check_end_load());

        } else if c == TRUSTED_CHK_VALIDATE_UNSAT {
            let mut out_sig: Option<Vec<u8>> = Some(Vec::new());
            let res = top_check::top_check_validate_unsat(&mut out_sig);
            self.say(res);
            if let Some(ref sig) = out_sig {
                self.buf_sig.copy_from_slice(&sig[..SIG_SIZE_BYTES]);
            }
            trusted_utils::trusted_utils_write_sig(&self.buf_sig, &mut self.output);
            self.output.flush().ok();
            if res {
                trusted_utils::trusted_utils_log("UNSAT validated");
            }

        } else if c == TRUSTED_CHK_VALIDATE_SAT {
            let model_size = trusted_utils::trusted_utils_read_int(&mut self.input);
            let mut model = vec![0i32; model_size as usize];
            trusted_utils::trusted_utils_read_ints(&mut model, model_size as u64, &mut self.input);
            let mut out_sig: Option<Vec<u8>> = Some(Vec::new());
            let res = top_check::top_check_validate_sat(&model, model_size as u64, &mut out_sig);
            self.say(res);
            if let Some(ref sig) = out_sig {
                self.buf_sig.copy_from_slice(&sig[..SIG_SIZE_BYTES]);
            }
            trusted_utils::trusted_utils_write_sig(&self.buf_sig, &mut self.output);
            self.output.flush().ok();
            if res {
                trusted_utils::trusted_utils_log("SAT validated");
            }

        } else if c == TRUSTED_CHK_TERMINATE {
            self.say_with_flush(true);
            break;

        } else {
            trusted_utils::trusted_utils_log_err("Invalid directive!");
            break;
        }

        if !top_check::top_check_valid() {
            if !reported_error {
                let msg = top_check::top_check_get_msgstr();
                trusted_utils::trusted_utils_log_err(&msg);
                reported_error = true;
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f32();
    let msg = format!("cpu:{:.3} prod:{} imp:{} del:{}", elapsed, nb_produced, nb_imported, nb_deleted);
    trusted_utils::trusted_utils_log(&msg);

    0
}
}
