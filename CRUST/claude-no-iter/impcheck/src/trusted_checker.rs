// Note: This file is not included via `lib.rs`. It is provided for
// completeness of the C-to-Rust translation.
#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write;

use crate::checker_interface::*;
use crate::top_check;
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

fn open_read(path: &str) -> File {
    match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            trusted_utils_exit_eof();
            unreachable!()
        }
    }
}

fn open_write(path: &str) -> File {
    match OpenOptions::new().write(true).create(true).truncate(true).open(path) {
        Ok(f) => f,
        Err(_) => {
            trusted_utils_exit_eof();
            unreachable!()
        }
    }
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        let fifo_in = std::env::var("IMPCHECK_FIFO_IN").unwrap_or_default();
        let fifo_out = std::env::var("IMPCHECK_FIFO_OUT").unwrap_or_default();
        let mut tc = if !fifo_in.is_empty() && !fifo_out.is_empty() {
            TrustedChecker::tc_init(&fifo_in, &fifo_out)
        } else {
            // Cannot meaningfully run without fifos.
            return 0;
        };
        let mut nb_produced: u64 = 0;
        let mut nb_imported: u64 = 0;
        let mut nb_deleted: u64 = 0;
        let mut reported_error = false;
        let mut buf_lits: Vec<i32> = vec![0; 1 << 14];
        let mut buf_hints: Vec<u64> = vec![0; 1 << 14];
        let mut buf_sig = vec![0u8; SIG_SIZE_BYTES];
        loop {
            let c = trusted_utils_read_char(&mut tc.input);
            if c == TRUSTED_CHK_CLS_PRODUCE as i32 {
                let id = trusted_utils_read_ul(&mut tc.input);
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                if buf_lits.len() < nb_lits as usize {
                    buf_lits.resize(nb_lits as usize, 0);
                }
                trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, &mut tc.input);
                let nb_hints = trusted_utils_read_int(&mut tc.input);
                if buf_hints.len() < nb_hints as usize {
                    buf_hints.resize(nb_hints as usize, 0);
                }
                trusted_utils_read_uls(&mut buf_hints, nb_hints as u64, &mut tc.input);
                let share = trusted_utils_read_bool(&mut tc.input);
                let mut sig_out: Option<Vec<u8>> = if share { Some(vec![0u8; SIG_SIZE_BYTES]) } else { None };
                let res = top_check::top_check_produce(
                    id, &buf_lits, nb_lits, &buf_hints, nb_hints, &mut sig_out,
                );
                tc.say(res);
                if let Some(sig) = sig_out {
                    buf_sig.copy_from_slice(&sig);
                    trusted_utils_write_sig(&buf_sig, &mut tc.output);
                }
                nb_produced += 1;
            } else if c == TRUSTED_CHK_CLS_IMPORT as i32 {
                let id = trusted_utils_read_ul(&mut tc.input);
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                if buf_lits.len() < nb_lits as usize {
                    buf_lits.resize(nb_lits as usize, 0);
                }
                trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, &mut tc.input);
                trusted_utils_read_sig(&mut buf_sig, &mut tc.input);
                let res = top_check::top_check_import(id, &buf_lits, nb_lits, &buf_sig);
                tc.say(res);
                nb_imported += 1;
            } else if c == TRUSTED_CHK_CLS_DELETE as i32 {
                let nb_hints = trusted_utils_read_int(&mut tc.input);
                if buf_hints.len() < nb_hints as usize {
                    buf_hints.resize(nb_hints as usize, 0);
                }
                trusted_utils_read_uls(&mut buf_hints, nb_hints as u64, &mut tc.input);
                let res = top_check::top_check_delete(&buf_hints, nb_hints);
                tc.say(res);
                nb_deleted += nb_hints as u64;
            } else if c == TRUSTED_CHK_LOAD as i32 {
                let nb_lits = trusted_utils_read_int(&mut tc.input);
                if buf_lits.len() < nb_lits as usize {
                    buf_lits.resize(nb_lits as usize, 0);
                }
                trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, &mut tc.input);
                for i in 0..nb_lits as usize {
                    top_check::top_check_load(buf_lits[i]);
                }
            } else if c == TRUSTED_CHK_INIT as i32 {
                let nb_vars = trusted_utils_read_int(&mut tc.input);
                top_check::top_check_init(nb_vars, check_model, lenient);
                let mut formula_sig = vec![0u8; SIG_SIZE_BYTES];
                trusted_utils_read_sig(&mut formula_sig, &mut tc.input);
                top_check::top_check_commit_formula_sig(&formula_sig);
                tc.say_with_flush(true);
            } else if c == TRUSTED_CHK_END_LOAD as i32 {
                let ok = top_check::top_check_end_load();
                tc.say_with_flush(ok);
            } else if c == TRUSTED_CHK_VALIDATE_UNSAT as i32 {
                let mut sig_out: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check::top_check_validate_unsat(&mut sig_out);
                tc.say(res);
                if let Some(sig) = sig_out {
                    buf_sig.copy_from_slice(&sig);
                }
                trusted_utils_write_sig(&buf_sig, &mut tc.output);
                let _ = tc.output.flush();
                if res {
                    trusted_utils_log("UNSAT validated");
                }
            } else if c == TRUSTED_CHK_VALIDATE_SAT as i32 {
                let model_size = trusted_utils_read_int(&mut tc.input);
                let mut model = vec![0i32; model_size as usize];
                trusted_utils_read_ints(&mut model, model_size as u64, &mut tc.input);
                let mut sig_out: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check::top_check_validate_sat(&model, model_size as u64, &mut sig_out);
                tc.say(res);
                if let Some(sig) = sig_out {
                    buf_sig.copy_from_slice(&sig);
                }
                trusted_utils_write_sig(&buf_sig, &mut tc.output);
                let _ = tc.output.flush();
                if res {
                    trusted_utils_log("SAT validated");
                }
            } else if c == TRUSTED_CHK_TERMINATE as i32 {
                tc.say_with_flush(true);
                break;
            } else {
                trusted_utils_log_err("Invalid directive!");
                break;
            }
            if !top_check::top_check_valid() && !reported_error {
                trusted_utils_log_err("error in checker");
                reported_error = true;
            }
        }
        let _ = nb_produced;
        let _ = nb_imported;
        let _ = nb_deleted;
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = open_read(fifo_in);
        let output = open_write(fifo_out);
        TrustedChecker { input, output }
    }

    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
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
        let c = if ok { TRUSTED_CHK_RES_ACCEPT } else { TRUSTED_CHK_RES_ERROR };
        trusted_utils_write_char(c, &mut self.output);
    }
}
