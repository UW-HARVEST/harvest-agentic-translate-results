pub fn math_err(msg: &str) {
    eprintln!("[ERROR] {}", msg);
    std::process::exit(136);
}

pub fn op_err(type_: &str, code: u8) {
    eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    std::process::exit(132);
}
