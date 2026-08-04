use std::fs::{File, OpenOptions};
use std::io::Write;

use crate::checker_interface::{
    TRUSTED_CHK_CLS_DELETE, TRUSTED_CHK_CLS_IMPORT, TRUSTED_CHK_CLS_PRODUCE,
    TRUSTED_CHK_END_LOAD, TRUSTED_CHK_INIT, TRUSTED_CHK_LOAD, TRUSTED_CHK_RES_ACCEPT,
    TRUSTED_CHK_RES_ERROR, TRUSTED_CHK_TERMINATE, TRUSTED_CHK_VALIDATE_SAT,
    TRUSTED_CHK_VALIDATE_UNSAT,
};
use crate::top_check::{
    top_check_commit_formula_sig, top_check_delete, top_check_end_load, top_check_import,
    top_check_init, top_check_load, top_check_produce, top_check_valid,
    top_check_validate_sat, top_check_validate_unsat,
};
use crate::trusted_utils::{
    trusted_utils_exit_eof, trusted_utils_log, trusted_utils_log_err,
    trusted_utils_read_bool, trusted_utils_read_char, trusted_utils_read_int,
    trusted_utils_read_ints, trusted_utils_read_sig, trusted_utils_read_ul,
    trusted_utils_read_uls, trusted_utils_write_char, trusted_utils_write_sig,
    SIG_SIZE_BYTES,
};

pub struct TrustedChecker {
    input: File,
    output: File,
}
impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        // The C version operates on globals; here we re-create the checker
        // and run the directive loop on it.
        let mut tc = TrustedChecker::tc_init("", "");
        let start = std::time::Instant::now();

        let mut nb_produced: u64 = 0;
        let mut nb_imported: u64 = 0;
        let mut nb_deleted: u64 = 0;
        let mut reported_error = false;

        let mut buf_lits: Vec<i32> = Vec::with_capacity(1 << 14);
        let mut buf_hints: Vec<u64> = Vec::with_capacity(1 << 14);
        let mut buf_sig: Vec<u8> = vec![0u8; SIG_SIZE_BYTES];

        loop {
            let c = trusted_utils_read_char(&mut tc.input);
            let ch = c as u8 as char;
            if ch == TRUSTED_CHK_CLS_PRODUCE {
                let id = trusted_utils_read_ul(&mut tc.input);
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                tc.read_literals_into(&mut buf_lits, nb_lits);
                let nb_hints = trusted_utils_read_int(&mut tc.input);
                tc.read_hints_into(&mut buf_hints, nb_hints);
                let share = trusted_utils_read_bool(&mut tc.input);
                let mut out_sig: Option<Vec<u8>> =
                    if share { Some(vec![0u8; SIG_SIZE_BYTES]) } else { None };
                let res = top_check_produce(
                    id,
                    &buf_lits,
                    nb_lits,
                    &buf_hints,
                    nb_hints,
                    &mut out_sig,
                );
                tc.say(res);
                if let Some(sig) = out_sig {
                    buf_sig = sig;
                    trusted_utils_write_sig(&buf_sig, &mut tc.output);
                }
                nb_produced += 1;
            } else if ch == TRUSTED_CHK_CLS_IMPORT {
                let id = trusted_utils_read_ul(&mut tc.input);
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                tc.read_literals_into(&mut buf_lits, nb_lits);
                trusted_utils_read_sig(&mut buf_sig, &mut tc.input);
                let res = top_check_import(id, &buf_lits, nb_lits, &buf_sig);
                tc.say(res);
                nb_imported += 1;
            } else if ch == TRUSTED_CHK_CLS_DELETE {
                let nb_hints = trusted_utils_read_int(&mut tc.input);
                tc.read_hints_into(&mut buf_hints, nb_hints);
                let res = top_check_delete(&buf_hints, nb_hints);
                tc.say(res);
                nb_deleted += nb_hints as u64;
            } else if ch == TRUSTED_CHK_LOAD {
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                tc.read_literals_into(&mut buf_lits, nb_lits);
                for i in 0..(nb_lits as usize) {
                    top_check_load(buf_lits[i]);
                }
            } else if ch == TRUSTED_CHK_INIT {
                let nb_vars = trusted_utils_read_int(&mut tc.input);
                top_check_init(nb_vars, check_model, lenient);
                let mut formula_sig = vec![0u8; SIG_SIZE_BYTES];
                trusted_utils_read_sig(&mut formula_sig, &mut tc.input);
                top_check_commit_formula_sig(&formula_sig);
                tc.say_with_flush(true);
            } else if ch == TRUSTED_CHK_END_LOAD {
                tc.say_with_flush(top_check_end_load());
            } else if ch == TRUSTED_CHK_VALIDATE_UNSAT {
                let mut out_sig: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check_validate_unsat(&mut out_sig);
                tc.say(res);
                if let Some(sig) = &out_sig {
                    trusted_utils_write_sig(sig, &mut tc.output);
                }
                let _ = tc.output.flush();
                if res {
                    trusted_utils_log("UNSAT validated");
                }
            } else if ch == TRUSTED_CHK_VALIDATE_SAT {
                let model_size = trusted_utils_read_int(&mut tc.input);
                let mut model: Vec<i32> = vec![0i32; model_size as usize];
                trusted_utils_read_ints(&mut model, model_size as u64, &mut tc.input);
                let mut out_sig: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check_validate_sat(&model, model_size as u64, &mut out_sig);
                tc.say(res);
                if let Some(sig) = &out_sig {
                    trusted_utils_write_sig(sig, &mut tc.output);
                }
                let _ = tc.output.flush();
                if res {
                    trusted_utils_log("SAT validated");
                }
            } else if ch == TRUSTED_CHK_TERMINATE {
                tc.say_with_flush(true);
                break;
            } else {
                trusted_utils_log_err("Invalid directive!");
                break;
            }

            if !top_check_valid() && !reported_error {
                trusted_utils_log_err("checker reported an error");
                reported_error = true;
            }
        }

        let elapsed = start.elapsed().as_secs_f32();
        trusted_utils_log(&format!(
            "cpu:{:.3} prod:{} imp:{} del:{}",
            elapsed, nb_produced, nb_imported, nb_deleted
        ));
        0
    }
    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = match File::open(fifo_in) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!()
            }
        };
        let output = match OpenOptions::new().write(true).create(true).open(fifo_out) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!()
            }
        };
        TrustedChecker { input, output }
    }
    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
        // input/output close on drop
    }
    pub fn read_literals(&mut self, nb_lits: i32) {
        let mut buf = vec![0i32; nb_lits as usize];
        trusted_utils_read_ints(&mut buf, nb_lits as u64, &mut self.input);
    }
    pub fn read_hints(&mut self, nb_hints: i32) {
        let mut buf = vec![0u64; nb_hints as usize];
        trusted_utils_read_uls(&mut buf, nb_hints as u64, &mut self.input);
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
}

impl TrustedChecker {
    fn read_literals_into(&mut self, buf: &mut Vec<i32>, nb_lits: i32) {
        buf.clear();
        buf.resize(nb_lits as usize, 0);
        trusted_utils_read_ints(buf, nb_lits as u64, &mut self.input);
    }
    fn read_hints_into(&mut self, buf: &mut Vec<u64>, nb_hints: i32) {
        buf.clear();
        buf.resize(nb_hints as usize, 0);
        trusted_utils_read_uls(buf, nb_hints as u64, &mut self.input);
    }
}
