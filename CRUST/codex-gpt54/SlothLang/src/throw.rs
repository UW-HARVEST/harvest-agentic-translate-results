pub fn math_err(msg: &str) {
if !msg.is_empty() {
eprintln!("[ERROR] {msg}");
}
std::process::exit(136);
}
pub fn op_err(type_: &str, code: u8) {
if !type_.is_empty() {
eprintln!("[ERROR] invalid {type_} code: 0x{code:02x}");
}
std::process::exit(132);
}
