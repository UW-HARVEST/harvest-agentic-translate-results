use crate::checker_interface::*;
use crate::top_check;
use crate::trusted_utils;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

pub struct TrustedChecker {
    input: File,
    output: File,
}

struct Buffers {
    buf_lits: Vec<i32>,
    buf_hints: Vec<u64>,
    buf_sig: [u8; 16],
}

fn buffers() -> &'static Mutex<Buffers> {
    use std::sync::OnceLock;
    static B: OnceLock<Mutex<Buffers>> = OnceLock::new();
    B.get_or_init(|| {
        Mutex::new(Buffers {
            buf_lits: Vec::with_capacity(1 << 14),
            buf_hints: Vec::with_capacity(1 << 14),
            buf_sig: [0u8; 16],
        })
    })
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        let start = std::time::Instant::now();
        let mut nb_produced: u64 = 0;
        let mut nb_imported: u64 = 0;
        let mut nb_deleted: u64 = 0;
        let mut reported_error = false;

        // We re-open the named pipes from the global instance.
        let (mut input, mut output) = {
            let g = global_tc().lock().unwrap();
            (
                g.0.try_clone().expect("clone input"),
                g.1.try_clone().expect("clone output"),
            )
        };

        loop {
            let c = trusted_utils::trusted_utils_read_char(&mut input);
            if c == TRUSTED_CHK_CLS_PRODUCE as i32 {
                let id = trusted_utils::trusted_utils_read_ul(&mut input);
                let nb_lits = trusted_utils::trusted_utils_read_int(&mut input);
                read_literals_internal(&mut input, nb_lits);
                let nb_hints = trusted_utils::trusted_utils_read_int(&mut input);
                read_hints_internal(&mut input, nb_hints);
                let share = trusted_utils::trusted_utils_read_bool(&mut input);
                let mut sig_opt: Option<Vec<u8>> = if share {
                    Some(vec![0u8; 16])
                } else {
                    None
                };
                let (lits, hints) = {
                    let b = buffers().lock().unwrap();
                    (b.buf_lits.clone(), b.buf_hints.clone())
                };
                let res = top_check::top_check_produce(
                    id, &lits, nb_lits, &hints, nb_hints, &mut sig_opt,
                );
                say_internal(&mut output, res);
                if let Some(sig) = sig_opt {
                    trusted_utils::trusted_utils_write_sig(&sig, &mut output);
                    let mut b = buffers().lock().unwrap();
                    for i in 0..16 {
                        b.buf_sig[i] = sig[i];
                    }
                }
                nb_produced += 1;
            } else if c == TRUSTED_CHK_CLS_IMPORT as i32 {
                let id = trusted_utils::trusted_utils_read_ul(&mut input);
                let nb_lits = trusted_utils::trusted_utils_read_int(&mut input);
                read_literals_internal(&mut input, nb_lits);
                let mut sig_buf = [0u8; 16];
                trusted_utils::trusted_utils_read_sig(&mut sig_buf, &mut input);
                let lits = {
                    let b = buffers().lock().unwrap();
                    b.buf_lits.clone()
                };
                let res = top_check::top_check_import(id, &lits, nb_lits, &sig_buf);
                say_internal(&mut output, res);
                nb_imported += 1;
            } else if c == TRUSTED_CHK_CLS_DELETE as i32 {
                let nb_hints = trusted_utils::trusted_utils_read_int(&mut input);
                read_hints_internal(&mut input, nb_hints);
                let hints = {
                    let b = buffers().lock().unwrap();
                    b.buf_hints.clone()
                };
                let res = top_check::top_check_delete(&hints, nb_hints);
                say_internal(&mut output, res);
                nb_deleted += nb_hints as u64;
            } else if c == TRUSTED_CHK_LOAD as i32 {
                let nb_lits = trusted_utils::trusted_utils_read_int(&mut input);
                read_literals_internal(&mut input, nb_lits);
                let lits = {
                    let b = buffers().lock().unwrap();
                    b.buf_lits.clone()
                };
                for i in 0..(nb_lits as usize) {
                    top_check::top_check_load(lits[i]);
                }
            } else if c == TRUSTED_CHK_INIT as i32 {
                let nb_vars = trusted_utils::trusted_utils_read_int(&mut input);
                top_check::top_check_init(nb_vars, check_model, lenient);
                let mut formula_sig = [0u8; 16];
                trusted_utils::trusted_utils_read_sig(&mut formula_sig, &mut input);
                top_check::top_check_commit_formula_sig(&formula_sig);
                say_with_flush_internal(&mut output, true);
            } else if c == TRUSTED_CHK_END_LOAD as i32 {
                let r = top_check::top_check_end_load();
                say_with_flush_internal(&mut output, r);
            } else if c == TRUSTED_CHK_VALIDATE_UNSAT as i32 {
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; 16]);
                let res = top_check::top_check_validate_unsat(&mut sig_opt);
                say_internal(&mut output, res);
                if let Some(sig) = sig_opt {
                    trusted_utils::trusted_utils_write_sig(&sig, &mut output);
                }
                let _ = output.flush();
                if res {
                    trusted_utils::trusted_utils_log("UNSAT validated");
                }
            } else if c == TRUSTED_CHK_VALIDATE_SAT as i32 {
                let model_size = trusted_utils::trusted_utils_read_int(&mut input);
                let mut model = vec![0i32; model_size as usize];
                trusted_utils::trusted_utils_read_ints(
                    &mut model,
                    model_size as u64,
                    &mut input,
                );
                let mut sig_opt: Option<Vec<u8>> = Some(vec![0u8; 16]);
                let res = top_check::top_check_validate_sat(
                    &model,
                    model_size as u64,
                    &mut sig_opt,
                );
                say_internal(&mut output, res);
                if let Some(sig) = sig_opt {
                    trusted_utils::trusted_utils_write_sig(&sig, &mut output);
                }
                let _ = output.flush();
                if res {
                    trusted_utils::trusted_utils_log("SAT validated");
                }
            } else if c == TRUSTED_CHK_TERMINATE as i32 {
                say_with_flush_internal(&mut output, true);
                break;
            } else {
                trusted_utils::trusted_utils_log_err("Invalid directive!");
                break;
            }

            if !top_check::top_check_valid() && !reported_error {
                trusted_utils::trusted_utils_log_err("(checker reports error)");
                reported_error = true;
            }
        }

        let elapsed = start.elapsed().as_secs_f32();
        let msg = format!(
            "cpu:{:.3} prod:{} imp:{} del:{}",
            elapsed, nb_produced, nb_imported, nb_deleted
        );
        trusted_utils::trusted_utils_log(&msg);

        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).unwrap_or_else(|_| {
            trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(fifo_out)
            .unwrap_or_else(|_| {
                trusted_utils::trusted_utils_exit_eof();
                unreachable!()
            });
        // Initialize global handles
        let i_clone = input.try_clone().expect("clone input");
        let o_clone = output.try_clone().expect("clone output");
        {
            let mut g = global_tc().lock().unwrap();
            *g = (i_clone, o_clone);
        }
        TrustedChecker { input, output }
    }

    pub fn tc_end(&mut self) {
        // Drop is automatic in Rust; flush to be safe.
        let _ = self.output.flush();
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        read_literals_internal(&mut self.input, nb_lits);
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        read_hints_internal(&mut self.input, nb_hints);
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        say_with_flush_internal(&mut self.output, ok);
    }

    pub fn say(&mut self, ok: bool) {
        say_internal(&mut self.output, ok);
    }
}

fn read_literals_internal(input: &mut File, nb_lits: i32) {
    let n = nb_lits as usize;
    let mut b = buffers().lock().unwrap();
    if b.buf_lits.len() < n {
        b.buf_lits.resize(n, 0);
    }
    let cap = b.buf_lits.capacity();
    if cap < n {
        b.buf_lits.reserve(n - cap);
    }
    let slice = &mut b.buf_lits[..n];
    trusted_utils::trusted_utils_read_ints(slice, n as u64, input);
}

fn read_hints_internal(input: &mut File, nb_hints: i32) {
    let n = nb_hints as usize;
    let mut b = buffers().lock().unwrap();
    if b.buf_hints.len() < n {
        b.buf_hints.resize(n, 0);
    }
    let slice = &mut b.buf_hints[..n];
    trusted_utils::trusted_utils_read_uls(slice, n as u64, input);
}

fn say_internal(output: &mut File, ok: bool) {
    let c = if ok {
        TRUSTED_CHK_RES_ACCEPT
    } else {
        TRUSTED_CHK_RES_ERROR
    };
    trusted_utils::trusted_utils_write_char(c, output);
}

fn say_with_flush_internal(output: &mut File, ok: bool) {
    say_internal(output, ok);
    let _ = output.flush();
}

fn global_tc() -> &'static Mutex<(File, File)> {
    use std::sync::OnceLock;
    static G: OnceLock<Mutex<(File, File)>> = OnceLock::new();
    G.get_or_init(|| {
        // Initial placeholder: stdin/stdout duplicates aren't useful here;
        // we'll overwrite in tc_init. Use /dev/null as safe default.
        let f1 = File::open("/dev/null").expect("open /dev/null");
        let f2 = OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null write");
        Mutex::new((f1, f2))
    })
}
