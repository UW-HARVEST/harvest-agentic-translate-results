mod config;
mod mdcore;

use std::ffi::{CStr, CString, OsStr, c_char, c_int};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

unsafe extern "C" {
    fn atoi(value: *const c_char) -> c_int;
}

fn parse_argument(value: &OsStr) -> c_int {
    let value = CString::new(value.as_bytes()).expect("argv cannot contain a NUL byte");
    // SAFETY: CString provides the same NUL-terminated representation received
    // by the C program, and atoi does not retain its argument.
    unsafe { atoi(value.as_ptr()) }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.len() < 3 {
        let mut stderr = io::stderr().lock();
        stderr.write_all(b"usage: ").unwrap();
        stderr.write_all(arguments[0].as_bytes()).unwrap();
        stderr.write_all(b" A B\n").unwrap();
        return ExitCode::from(2);
    }

    let a = parse_argument(&arguments[1]);
    let b = parse_argument(&arguments[2]);

    let result_call = config::OP.apply(a, b);
    let accumulator = config::run_unrolled(config::OP, config::REPEAT);

    let x1 = mdcore::helper_call(a, b);
    let x2 = mdcore::helper_ptr(a, b);
    let x3 = mdcore::use_generated(config::REPEAT);

    // SAFETY: Both globals are initialized to matching static values and this
    // program does not mutate them.
    let (global_result, operation_name) = unsafe {
        let global_result = (mdcore::G_OP)(a, b);
        let operation_name = CStr::from_ptr(mdcore::G_OP_NAME).to_str().unwrap();
        (global_result, operation_name)
    };

    println!("op={operation_name} call={result_call} acc={accumulator} g.call={global_result}");

    let summary = result_call
        .wrapping_add(accumulator)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(global_result);
    println!("summary={summary}");

    ExitCode::SUCCESS
}
