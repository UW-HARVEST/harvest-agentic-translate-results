#![no_main]
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Mimics C strtol: parse a leading decimal integer from `s`.
fn strtol_prefix(s: &[u8]) -> (i64, bool) {
    let mut i = 0;
    let neg = if i < s.len() && s[i] == b'-' {
        i += 1;
        true
    } else if i < s.len() && s[i] == b'+' {
        i += 1;
        false
    } else {
        false
    };
    let start = i;
    let mut val: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return (0, false);
    }
    if neg {
        val = val.wrapping_neg();
    }
    (val, true)
}

#[no_mangle]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        return 1;
    }

    let args: Vec<&CStr> = (0..argc)
        .map(|i| unsafe { CStr::from_ptr(*argv.add(i as usize)) })
        .collect();

    let s = args[1].to_bytes();
    let len = s.len() as i64;

    let start: i64;
    let mut end_equals_arg2 = false;

    if argc >= 3 {
        let arg2 = args[2].to_bytes();
        let (val, consumed) = strtol_prefix(arg2);
        end_equals_arg2 = !consumed;
        if !consumed {
            print!("Second argument must be an integer!");
            return 1;
        }
        start = val;
        if (start as u64) > (len as u64) {
            print!("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    let stop: i64;
    if argc == 4 {
        let arg3 = args[3].to_bytes();
        let (val, _consumed) = strtol_prefix(arg3);
        if end_equals_arg2 {
            print!("Third argument must be an integer!");
            return 1;
        }
        stop = val;
        if (stop as u64) > (len as u64) {
            print!("Error: stop is off the end of the string!\n");
            return 1;
        }
        if stop <= start {
            print!("Error: stop must come after start!\n");
            return 1;
        }
    } else {
        stop = len;
    }

    let start_u = start as usize;
    let count = (stop - start) as usize;
    let slice = &s[start_u..start_u + count];
    use std::io::Write;
    std::io::stdout().write_all(slice).unwrap();
    print!("\n");

    0
}
