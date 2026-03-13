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
    let len = s.len();

    // In C, `end` is set by strtol for argv[2] and then REUSED (stale) in the argv[3] check.
    // We reproduce this bug: end_ptr tracks whether argv[2] parse consumed any chars.
    let mut end_is_argv2_start = false;

    let start: i32;
    if argc >= 3 {
        let arg2 = &args[2];
        match parse_strtol(arg2) {
            Some(v) => start = v,
            None => {
                end_is_argv2_start = true;
                start = 0; // value doesn't matter, we'll exit
                print!("Second argument must be an integer!");
                process::exit(1);
            }
        }
        // C: if (start > len) — signed/unsigned comparison.
        // Negative start becomes huge unsigned, so > len is true.
        if (start as u64) > (len as u64) {
            print!("Error: start is off the end of the string!\n");
            process::exit(1);
        }
    } else {
        start = 0;
    }

    let stop: i32;
    if argc == 4 {
        let arg3 = &args[3];
        let parsed = parse_strtol(arg3);
        stop = parsed.unwrap_or(0);

        // C bug: checks stale `end == argv[3]` — end was set during argv[2] parse.
        // This condition is true only if argv[2] parse failed (end still points to argv[2] start),
        // but we already exited in that case. So this branch is effectively dead code.
        // We reproduce it: end_is_argv2_start would be true only if we didn't exit above.
        if end_is_argv2_start {
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

    // printf("%.*s\n", stop - start, argv[1] + start)
    let count = (stop - start) as usize;
    let offset = start as usize;
    let slice = &s.as_bytes()[offset..offset + count];
    let out = std::str::from_utf8(slice).unwrap_or("");
    print!("{}\n", out);
}

/// Mimics C strtol: returns Some(value) if at least one digit was consumed, None otherwise.
fn parse_strtol(s: &str) -> Option<i32> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let mut chars = s.chars().peekable();
    let negative = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut found = false;
    let mut val: i64 = 0;
    while let Some(&c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            found = true;
            val = val * 10 + d as i64;
            chars.next();
        } else {
            break;
        }
    }
    if !found {
        return None;
    }
    if negative {
        val = -val;
    }
    Some(val as i32)
}
