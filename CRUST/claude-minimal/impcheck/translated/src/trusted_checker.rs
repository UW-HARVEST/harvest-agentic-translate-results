// Direct port of c_src/src/trusted/trusted_checker.c
//
// Note: this module is not currently included in lib.rs, so it serves as an
// "orphan" port mirroring the C implementation. The original struct only
// stores the input and output FILE handles; auxiliary buffers (buf_lits,
// buf_hints, buf_sig) are stored in thread-local globals to mirror the C
// module-level statics.

use crate::checker_interface::*;
use crate::top_check::*;
use crate::trusted_utils::*;

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;

pub struct TrustedChecker {
    input: File,
    output: File,
}

thread_local! {
    pub static BUF_LITS: RefCell<Vec<i32>> = RefCell::new(vec![0i32; 1 << 14]);
    pub static BUF_HINTS: RefCell<Vec<u64>> = RefCell::new(vec![0u64; 1 << 14]);
    pub static BUF_SIG: RefCell<[u8; SIG_SIZE_BYTES]> = RefCell::new([0u8; SIG_SIZE_BYTES]);
    pub static FORMULA_SIG: RefCell<[u8; SIG_SIZE_BYTES]> = RefCell::new([0u8; SIG_SIZE_BYTES]);
    pub static NB_VARS: RefCell<i32> = RefCell::new(0);
    pub static TC_HOLDER: RefCell<Option<TrustedChecker>> = RefCell::new(None);
}

impl TrustedChecker {
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
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!()
            }
        };
        BUF_LITS.with(|b| b.borrow_mut().clear());
        BUF_LITS.with(|b| b.borrow_mut().resize(1 << 14, 0));
        BUF_HINTS.with(|b| b.borrow_mut().clear());
        BUF_HINTS.with(|b| b.borrow_mut().resize(1 << 14, 0));
        TrustedChecker { input, output }
    }

    pub fn tc_end(&mut self) {
        BUF_HINTS.with(|b| b.borrow_mut().clear());
        BUF_LITS.with(|b| b.borrow_mut().clear());
        // The File handles are dropped automatically when self goes out of scope.
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        BUF_LITS.with(|b| {
            let mut buf = b.borrow_mut();
            if (nb_lits as usize) > buf.len() {
                buf.resize(nb_lits as usize, 0);
            }
            trusted_utils_read_ints(&mut buf, nb_lits as u64, &mut self.input);
        });
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        BUF_HINTS.with(|b| {
            let mut buf = b.borrow_mut();
            if (nb_hints as usize) > buf.len() {
                buf.resize(nb_hints as usize, 0);
            }
            trusted_utils_read_uls(&mut buf, nb_hints as u64, &mut self.input);
        });
    }

    pub fn say(&mut self, ok: bool) {
        let c = if ok {
            TRUSTED_CHK_RES_ACCEPT
        } else {
            TRUSTED_CHK_RES_ERROR
        };
        trusted_utils_write_char(c, &mut self.output);
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        self.say(ok);
        let _ = self.output.flush();
    }

    /// Run the trusted checker event loop. The C version stores `input`/`output`
    /// as module-level globals; here we store the active `TrustedChecker` in a
    /// thread-local cell so the API can match the C signature.
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        let start = Instant::now();
        let mut nb_produced: u64 = 0;
        let mut nb_imported: u64 = 0;
        let mut nb_deleted: u64 = 0;
        let mut reported_error = false;

        loop {
            let c = TC_HOLDER.with(|h| {
                let mut hopt = h.borrow_mut();
                let tc = hopt.as_mut().expect("tc not initialized");
                trusted_utils_read_char(&mut tc.input)
            });
            let cc = c as u8 as char;

            if cc == TRUSTED_CHK_CLS_PRODUCE {
                let (id, nb_lits) = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    let id = trusted_utils_read_ul(&mut tc.input);
                    let nb_lits = trusted_utils_read_int(&mut tc.input);
                    (id, nb_lits)
                });
                Self::read_literals_global(nb_lits);
                let nb_hints = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_int(&mut tc.input)
                });
                Self::read_hints_global(nb_hints);
                let share = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_bool(&mut tc.input)
                });
                let lits: Vec<i32> = BUF_LITS.with(|b| b.borrow()[..nb_lits as usize].to_vec());
                let hints: Vec<u64> =
                    BUF_HINTS.with(|b| b.borrow()[..nb_hints as usize].to_vec());
                let mut sig_opt: Option<Vec<u8>> =
                    if share { Some(vec![0u8; SIG_SIZE_BYTES]) } else { None };
                let res = top_check_produce(
                    id,
                    &lits,
                    nb_lits,
                    &hints,
                    nb_hints,
                    &mut sig_opt,
                );
                Self::say_global(res);
                if let Some(s) = sig_opt {
                    BUF_SIG.with(|b| {
                        let mut buf = b.borrow_mut();
                        for i in 0..SIG_SIZE_BYTES {
                            buf[i] = s[i];
                        }
                    });
                    let buf = BUF_SIG.with(|b| *b.borrow());
                    Self::write_sig_global(&buf);
                }
                nb_produced += 1;
            } else if cc == TRUSTED_CHK_CLS_IMPORT {
                let (id, nb_lits) = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    let id = trusted_utils_read_ul(&mut tc.input);
                    let nb_lits = trusted_utils_read_int(&mut tc.input);
                    (id, nb_lits)
                });
                Self::read_literals_global(nb_lits);
                let mut sig_buf = [0u8; SIG_SIZE_BYTES];
                TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_sig(&mut sig_buf, &mut tc.input);
                });
                let lits: Vec<i32> = BUF_LITS.with(|b| b.borrow()[..nb_lits as usize].to_vec());
                let res = top_check_import(id, &lits, nb_lits, &sig_buf);
                Self::say_global(res);
                nb_imported += 1;
            } else if cc == TRUSTED_CHK_CLS_DELETE {
                let nb_hints = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_int(&mut tc.input)
                });
                Self::read_hints_global(nb_hints);
                let hints: Vec<u64> =
                    BUF_HINTS.with(|b| b.borrow()[..nb_hints as usize].to_vec());
                let res = top_check_delete(&hints, nb_hints);
                Self::say_global(res);
                nb_deleted += nb_hints as u64;
            } else if cc == TRUSTED_CHK_LOAD {
                let nb_lits = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_int(&mut tc.input)
                });
                Self::read_literals_global(nb_lits);
                BUF_LITS.with(|b| {
                    let buf = b.borrow();
                    for i in 0..(nb_lits as usize) {
                        top_check_load(buf[i]);
                    }
                });
            } else if cc == TRUSTED_CHK_INIT {
                let nb_vars = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_int(&mut tc.input)
                });
                NB_VARS.with(|v| *v.borrow_mut() = nb_vars);
                top_check_init(nb_vars, check_model, lenient);
                let mut fsig = [0u8; SIG_SIZE_BYTES];
                TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_sig(&mut fsig, &mut tc.input);
                });
                FORMULA_SIG.with(|f| *f.borrow_mut() = fsig);
                top_check_commit_formula_sig(&fsig);
                Self::say_with_flush_global(true);
            } else if cc == TRUSTED_CHK_END_LOAD {
                let res = top_check_end_load();
                Self::say_with_flush_global(res);
            } else if cc == TRUSTED_CHK_VALIDATE_UNSAT {
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res = top_check_validate_unsat(&mut sig_opt);
                Self::say_global(res);
                if let Some(s) = sig_opt {
                    BUF_SIG.with(|b| {
                        let mut buf = b.borrow_mut();
                        for i in 0..SIG_SIZE_BYTES {
                            buf[i] = s[i];
                        }
                    });
                }
                let buf = BUF_SIG.with(|b| *b.borrow());
                Self::write_sig_global(&buf);
                Self::flush_output_global();
                if res {
                    trusted_utils_log("UNSAT validated");
                }
            } else if cc == TRUSTED_CHK_VALIDATE_SAT {
                let model_size = TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_int(&mut tc.input)
                });
                let mut model: Vec<i32> = vec![0; model_size as usize];
                TC_HOLDER.with(|h| {
                    let mut hopt = h.borrow_mut();
                    let tc = hopt.as_mut().unwrap();
                    trusted_utils_read_ints(&mut model, model_size as u64, &mut tc.input);
                });
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
                let res =
                    top_check_validate_sat(&model, model_size as u64, &mut sig_opt);
                Self::say_global(res);
                if let Some(s) = sig_opt {
                    BUF_SIG.with(|b| {
                        let mut buf = b.borrow_mut();
                        for i in 0..SIG_SIZE_BYTES {
                            buf[i] = s[i];
                        }
                    });
                }
                let buf = BUF_SIG.with(|b| *b.borrow());
                Self::write_sig_global(&buf);
                Self::flush_output_global();
                if res {
                    trusted_utils_log("SAT validated");
                }
            } else if cc == TRUSTED_CHK_TERMINATE {
                Self::say_with_flush_global(true);
                break;
            } else {
                trusted_utils_log_err("Invalid directive!");
                break;
            }

            if !top_check_valid() && !reported_error {
                crate::lrat_check::STATE.with(|s| {
                    let st = s.borrow();
                    trusted_utils_log_err(&st.msgstr);
                });
                reported_error = true;
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        let msg = format!(
            "cpu:{:.3} prod:{} imp:{} del:{}",
            elapsed, nb_produced, nb_imported, nb_deleted
        );
        trusted_utils_log(&msg);
        0
    }

    fn read_literals_global(nb_lits: i32) {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            BUF_LITS.with(|b| {
                let mut buf = b.borrow_mut();
                if (nb_lits as usize) > buf.len() {
                    buf.resize(nb_lits as usize, 0);
                }
                trusted_utils_read_ints(&mut buf, nb_lits as u64, &mut tc.input);
            });
        });
    }
    fn read_hints_global(nb_hints: i32) {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            BUF_HINTS.with(|b| {
                let mut buf = b.borrow_mut();
                if (nb_hints as usize) > buf.len() {
                    buf.resize(nb_hints as usize, 0);
                }
                trusted_utils_read_uls(&mut buf, nb_hints as u64, &mut tc.input);
            });
        });
    }
    fn say_global(ok: bool) {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            tc.say(ok);
        });
    }
    fn say_with_flush_global(ok: bool) {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            tc.say_with_flush(ok);
        });
    }
    fn write_sig_global(sig: &[u8]) {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            trusted_utils_write_sig(sig, &mut tc.output);
        });
    }
    fn flush_output_global() {
        TC_HOLDER.with(|h| {
            let mut hopt = h.borrow_mut();
            let tc = hopt.as_mut().unwrap();
            let _ = tc.output.flush();
        });
    }
}
