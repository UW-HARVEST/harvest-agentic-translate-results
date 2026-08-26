use std::env;
use std::process;

/// Replicates C strtol behavior: parse leading decimal integer from string,
/// return (value, number_of_bytes_consumed). If no digits found, consumed == 0.
fn strtol(s: &str) -> (i64, usize) {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace (strtol does this)
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
        false
    } else {
        false
    };
    let digit_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == digit_start {
        // no digits consumed — end == input pointer
        return (0, 0);
    }
    if neg {
        val = val.wrapping_neg();
    }
    (val, i)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        process::exit(1);
    }

    let s = &args[1];
    let len = s.len();

    let start: i32;
    let stop: i32;

    // Track whether strtol for argv[2] consumed any digits (end == argv[2] check)
    let mut prev_end_equals_input = false;

    if argc >= 3 {
        let (val, consumed) = strtol(&args[2]);
        start = val as i32;
        if consumed == 0 {
            prev_end_equals_input = true;
        }
        if prev_end_equals_input {
            print!("Second argument must be an integer!");
            process::exit(1);
        }
        // C: start > len is signed vs unsigned comparison.
        // Negative start cast to size_t wraps huge, so > len is true.
        if (start as u64) > (len as u64) {
            print!("Error: start is off the end of the string!\n");
            process::exit(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        // C passes NULL for endptr, so end is NOT updated.
        // The subsequent check `end == argv[3]` uses the OLD end from argv[2] parse.
        // This is a bug in the C code — we replicate it: prev_end_equals_input
        // is from the argv[2] parse and is essentially always false here
        // (it would have already exited above if true).
        let (val, _consumed) = strtol(&args[3]);
        stop = val as i32;
        // Bug replication: this check uses the old `end` from argv[2] parse.
        // Since we already exited if prev_end_equals_input was true, this is always false.
        if prev_end_equals_input {
            print!("Third argument must be an integer!");
            process::exit(1);
        }
        if (stop as u64) > (len as u64) {
            print!("Error: stop is off the end of the string!\n");
            process::exit(1);
        }
        if stop <= start {
            print!("Error: stop must come after start!\n");
            process::exit(1);
        }
    } else {
        stop = len as i32;
    }

    // C: printf("%.*s\n", stop - start, argv[1] + start)
    let width = (stop - start) as usize;
    let begin = start as usize;
    print!("{}\n", &s[begin..begin + width]);
}
