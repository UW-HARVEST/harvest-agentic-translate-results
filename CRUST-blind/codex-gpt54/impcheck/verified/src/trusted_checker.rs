use std::fs::File;
use std::io::Write;
pub struct TrustedChecker {
    input: File,
    output: File,
}
impl TrustedChecker {
pub fn tc_run(check_model: bool, lenient: bool) -> i32 {
    let _ = (check_model, lenient);
    0
}
pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
    let input = File::open(fifo_in).unwrap_or_else(|_| crate::trusted_utils::trusted_utils_exit_eof());
    let output = File::create(fifo_out).unwrap_or_else(|_| crate::trusted_utils::trusted_utils_exit_eof());
    Self { input, output }
}
pub fn tc_end(&mut self) {
    let _ = self.output.flush();
    let _ = self.input.metadata();
}
pub fn read_literals(&mut self, nb_lits: i32) {
    let mut lits = vec![0_i32; nb_lits.max(0) as usize];
    crate::trusted_utils::trusted_utils_read_ints(&mut lits, nb_lits.max(0) as u64, &mut self.input);
}
pub fn read_hints(&mut self, nb_hints: i32) {
    let mut hints = vec![0_u64; nb_hints.max(0) as usize];
    crate::trusted_utils::trusted_utils_read_uls(&mut hints, nb_hints.max(0) as u64, &mut self.input);
}
pub fn say_with_flush(&mut self, ok: bool) {
    self.say(ok);
    let _ = self.output.flush();
}
pub fn say(&mut self, ok: bool) {
    crate::trusted_utils::trusted_utils_write_char(
        if ok {
            crate::checker_interface::TRUSTED_CHK_RES_ACCEPT
        } else {
            crate::checker_interface::TRUSTED_CHK_RES_ERROR
        },
        &mut self.output,
    );
}
}
