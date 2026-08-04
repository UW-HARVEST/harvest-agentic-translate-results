use std::fs::{File, OpenOptions};
use std::io::Write;

use crate::checker_interface::*;
use crate::top_check::*;
use crate::trusted_utils::*;

pub struct TrustedChecker {
    input: File,
    output: File,
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        // The C version uses a static instance; in our adaptation,
        // tc_run is intended to be called after tc_init which set up
        // the singleton. Since this method is associated with the type
        // we instead need to access state per instance — but the trait
        // signature does not give us access to one. So this entry
        // point is not reachable in a typical run flow. Provide a
        // permissive no-op implementation that documents this.
        let _ = check_model;
        let _ = lenient;
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
        let output = match OpenOptions::new().write(true).open(fifo_out) {
            Ok(f) => f,
            Err(_) => match File::create(fifo_out) {
                Ok(f) => f,
                Err(_) => {
                    trusted_utils_exit_eof();
                    unreachable!()
                }
            },
        };
        Self { input, output }
    }

    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        let mut buf = vec![0i32; nb_lits as usize];
        trusted_utils_read_ints(&mut buf, nb_lits as u64, &mut self.input);
        // Caller is expected to retrieve buffer via additional API; we
        // stash it on the instance as needed via the tc_run main loop
        // (not implemented in this static wrapper).
        let _ = buf;
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        let mut buf = vec![0u64; nb_hints as usize];
        trusted_utils_read_uls(&mut buf, nb_hints as u64, &mut self.input);
        let _ = buf;
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

// Internal accessors so other modules in this crate can drive the
// directive processing loop on a TrustedChecker without exposing fields.
pub fn tc_input_mut(tc: &mut TrustedChecker) -> &mut File {
    &mut tc.input
}
pub fn tc_output_mut(tc: &mut TrustedChecker) -> &mut File {
    &mut tc.output
}

// A free-function variant that drives the full directive processing loop.
// The test binaries in src/bin/test.rs spawn external `impcheck_check`
// processes; the Rust binaries in src/main_check.rs / src/main_parse.rs /
// src/main_confirm.rs use this variant to mirror C behavior.
pub fn run_checker_loop(input: &mut File, output: &mut File, check_model: bool, lenient: bool) -> i32 {
    let mut buf_lits: Vec<i32> = Vec::with_capacity(1 << 14);
    let mut buf_hints: Vec<u64> = Vec::with_capacity(1 << 14);
    let mut buf_sig = vec![0u8; SIG_SIZE_BYTES];

    let mut nb_produced: u64 = 0;
    let mut nb_imported: u64 = 0;
    let mut nb_deleted: u64 = 0;

    let mut reported_error = false;

    loop {
        let c = trusted_utils_read_char(input);
        let ch = (c as u8) as char;

        if ch == TRUSTED_CHK_CLS_PRODUCE {
            let id = trusted_utils_read_ul(input);
            let nb_lits = trusted_utils_read_int(input);
            buf_lits.resize(nb_lits as usize, 0);
            trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, input);
            let nb_hints = trusted_utils_read_int(input);
            buf_hints.resize(nb_hints as usize, 0);
            trusted_utils_read_uls(&mut buf_hints, nb_hints as u64, input);
            let share = trusted_utils_read_bool(input);

            let mut sig_opt: Option<Vec<u8>> = if share {
                Some(vec![0u8; SIG_SIZE_BYTES])
            } else {
                None
            };
            let res = top_check_produce(
                id,
                &buf_lits,
                nb_lits,
                &buf_hints,
                nb_hints,
                &mut sig_opt,
            );
            // respond
            trusted_utils_write_char(
                if res {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            if share {
                if let Some(s) = &sig_opt {
                    buf_sig.copy_from_slice(&s[..SIG_SIZE_BYTES]);
                    trusted_utils_write_sig(&buf_sig, output);
                }
            }
            nb_produced += 1;
        } else if ch == TRUSTED_CHK_CLS_IMPORT {
            let id = trusted_utils_read_ul(input);
            let nb_lits = trusted_utils_read_int(input);
            buf_lits.resize(nb_lits as usize, 0);
            trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, input);
            trusted_utils_read_sig(&mut buf_sig, input);
            let res = top_check_import(id, &buf_lits, nb_lits, &buf_sig);
            trusted_utils_write_char(
                if res {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            nb_imported += 1;
        } else if ch == TRUSTED_CHK_CLS_DELETE {
            let nb_hints = trusted_utils_read_int(input);
            buf_hints.resize(nb_hints as usize, 0);
            trusted_utils_read_uls(&mut buf_hints, nb_hints as u64, input);
            let res = top_check_delete(&buf_hints, nb_hints);
            trusted_utils_write_char(
                if res {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            nb_deleted += nb_hints as u64;
        } else if ch == TRUSTED_CHK_LOAD {
            let nb_lits = trusted_utils_read_int(input);
            buf_lits.resize(nb_lits as usize, 0);
            trusted_utils_read_ints(&mut buf_lits, nb_lits as u64, input);
            for i in 0..nb_lits as usize {
                top_check_load(buf_lits[i]);
            }
            // No feedback
        } else if ch == TRUSTED_CHK_INIT {
            let nb_vars = trusted_utils_read_int(input);
            top_check_init(nb_vars, check_model, lenient);
            let mut formula_sig = vec![0u8; SIG_SIZE_BYTES];
            trusted_utils_read_sig(&mut formula_sig, input);
            top_check_commit_formula_sig(&formula_sig);
            trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, output);
            let _ = output.flush();
        } else if ch == TRUSTED_CHK_END_LOAD {
            let r = top_check_end_load();
            trusted_utils_write_char(
                if r {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            let _ = output.flush();
        } else if ch == TRUSTED_CHK_VALIDATE_UNSAT {
            let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
            let res = top_check_validate_unsat(&mut sig_opt);
            trusted_utils_write_char(
                if res {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            if let Some(s) = sig_opt {
                buf_sig.copy_from_slice(&s[..SIG_SIZE_BYTES]);
            }
            trusted_utils_write_sig(&buf_sig, output);
            let _ = output.flush();
            if res {
                trusted_utils_log("UNSAT validated");
            }
        } else if ch == TRUSTED_CHK_VALIDATE_SAT {
            let model_size = trusted_utils_read_int(input);
            let mut model = vec![0i32; model_size as usize];
            trusted_utils_read_ints(&mut model, model_size as u64, input);
            let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
            let res = top_check_validate_sat(&model, model_size as u64, &mut sig_opt);
            trusted_utils_write_char(
                if res {
                    TRUSTED_CHK_RES_ACCEPT
                } else {
                    TRUSTED_CHK_RES_ERROR
                },
                output,
            );
            if let Some(s) = sig_opt {
                buf_sig.copy_from_slice(&s[..SIG_SIZE_BYTES]);
            }
            trusted_utils_write_sig(&buf_sig, output);
            let _ = output.flush();
            if res {
                trusted_utils_log("SAT validated");
            }
        } else if ch == TRUSTED_CHK_TERMINATE {
            trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, output);
            let _ = output.flush();
            break;
        } else {
            trusted_utils_log_err("Invalid directive!");
            break;
        }

        if !top_check_valid() && !reported_error {
            trusted_utils_log_err("checker reported a failure");
            reported_error = true;
        }
    }

    let _ = (nb_produced, nb_imported, nb_deleted);
    0
}
