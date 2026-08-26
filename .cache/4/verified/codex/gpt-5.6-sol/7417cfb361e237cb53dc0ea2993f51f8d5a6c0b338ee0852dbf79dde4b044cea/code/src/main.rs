use std::env;
use std::ffi::{c_char, c_int, c_long, CString, OsStr};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::ptr;

extern "C" {
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

#[derive(Clone, Copy)]
enum RunningSum {
    Initial,
    Inner,
}

fn parse_c_int(argument: &OsStr) -> Option<i32> {
    let argument = CString::new(argument.as_bytes()).expect("arguments cannot contain NUL bytes");
    let start = argument.as_ptr();
    let mut end = ptr::null_mut();
    let value = unsafe { strtol(start, &mut end, 10) };

    (end != start.cast_mut()).then_some(value as i32)
}

fn static_alias(running_sum: RunningSum, initial_value: &mut i32, inner: &mut i32) -> RunningSum {
    match running_sum {
        RunningSum::Inner => {
            *inner = inner.wrapping_add(*inner);
            RunningSum::Inner
        }
        RunningSum::Initial if *initial_value >= *inner => {
            *inner = inner.wrapping_add(*initial_value);
            RunningSum::Inner
        }
        RunningSum::Initial => {
            *initial_value = initial_value.wrapping_add(*inner);
            RunningSum::Initial
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().collect();
    let stdout = io::stdout();
    let mut output = stdout.lock();

    if arguments.len() != 3 {
        let _ = output.write_all(b"Error: should only be two (integer) arguments!\n");
        return ExitCode::from(1);
    }

    let Some(mut initial_value) = parse_c_int(&arguments[1]) else {
        let _ = output.write_all(b"Error: first argument must be an integer!\n");
        return ExitCode::from(1);
    };

    let Some(iterations) = parse_c_int(&arguments[2]) else {
        let _ = output.write_all(b"Error: second argument must be an integer!\n");
        return ExitCode::from(1);
    };

    let mut inner = 1_i32;
    let mut running_sum = RunningSum::Initial;
    for _ in 0..iterations {
        running_sum = static_alias(running_sum, &mut initial_value, &mut inner);
        let value = match running_sum {
            RunningSum::Initial => initial_value,
            RunningSum::Inner => inner,
        };
        let _ = writeln!(output, "{value}");
    }

    ExitCode::SUCCESS
}
