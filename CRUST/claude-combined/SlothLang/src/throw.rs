pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    // Equivalent to raise(SIGFPE) - terminate with a math error.
    std::process::exit(136);
}

pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    // Equivalent to raise(SIGILL) - terminate with an illegal-op error.
    std::process::exit(132);
}
