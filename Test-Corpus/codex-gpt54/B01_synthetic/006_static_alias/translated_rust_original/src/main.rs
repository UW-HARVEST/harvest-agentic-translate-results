use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStringExt;
use std::process;
use std::sync::atomic::{AtomicI32, Ordering};

static INNER: AtomicI32 = AtomicI32::new(1);

enum RunningSum {
    Outer(i32),
    Inner,
}

fn static_alias(running_sum: &mut RunningSum) {
    match running_sum {
        RunningSum::Outer(outer) => {
            let inner = INNER.load(Ordering::Relaxed);
            if *outer >= inner {
                INNER.store(inner.wrapping_add(*outer), Ordering::Relaxed);
                *running_sum = RunningSum::Inner;
            } else {
                *outer = outer.wrapping_add(inner);
            }
        }
        RunningSum::Inner => {
            let inner = INNER.load(Ordering::Relaxed);
            if inner >= inner {
                INNER.store(inner.wrapping_add(inner), Ordering::Relaxed);
            }
        }
    }
}

fn parse_int_arg(arg: &[u8]) -> Option<i32> {
    let c_arg = CString::new(arg).ok()?;
    let mut end = std::ptr::null_mut();
    let parsed = unsafe { libc::strtol(c_arg.as_ptr(), &mut end, 10) };
    if end == c_arg.as_ptr() as *mut libc::c_char {
        None
    } else {
        Some(parsed as i32)
    }
}

fn print_running_sum(running_sum: &RunningSum) {
    match running_sum {
        RunningSum::Outer(value) => println!("{value}"),
        RunningSum::Inner => println!("{}", INNER.load(Ordering::Relaxed)),
    }
}

fn main() {
    let args: Vec<_> = env::args_os().collect();

    if args.len() != 3 {
        print!("Error: should only be two (integer) arguments!\n");
        process::exit(1);
    }

    let initial_value = match parse_int_arg(&args[1].clone().into_vec()) {
        Some(value) => value,
        None => {
            print!("Error: first argument must be an integer!\n");
            process::exit(1);
        }
    };

    let iterations = match parse_int_arg(&args[2].clone().into_vec()) {
        Some(value) => value,
        None => {
            print!("Error: second argument must be an integer!\n");
            process::exit(1);
        }
    };

    let mut running_sum = RunningSum::Outer(initial_value);
    for _ in 0..iterations {
        static_alias(&mut running_sum);
        print_running_sum(&running_sum);
    }
}
