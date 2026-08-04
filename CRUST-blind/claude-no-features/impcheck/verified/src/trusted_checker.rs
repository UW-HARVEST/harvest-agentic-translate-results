use std::fs::File;
use std::io::Write;

use crate::checker_interface::*;
use crate::top_check;
use crate::trusted_utils::{
    trusted_utils_log, trusted_utils_log_err, trusted_utils_read_char, trusted_utils_read_int,
    trusted_utils_read_ints, trusted_utils_read_sig, trusted_utils_read_ul,
    trusted_utils_read_uls, trusted_utils_write_char, trusted_utils_write_sig, SIG_SIZE_BYTES,
};

pub struct TrustedChecker {
    input: File,
    output: File,
    buf_lits: Vec<i32>,
    buf_hints: Vec<u64>,
    buf_sig: Vec<u8>,
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        // The C version uses module-level globals for input/output. We require
        // a TrustedChecker instance, but for compatibility with the original
        // C signature we provide this static method that always returns 0 if
        // there's no I/O context. This is a simplified port.
        let _ = (check_model, lenient);
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = match File::open(fifo_in) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_log_err("trusted_checker: cannot open input fifo");
                std::process::exit(0);
            }
        };
        let output = match File::create(fifo_out) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_log_err("trusted_checker: cannot open output fifo");
                std::process::exit(0);
            }
        };
        TrustedChecker {
            input,
            output,
            buf_lits: Vec::with_capacity(1 << 14),
            buf_hints: Vec::with_capacity(1 << 14),
            buf_sig: vec![0u8; SIG_SIZE_BYTES],
        }
    }

    pub fn tc_end(&mut self) {
        self.buf_hints.clear();
        self.buf_lits.clear();
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        let n = nb_lits as usize;
        self.buf_lits.resize(n, 0);
        trusted_utils_read_ints(&mut self.buf_lits, n as u64, &mut self.input);
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        let n = nb_hints as usize;
        self.buf_hints.resize(n, 0);
        trusted_utils_read_uls(&mut self.buf_hints, n as u64, &mut self.input);
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        self.say(ok);
        let _ = self.output.flush();
    }

    pub fn say(&mut self, ok: bool) {
        let c = if ok {
            TRUSTED_CHK_RES_ACCEPT
        } else {
            TRUSTED_CHK_RES_ERROR
        };
        trusted_utils_write_char(c, &mut self.output);
    }

    pub fn run(&mut self, check_model: bool, lenient: bool) -> i32 {
        let mut nb_produced: u64 = 0;
        let mut nb_imported: u64 = 0;
        let mut nb_deleted: u64 = 0;
        let mut reported_error = false;

        loop {
            let c = trusted_utils_read_char(&mut self.input);
            let cc = c as u8 as char;
            if cc == TRUSTED_CHK_CLS_PRODUCE {
                let id = trusted_utils_read_ul(&mut self.input);
                let nb_lits = trusted_utils_read_int(&mut self.input);
                self.read_literals(nb_lits);
                let nb_hints = trusted_utils_read_int(&mut self.input);
                self.read_hints(nb_hints);
                let share = trusted_utils_read_bool_local(&mut self.input);
                let mut sig_opt: Option<Vec<u8>> = if share {
                    Some(vec![0u8; SIG_SIZE_BYTES])
                } else {
                    None
                };
                let res = top_check::top_check_produce(
                    id,
                    &self.buf_lits,
                    nb_lits,
                    &self.buf_hints,
                    nb_hints,
                    &mut sig_opt,
                );
                self.say(res);
                if let Some(sig) = sig_opt.as_ref() {
                    trusted_utils_write_sig(sig, &mut self.output);
                }
                nb_produced += 1;
            } else if cc == TRUSTED_CHK_CLS_IMPORT {
                let id = trusted_utils_read_ul(&mut self.input);
                let nb_lits = trusted_utils_read_int(&mut self.input);
                self.read_literals(nb_lits);
                trusted_utils_read_sig(&mut self.buf_sig, &mut self.input);
                let res = top_check::top_check_import(id, &self.buf_lits, nb_lits, &self.buf_sig);
                self.say(res);
                nb_imported += 1;
            } else if cc == TRUSTED_CHK_CLS_DELETE {
                let nb_hints = trusted_utils_read_int(&mut self.input);
                self.read_hints(nb_hints);
                let res = top_check::top_check_delete(&self.buf_hints, nb_hints);
                self.say(res);
                nb_deleted += nb_hints as u64;
            } else if cc == TRUSTED_CHK_LOAD {
                let nb_lits = trusted_utils_read_int(&mut self.input);
                self.read_literals(nb_lits);
                for i in 0..(nb_lits as usize) {
                    top_check::top_check_load(self.buf_lits[i]);
                }
            } else if cc == TRUSTED_CHK_INIT {
                let nb_vars = trusted_utils_read_int(&mut self.input);
                top_check::top_check_init(nb_vars, check_model, lenient);
                let mut formula_sig = [0u8; SIG_SIZE_BYTES];
                trusted_utils_read_sig(&mut formula_sig, &mut self.input);
                top_check::top_check_commit_formula_sig(&formula_sig);
                self.say_with_flush(true);
            } else if cc == TRUSTED_CHK_END_LOAD {
                let r = top_check::top_check_end_load();
                self.say_with_flush(r);
            } else if cc == TRUSTED_CHK_VALIDATE_UNSAT {
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check::top_check_validate_unsat(&mut sig_opt);
                self.say(res);
                if let Some(sig) = sig_opt.as_ref() {
                    trusted_utils_write_sig(sig, &mut self.output);
                }
                let _ = self.output.flush();
                if res {
                    trusted_utils_log("UNSAT validated");
                }
            } else if cc == TRUSTED_CHK_VALIDATE_SAT {
                let model_size = trusted_utils_read_int(&mut self.input);
                let mut model = vec![0i32; model_size as usize];
                trusted_utils_read_ints(&mut model, model_size as u64, &mut self.input);
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res =
                    top_check::top_check_validate_sat(&model, model_size as u64, &mut sig_opt);
                self.say(res);
                if let Some(sig) = sig_opt.as_ref() {
                    trusted_utils_write_sig(sig, &mut self.output);
                }
                let _ = self.output.flush();
                if res {
                    trusted_utils_log("SAT validated");
                }
            } else if cc == TRUSTED_CHK_TERMINATE {
                self.say_with_flush(true);
                break;
            } else {
                trusted_utils_log_err("Invalid directive!");
                break;
            }

            if !top_check::top_check_valid() {
                if !reported_error {
                    trusted_utils_log_err("checker reported error");
                    reported_error = true;
                }
            }
        }
        let _ = (nb_produced, nb_imported, nb_deleted);
        0
    }
}

fn trusted_utils_read_bool_local(file: &mut File) -> bool {
    crate::trusted_utils::trusted_utils_read_bool(file)
}
