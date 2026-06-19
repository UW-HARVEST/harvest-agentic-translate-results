use std::env;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[derive(Clone, Copy)]
enum IntRef {
    Initial,
    Inner,
}

fn parse_c_strtol_i32(arg: &[u8]) -> Option<i32> {
    let mut index = 0;
    while matches!(
        arg.get(index),
        Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    ) {
        index += 1;
    }

    let mut negative = false;
    if let Some(sign) = arg.get(index) {
        if *sign == b'-' {
            negative = true;
            index += 1;
        } else if *sign == b'+' {
            index += 1;
        }
    }

    let first_digit = index;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;
    let mut overflowed = false;

    while let Some(byte) = arg.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }

        let digit = (byte - b'0') as u64;
        if value > (limit - digit) / 10 {
            overflowed = true;
            value = limit;
        } else if !overflowed {
            value = value * 10 + digit;
        }
        index += 1;
    }

    if index == first_digit {
        return None;
    }

    let as_long = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative && value == (i64::MAX as u64) + 1 {
        i64::MIN
    } else if negative {
        -(value as i64)
    } else {
        value as i64
    };

    Some(as_long as i32)
}

fn static_alias(which: IntRef, initial: &mut i32, inner: &mut i32) -> IntRef {
    let outer = match which {
        IntRef::Initial => *initial,
        IntRef::Inner => *inner,
    };

    if outer >= *inner {
        *inner = inner.wrapping_add(outer);
        IntRef::Inner
    } else {
        match which {
            IntRef::Initial => *initial = initial.wrapping_add(*inner),
            IntRef::Inner => *inner = inner.wrapping_add(*inner),
        }
        which
    }
}

fn current_value(which: IntRef, initial: i32, inner: i32) -> i32 {
    match which {
        IntRef::Initial => initial,
        IntRef::Inner => inner,
    }
}

#[cfg(unix)]
fn args_as_bytes() -> Vec<Vec<u8>> {
    env::args_os().map(|arg| arg.into_vec()).collect()
}

#[cfg(not(unix))]
fn args_as_bytes() -> Vec<Vec<u8>> {
    env::args_os()
        .map(|arg| arg.to_string_lossy().as_bytes().to_vec())
        .collect()
}

fn main() {
    let args = args_as_bytes();

    if args.len() != 3 {
        print!("Error: should only be two (integer) arguments!\n");
        std::process::exit(1);
    }

    let mut initial_value = match parse_c_strtol_i32(&args[1]) {
        Some(value) => value,
        None => {
            print!("Error: first argument must be an integer!\n");
            std::process::exit(1);
        }
    };

    let iterations = match parse_c_strtol_i32(&args[2]) {
        Some(value) => value,
        None => {
            print!("Error: second argument must be an integer!\n");
            std::process::exit(1);
        }
    };

    let mut inner = 1_i32;
    let mut running_sum = IntRef::Initial;
    let mut i = 0_i32;
    while i < iterations {
        running_sum = static_alias(running_sum, &mut initial_value, &mut inner);
        println!("{}", current_value(running_sum, initial_value, inner));
        i = i.wrapping_add(1);
    }
}
