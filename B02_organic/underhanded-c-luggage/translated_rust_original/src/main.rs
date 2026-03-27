use std::io::Read;
use std::process;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

/// Skip whitespace (spaces, tabs, newlines) in input. Returns remaining slice.
fn skip_whitespace(input: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < input.len() && (input[i] == b' ' || input[i] == b'\t' || input[i] == b'\n' || input[i] == b'\r') {
        i += 1;
    }
    &input[i..]
}

/// Parse a decimal integer like scanf("%d "). Returns (value, remaining) or None on EOF/no digits.
/// The trailing space in scanf means "skip whitespace after the number".
fn scan_int(input: &[u8]) -> Option<(u32, &[u8])> {
    let s = skip_whitespace(input);
    if s.is_empty() {
        return None;
    }
    // scanf %d reads optional sign then digits. C code stores in unsigned int.
    // We need to handle negative numbers the same way C does: scanf reads a signed int,
    // then it's reinterpreted as unsigned via the %d -> unsigned int store.
    let (negative, s) = if !s.is_empty() && s[0] == b'-' {
        (true, &s[1..])
    } else if !s.is_empty() && s[0] == b'+' {
        (false, &s[1..])
    } else {
        (false, s)
    };
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None; // no digits found, scanf returns EOF-like behavior
    }
    let digits: i64 = std::str::from_utf8(&s[..i]).unwrap().parse().unwrap_or(0);
    let val = if negative { -digits } else { digits } as i32 as u32;
    // skip trailing whitespace (the space after %d in the format string)
    let rest = skip_whitespace(&s[i..]);
    Some((val, rest))
}

/// Scan a scanset like %8[A-Z0-9] or %3[A-Z] or %80[^\n].
/// Returns (matched_string, remaining) or None if no chars matched (EOF-like).
fn scan_set(input: &[u8], max_len: usize, char_fn: fn(u8) -> bool) -> Option<(String, &[u8])> {
    let mut i = 0;
    while i < input.len() && i < max_len && char_fn(input[i]) {
        i += 1;
    }
    if i == 0 {
        // scanf returns EOF when scanset matches nothing and input is at EOF,
        // but if there are chars that just don't match, it returns 0 (failure).
        // The C code checks == EOF for all, and breaks on any failure.
        // If input is empty, treat as EOF. Otherwise it's a match failure
        // but the C code would still break out of the loop.
        return None;
    }
    let s = String::from_utf8_lossy(&input[..i]).into_owned();
    Some((s, &input[i..]))
}

fn is_alnum_upper(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

fn is_upper(b: u8) -> bool {
    b.is_ascii_uppercase()
}

fn is_not_newline(b: u8) -> bool {
    b != b'\n'
}

fn matches(expected: &str, actual: &str) -> bool {
    expected.starts_with('-') || expected == actual
}

fn supersedes(list: &[RoutingDirective], start: usize, luggage_id: &str, departure: &str) -> bool {
    for i in start..list.len() {
        if list[i].luggage_id != luggage_id {
            continue;
        }
        if list[i].departure == departure {
            return true;
        }
        return false;
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprint!("Command line error: 4 arguments expected\n");
        process::exit(1);
    }

    let mut input_buf = Vec::new();
    std::io::stdin().read_to_end(&mut input_buf).unwrap_or(0);
    let mut input: &[u8] = &input_buf;

    // Sorted linked list stored as a Vec; we insert in sorted order by time_stamp.
    let mut directives: Vec<RoutingDirective> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        let time_stamp;
        match scan_int(input) {
            Some((v, rest)) => { time_stamp = v; input = rest; }
            None => break,
        }

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        let luggage_id;
        match scan_set(input, 8, is_alnum_upper) {
            Some((s, rest)) => { luggage_id = s; input = skip_whitespace(rest); }
            None => break,
        }
        let flight_id;
        match scan_set(input, 6, is_alnum_upper) {
            Some((s, rest)) => { flight_id = s; input = skip_whitespace(rest); }
            None => break,
        }

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        let departure;
        match scan_set(input, 3, is_upper) {
            Some((s, rest)) => { departure = s; input = skip_whitespace(rest); }
            None => break,
        }
        let arrival;
        match scan_set(input, 3, is_upper) {
            Some((s, rest)) => { arrival = s; input = rest; }
            None => break,
        }

        // scanf("%80[^\n]", comments) — no trailing whitespace skip
        let comments;
        // The C code does comments[0] = 0 first, so comments are optional.
        // If scanf returns EOF here, the C code breaks. But comments might just
        // be empty (next char is \n). In that case scanf %80[^\n] matches nothing
        // and returns 0 (not EOF), but the C code checks == EOF.
        // If input is exhausted, we break. Otherwise comments stay empty.
        if input.is_empty() {
            break;
        }
        match scan_set(input, 80, is_not_newline) {
            Some((s, rest)) => { comments = s; input = rest; }
            None => { comments = String::new(); }
        }

        let d = RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        };

        // Insert in sorted order by time_stamp (stable: after equal elements)
        let pos = directives.partition_point(|x| x.time_stamp <= d.time_stamp);
        directives.insert(pos, d);
    }

    // Print matching, non-superseded directives
    for i in 0..directives.len() {
        let d = &directives[i];
        let is_superseded = supersedes(&directives, i + 1, &d.luggage_id, &d.departure);
        if !is_superseded
            && matches(&args[1], &d.luggage_id)
            && matches(&args[2], &d.flight_id)
            && matches(&args[3], &d.departure)
            && matches(&args[4], &d.arrival)
        {
            println!(
                "{:010} {} {} {} {} {}",
                d.time_stamp, d.luggage_id, d.flight_id, d.departure, d.arrival, d.comments
            );
        }
    }

    process::exit(0);
}
