pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    // Mimic raise(SIGFPE) by panicking; original C aborts execution.
    panic!("math error: {}", msg);
}

pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    // Mimic raise(SIGILL) by panicking.
    panic!("invalid {} code: 0x{:02x}", type_, code);
}
