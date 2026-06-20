pub fn math_err(msg: &str) {
    eprintln!("[ERROR] {msg}");
    std::process::abort()
}
pub fn op_err(type_: &str, code: u8) {
    eprintln!("[ERROR] invalid {type_} code: 0x{code:02x}");
    std::process::abort()
}
