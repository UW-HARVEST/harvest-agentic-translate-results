pub fn math_err(msg: &str) {
    eprintln!("[ERROR] {}", msg);
    panic!("math error: {}", msg);
}
pub fn op_err(type_: &str, code: u8) {
    eprintln!("[ERROR] invalid {} code: 0x{:02x}", type_, code);
    panic!("op error: invalid {} code: 0x{:02x}", type_, code);
}
