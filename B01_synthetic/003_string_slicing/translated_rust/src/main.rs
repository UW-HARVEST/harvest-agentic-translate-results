use std::env;
use std::process;

/// Mimics C strtol: parse leading decimal integer from string.
/// Returns (value, rest_of_string). If no digits parsed, rest == input.
fn strtol(s: &str) -> (i64, &str) {
    let s = s.trim_start();
    let mut chars = s.char_indices().peekable();
    let negative = matches!(chars.peek(), Some((_, '-')));
    let positive = matches!(chars.peek(), Some((_, '+')));
    if negative || positive {
        chars.next();
    }
    let start = chars.peek().map(|&(i, _)| i).unwrap_or(s.len());
    let mut end = start;
    for (i, c) in chars {
        if c.is_ascii_digit() {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == start {
        return (0, s); // no digits consumed — rest == original
    }
    let val: i64 = s[..end].parse().unwrap_or(0);
    (val, &s[end..])
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len(); // argc in C includes program name

    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        process::exit(1);
    }

    let argv1 = args[1].as_bytes();
    let len = argv1.len() as u64; // size_t

    let start: i32;
    // `end` tracks whether strtol consumed digits for argv[2].
    // Bug in C: `end` is never updated for argv[3] parse, so the
    // argv[3] integer check is dead code. We reproduce this exactly.
    let end_is_start: bool; // true means strtol consumed no digits from argv[2]

    if argc >= 3 {
        let (val, rest) = strtol(&args[2]);
        end_is_start = std::ptr::eq(rest.as_ptr(), args[2].as_ptr())
            || rest.len() == args[2].len();
        start = val as i32;
        if end_is_start {
            print!("Second argument must be an integer!");
            process::exit(1);
        }
        // C: (int)start > (size_t)len — signed/unsigned comparison.
        // In C, negative int promotes to large unsigned, so negative start passes.
        if (start as u64) > len {
            print!("Error: start is off the end of the string!\n");
            process::exit(1);
        }
    } else {
        start = 0;
        end_is_start = false;
    }

    let stop: i32;
    if argc == 4 {
        // C passes NULL for endptr here, so `end` is NOT updated.
        let (val, _) = strtol(&args[3]);
        stop = val as i32;
        // Bug: checks stale `end` from argv[2] parse against argv[3].
        // This can never be true when argc==4 (end points into argv[2]),
        // so this branch is dead code. We keep it for fidelity.
        if end_is_start {
            // Would compare end (from argv[2] parse) == argv[3]
            print!("Third argument must be an integer!");
            process::exit(1);
        }
        if (stop as u64) > len {
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
    let slice = &argv1[offset..offset + count];
    let s = std::str::from_utf8(slice).unwrap_or_default();
    print!("{}\n", s);
}
