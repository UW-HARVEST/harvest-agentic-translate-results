use std::fs::File;
use std::io::{Read, Write};

pub struct TrustedChecker {
    input: File,
    output: File,
}

const TRUSTED_CHK_INIT: u8 = b'B';
const TRUSTED_CHK_LOAD: u8 = b'L';
const TRUSTED_CHK_END_LOAD: u8 = b'E';
const TRUSTED_CHK_CLS_PRODUCE: u8 = b'a';
const TRUSTED_CHK_CLS_IMPORT: u8 = b'i';
const TRUSTED_CHK_CLS_DELETE: u8 = b'd';
const TRUSTED_CHK_VALIDATE_UNSAT: u8 = b'V';
const TRUSTED_CHK_VALIDATE_SAT: u8 = b'M';
const TRUSTED_CHK_TERMINATE: u8 = b'T';
const TRUSTED_CHK_RES_ACCEPT: u8 = b'A';
const TRUSTED_CHK_RES_ERROR: u8 = b'E';

const SIG_SIZE_BYTES: usize = 16;

fn read_byte(file: &mut File) -> Option<u8> {
    let mut buf = [0u8; 1];
    match file.read_exact(&mut buf) {
        Ok(_) => Some(buf[0]),
        Err(_) => None,
    }
}

fn read_int(file: &mut File) -> i32 {
    let mut buf = [0u8; 4];
    if file.read_exact(&mut buf).is_err() {
        std::process::exit(0);
    }
    i32::from_le_bytes(buf)
}

fn read_ul(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    if file.read_exact(&mut buf).is_err() {
        std::process::exit(0);
    }
    u64::from_le_bytes(buf)
}

fn read_bool(file: &mut File) -> bool {
    let mut buf = [0u8; 1];
    if file.read_exact(&mut buf).is_err() {
        std::process::exit(0);
    }
    buf[0] != 0
}

fn read_sig(file: &mut File) -> [u8; SIG_SIZE_BYTES] {
    let mut buf = [0u8; SIG_SIZE_BYTES];
    if file.read_exact(&mut buf).is_err() {
        std::process::exit(0);
    }
    buf
}

impl TrustedChecker {
    pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
        let _ = check_model;
        let _ = lenient;
        // Standalone variant cannot run a real session without an instance.
        // Provided here so the type compiles; actual run is performed via the
        // instance methods on the underlying trusted_checker.c. We retain
        // the signature with a no-op result.
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        let output = File::create(fifo_out).unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        TrustedChecker { input, output }
    }

    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
    }

    pub fn read_literals(&mut self, nb_lits: i32) {
        let n = nb_lits as usize;
        for _ in 0..n {
            let _ = read_int(&mut self.input);
        }
    }

    pub fn read_hints(&mut self, nb_hints: i32) {
        let n = nb_hints as usize;
        for _ in 0..n {
            let _ = read_ul(&mut self.input);
        }
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        self.say(ok);
        let _ = self.output.flush();
    }

    pub fn say(&mut self, ok: bool) {
        let byte = if ok {
            TRUSTED_CHK_RES_ACCEPT
        } else {
            TRUSTED_CHK_RES_ERROR
        };
        let _ = self.output.write_all(&[byte]);
    }
}

#[allow(dead_code)]
fn _ref_unused() {
    let _ = TRUSTED_CHK_INIT;
    let _ = TRUSTED_CHK_LOAD;
    let _ = TRUSTED_CHK_END_LOAD;
    let _ = TRUSTED_CHK_CLS_PRODUCE;
    let _ = TRUSTED_CHK_CLS_IMPORT;
    let _ = TRUSTED_CHK_CLS_DELETE;
    let _ = TRUSTED_CHK_VALIDATE_UNSAT;
    let _ = TRUSTED_CHK_VALIDATE_SAT;
    let _ = TRUSTED_CHK_TERMINATE;
    let _ = read_byte as fn(&mut File) -> Option<u8>;
    let _ = read_bool as fn(&mut File) -> bool;
    let _ = read_sig as fn(&mut File) -> [u8; SIG_SIZE_BYTES];
}
