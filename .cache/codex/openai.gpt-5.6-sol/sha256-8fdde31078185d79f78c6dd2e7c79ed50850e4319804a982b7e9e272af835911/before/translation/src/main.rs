use std::env;
use std::ffi::CString;
use std::io::{self, Write};
use std::os::raw::{c_char, c_long};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

extern "C" {
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: i32) -> c_long;
}

#[derive(Clone, Copy)]
enum RunningSum {
    Initial,
    Inner,
}

fn static_alias(
    running_sum: &mut RunningSum,
    initial_value: &mut i32,
    inner: &mut i32,
) -> i32 {
    let outer = match running_sum {
        RunningSum::Initial => *initial_value,
        RunningSum::Inner => *inner,
    };

    if outer >= *inner {
        *inner = inner.wrapping_add(outer);
        *running_sum = RunningSum::Inner;
        *inner
    } else {
        match running_sum {
            RunningSum::Initial => {
                *initial_value = initial_value.wrapping_add(*inner);
                *initial_value
            }
            RunningSum::Inner => {
                *inner = inner.wrapping_add(*inner);
                *inner
            }
        }
    }
}

fn parse_like_c(argument: &std::ffi::OsStr) -> Option<i32> {
    let argument = CString::new(argument.as_bytes()).expect("arguments cannot contain NUL bytes");
    let start = argument.as_ptr();
    let mut end = start.cast_mut();

    // SAFETY: `argument` is NUL-terminated, and `end` points to writable pointer storage.
    let value = unsafe { strtol(start, &mut end, 10) };
    (end != start.cast_mut()).then_some(value as i32)
}

fn write_stdout(bytes: &[u8]) {
    io::stdout().write_all(bytes).ok();
}

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        write_stdout(b"Error: should only be two (integer) arguments!\n");
        return ExitCode::FAILURE;
    }

    let Some(mut initial_value) = parse_like_c(&arguments[1]) else {
        write_stdout(b"Error: first argument must be an integer!\n");
        return ExitCode::FAILURE;
    };

    let Some(iterations) = parse_like_c(&arguments[2]) else {
        write_stdout(b"Error: second argument must be an integer!\n");
        return ExitCode::FAILURE;
    };

    let mut inner = 1_i32;
    let mut running_sum = RunningSum::Initial;
    let stdout = io::stdout();
    let mut output = stdout.lock();

    for _ in 0..iterations {
        let value = static_alias(&mut running_sum, &mut initial_value, &mut inner);
        writeln!(output, "{value}").ok();
    }

    ExitCode::SUCCESS
}
