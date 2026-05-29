pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    // Mimic raise(SIGFPE) in C by aborting the program.
    std::process::exit(136); // 128 + SIGFPE(8)
}
pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    // Mimic raise(SIGILL) in C.
    std::process::exit(132); // 128 + SIGILL(4)
}
