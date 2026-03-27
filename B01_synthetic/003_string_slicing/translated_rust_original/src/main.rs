use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        process::exit(1);
    }

    let s = &args[1];
    let len = s.len() as i64;

    let start: i64;
    // Track whether strtol consumed any characters for argv[2].
    // In the C code, `end` is set by strtol(argv[2], &end, 10) and then
    // *also* checked against argv[3] — reproducing that bug exactly.
    let mut end_equals_arg2 = false;

    if argc >= 3 {
        let (val, consumed) = strtol_prefix(&args[2]);
        end_equals_arg2 = !consumed;
        if !consumed {
            print!("Second argument must be an integer!");
            process::exit(1);
        }
        start = val;
        if start > len {
            print!("Error: start is off the end of the string!\n");
            process::exit(1);
        }
    } else {
        start = 0;
    }

    let stop: i64;
    if argc == 4 {
        let (val, _consumed) = strtol_prefix(&args[3]);
        // BUG REPRODUCTION: C code checks `end == argv[3]` but `end` was
        // set by the argv[2] parse, NOT the argv[3] parse. So this check
        // can never be true when argv[2] parsed successfully (end moved past
        // argv[2]'s start, so end != argv[3]'s start). We replicate by
        // checking end_equals_arg2 against argv[3], which is always false
        // when argv[2] parsed successfully.
        if end_equals_arg2 {
            print!("Third argument must be an integer!");
            process::exit(1);
        }
        stop = val;
        if stop > len {
            print!("Error: stop is off the end of the string!\n");
            process::exit(1);
        }
        if stop <= start {
            print!("Error: stop must come after start!\n");
            process::exit(1);
        }
    } else {
        stop = len;
    }

    // Equivalent to printf("%.*s\n", stop - start, argv[1] + start)
    let start_u = start as usize;
    let count = (stop - start) as usize;
    let bytes = s.as_bytes();
    let slice = &bytes[start_u..start_u + count];
    // Write raw bytes to match C behavior exactly
    use std::io::Write;
    std::io::stdout().write_all(slice).unwrap();
    print!("\n");
}

/// Mimics C strtol: parse a leading decimal integer from `s`.
/// Returns (value, consumed) where consumed is true if at least one digit was parsed.
fn strtol_prefix(s: &str) -> (i64, bool) {
    let s = s.as_bytes();
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
