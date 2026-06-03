pub fn math_err(msg: &str) {
    if !msg.is_empty() {
        eprintln!("[ERROR] {}", msg);
    }
    std::process::exit(1);
}
pub fn op_err(type_: &str, code: u8) {
    if !type_.is_empty() {
        eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    }
    std::process::exit(1);
}
