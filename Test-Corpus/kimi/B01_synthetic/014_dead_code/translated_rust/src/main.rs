use std::ffi::CStr;
use std::os::raw::c_char;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() {
    print_line(Some("helperBad()"));
}

fn bad() {
    print_line(Some("bad()"));
}

fn helper_good() {
    print_line(Some("helperGood()"));
}

fn good() {
    print_line(Some("good()"));
    helper_good();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}
