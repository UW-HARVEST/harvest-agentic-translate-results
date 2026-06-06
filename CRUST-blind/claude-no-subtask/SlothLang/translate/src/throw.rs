pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    // Mimic raise(SIGFPE) by terminating the process with the same exit code
    // that a SIGFPE-killed program would produce on Unix (128 + 8 = 136).
    std::process::exit(136);
}

pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    // Mimic raise(SIGILL) (128 + 4 = 132).
    std::process::exit(132);
}
