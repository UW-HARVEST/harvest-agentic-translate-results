pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    // C code raises SIGFPE; we exit with the conventional code for SIGFPE (128 + 8).
    std::process::exit(136);
}
pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    // C code raises SIGILL; we exit with the conventional code for SIGILL (128 + 4).
    std::process::exit(132);
}
