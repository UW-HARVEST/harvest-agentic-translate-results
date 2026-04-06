use std::os::raw::c_char;

fn print_line(s: &[u8]) {
    driver::printLine(s.as_ptr() as *const c_char);
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    print_line(b"Calling good()...\0");
    driver::good_g2b();
    driver::good_b2g(&mut reader);
    print_line(b"Finished good()\0");
    print_line(b"Calling bad()...\0");
    driver::bad_impl(&mut reader);
    print_line(b"Finished bad()\0");
}
