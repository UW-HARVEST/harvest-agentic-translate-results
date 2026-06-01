use crate::checker_interface::*;
use crate::top_check;
use crate::trusted_utils::{
    self, trusted_utils_log, trusted_utils_log_err, trusted_utils_read_bool, trusted_utils_read_char,
    trusted_utils_read_int, trusted_utils_read_ints, trusted_utils_read_sig, trusted_utils_read_ul,
    trusted_utils_read_uls, trusted_utils_write_char, trusted_utils_write_sig, SIG_SIZE_BYTES,
};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

pub struct TrustedChecker {
    input: File,
    output: File,
}

thread_local! {
    static BUF_LITS: RefCell<Vec<i32>> = RefCell::new(Vec::with_capacity(1 << 14));
    static BUF_HINTS: RefCell<Vec<u64>> = RefCell::new(Vec::with_capacity(1 << 14));
    static BUF_SIG: RefCell<Vec<u8>> = RefCell::new(vec![0u8; SIG_SIZE_BYTES]);
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        // The C code uses globals; here we open files via tc_init then pass through.
        // For our standalone use, we expect tc_init has been called first.
        // We'll grab globals through thread-local STATE in this struct? We only have
        // a Self with input/output, but tc_run is associated and not on self.
        // Mimic C behavior: there's an external "input" and "output" set by tc_init.
        unsafe {
            run_loop(check_model, lenient);
        }
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).expect("tc_init: failed to open input");
        let output = std::fs::OpenOptions::new()
            .write(true)
            .open(fifo_out)
            .expect("tc_init: failed to open output");
        unsafe {
            GLOBAL_INPUT = Some(File::open(fifo_in).expect("tc_init dup input"));
            GLOBAL_OUTPUT = Some(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(fifo_out)
                    .expect("tc_init dup output"),
            );
        }
        Self { input, output }
    }

    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
        // Files dropped at end of life
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        BUF_LITS.with(|v| {
            let mut buf = v.borrow_mut();
            buf.resize(nb_lits as usize, 0);
            trusted_utils_read_ints(&mut buf, nb_lits as u64, &mut self.input);
        });
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        BUF_HINTS.with(|v| {
            let mut buf = v.borrow_mut();
            buf.resize(nb_hints as usize, 0);
            trusted_utils_read_uls(&mut buf, nb_hints as u64, &mut self.input);
        });
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

// Globals matching the C-style file-scope statics.
static mut GLOBAL_INPUT: Option<File> = None;
static mut GLOBAL_OUTPUT: Option<File> = None;

unsafe fn run_loop(check_model: bool, lenient: bool) {
    let input = GLOBAL_INPUT.as_mut().expect("input not initialized");
    let output = GLOBAL_OUTPUT.as_mut().expect("output not initialized");

    let mut reported_error = false;

    loop {
        let c = trusted_utils_read_char(input);
        if c == TRUSTED_CHK_CLS_PRODUCE as i32 {
            let id = trusted_utils_read_ul(input);
            let nb_lits = trusted_utils_read_int(input);
            let mut lits = vec![0i32; nb_lits as usize];
            trusted_utils_read_ints(&mut lits, nb_lits as u64, input);
            let nb_hints = trusted_utils_read_int(input);
            let mut hints = vec![0u64; nb_hints as usize];
            trusted_utils_read_uls(&mut hints, nb_hints as u64, input);
            let share = trusted_utils_read_bool(input);
            let mut sig: Option<Vec<u8>> = if share { Some(vec![0u8; SIG_SIZE_BYTES]) } else { None };
            let res = top_check::top_check_produce(id, &lits, nb_lits, &hints, nb_hints, &mut sig);
            let cc = if res {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            if let Some(s) = sig {
                trusted_utils_write_sig(&s, output);
            }
            let _ = output.flush();
        } else if c == TRUSTED_CHK_CLS_IMPORT as i32 {
            let id = trusted_utils_read_ul(input);
            let nb_lits = trusted_utils_read_int(input);
            let mut lits = vec![0i32; nb_lits as usize];
            trusted_utils_read_ints(&mut lits, nb_lits as u64, input);
            let mut sig = vec![0u8; SIG_SIZE_BYTES];
            trusted_utils_read_sig(&mut sig, input);
            let res = top_check::top_check_import(id, &lits, nb_lits, &sig);
            let cc = if res {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            let _ = output.flush();
        } else if c == TRUSTED_CHK_CLS_DELETE as i32 {
            let nb_hints = trusted_utils_read_int(input);
            let mut hints = vec![0u64; nb_hints as usize];
            trusted_utils_read_uls(&mut hints, nb_hints as u64, input);
            let res = top_check::top_check_delete(&hints, nb_hints);
            let cc = if res {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            let _ = output.flush();
        } else if c == TRUSTED_CHK_LOAD as i32 {
            let nb_lits = trusted_utils_read_int(input);
            let mut lits = vec![0i32; nb_lits as usize];
            trusted_utils_read_ints(&mut lits, nb_lits as u64, input);
            for &lit in lits.iter() {
                top_check::top_check_load(lit);
            }
        } else if c == TRUSTED_CHK_INIT as i32 {
            let nb_vars = trusted_utils_read_int(input);
            top_check::top_check_init(nb_vars, check_model, lenient);
            let mut sig = vec![0u8; SIG_SIZE_BYTES];
            trusted_utils_read_sig(&mut sig, input);
            top_check::top_check_commit_formula_sig(&sig);
            trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, output);
            let _ = output.flush();
        } else if c == TRUSTED_CHK_END_LOAD as i32 {
            let ok = top_check::top_check_end_load();
            let cc = if ok {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            let _ = output.flush();
        } else if c == TRUSTED_CHK_VALIDATE_UNSAT as i32 {
            let mut sig: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
            let res = top_check::top_check_validate_unsat(&mut sig);
            let cc = if res {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            if let Some(s) = sig {
                trusted_utils_write_sig(&s, output);
            } else {
                let s = vec![0u8; SIG_SIZE_BYTES];
                trusted_utils_write_sig(&s, output);
            }
            let _ = output.flush();
            if res {
                trusted_utils_log("UNSAT validated");
            }
        } else if c == TRUSTED_CHK_VALIDATE_SAT as i32 {
            let model_size = trusted_utils_read_int(input);
            let mut model = vec![0i32; model_size as usize];
            trusted_utils_read_ints(&mut model, model_size as u64, input);
            let mut sig: Option<Vec<u8>> = Some(vec![0u8; SIG_SIZE_BYTES]);
            let res = top_check::top_check_validate_sat(&model, model_size as u64, &mut sig);
            let cc = if res {
                TRUSTED_CHK_RES_ACCEPT
            } else {
                TRUSTED_CHK_RES_ERROR
            };
            trusted_utils_write_char(cc, output);
            if let Some(s) = sig {
                trusted_utils_write_sig(&s, output);
            } else {
                let s = vec![0u8; SIG_SIZE_BYTES];
                trusted_utils_write_sig(&s, output);
            }
            let _ = output.flush();
            if res {
                trusted_utils_log("SAT validated");
            }
        } else if c == TRUSTED_CHK_TERMINATE as i32 {
            trusted_utils_write_char(TRUSTED_CHK_RES_ACCEPT, output);
            let _ = output.flush();
            break;
        } else {
            trusted_utils_log_err("Invalid directive!");
            break;
        }

        if !top_check::top_check_valid() && !reported_error {
            reported_error = true;
        }
    }
    let _ = trusted_utils::SIG_SIZE_BYTES;
}
