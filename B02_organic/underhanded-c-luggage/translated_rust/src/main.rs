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

/// Skip whitespace (spaces, tabs, newlines) in input, matching scanf's
/// trailing-space behavior. Returns remaining slice.
fn skip_whitespace(input: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < input.len() && input[i].is_ascii_whitespace() {
        i += 1;
    }
    &input[i..]
}

/// Read characters matching a predicate, up to `max` chars.
/// Returns (matched_string, remaining) or None if zero chars matched.
fn scan_chars(input: &[u8], max: usize, pred: fn(u8) -> bool) -> Option<(String, &[u8])> {
    let mut i = 0;
    while i < input.len() && i < max && pred(input[i]) {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let s = String::from_utf8_lossy(&input[..i]).into_owned();
    Some((s, &input[i..]))
}

/// Read an unsigned decimal integer from input.
/// Returns (value, remaining) or None on EOF/no digits.
fn scan_uint(input: &[u8]) -> Option<(u32, &[u8])> {
    let mut i = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let s = std::str::from_utf8(&input[..i]).unwrap();
    let val: u32 = s.parse().unwrap();
    Some((val, &input[i..]))
}

/// Read up to `max` chars until newline (scanf %[^\n] behavior).
fn scan_until_newline(input: &[u8], max: usize) -> (String, &[u8]) {
    let mut i = 0;
    while i < input.len() && i < max && input[i] != b'\n' {
        i += 1;
    }
    let s = String::from_utf8_lossy(&input[..i]).into_owned();
    (s, &input[i..])
}

fn is_upper_or_digit(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

fn is_upper(b: u8) -> bool {
    b.is_ascii_uppercase()
}

fn matches(expected: &str, actual: &str) -> bool {
    expected.starts_with('-') || expected == actual
}

fn supersedes(directives: &[RoutingDirective], from: usize, luggage_id: &str, departure: &str) -> bool {
    for d in &directives[from..] {
        if d.luggage_id != luggage_id {
            continue;
        }
        if d.departure == departure {
            return true;
        }
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
    std::io::stdin().read_to_end(&mut input_buf).unwrap();
    let mut input: &[u8] = &input_buf;

    let mut directives: Vec<RoutingDirective> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        input = skip_whitespace(input);
        let (time_stamp, rest) = match scan_uint(input) {
            Some(v) => v,
            None => break,
        };
        input = skip_whitespace(rest);

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        let (luggage_id, rest) = match scan_chars(input, 8, is_upper_or_digit) {
            Some(v) => v,
            None => break,
        };
        input = skip_whitespace(rest);

        let (flight_id, rest) = match scan_chars(input, 6, is_upper_or_digit) {
            Some(v) => v,
            None => break,
        };
        input = skip_whitespace(rest);

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        let (departure, rest) = match scan_chars(input, 3, is_upper) {
            Some(v) => v,
            None => break,
        };
        input = skip_whitespace(rest);

        let (arrival, rest) = match scan_chars(input, 3, is_upper) {
            Some(v) => v,
            None => break,
        };
        input = rest;

        // scanf("%80[^\n]", comments) — optional
        let (comments, rest) = scan_until_newline(input, 80);
        input = rest;

        // Insert sorted by time_stamp
        let pos = directives
            .iter()
            .position(|d| d.time_stamp > time_stamp)
            .unwrap_or(directives.len());
        directives.insert(pos, RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        });
    }

    // Print matching, non-superseded directives
    let expected_luggage_id = &args[1];
    let expected_flight_id = &args[2];
    let expected_departure = &args[3];
    let expected_arrival = &args[4];

    for i in 0..directives.len() {
        let d = &directives[i];
        // superseded = later directive in list with same luggage_id and departure
        if supersedes(&directives, i + 1, &d.luggage_id, &d.departure) {
            continue;
        }
        if matches(expected_luggage_id, &d.luggage_id)
            && matches(expected_flight_id, &d.flight_id)
            && matches(expected_departure, &d.departure)
            && matches(expected_arrival, &d.arrival)
        {
            println!(
                "{:010} {} {} {} {} {}",
                d.time_stamp, d.luggage_id, d.flight_id, d.departure, d.arrival, d.comments
            );
        }
    }

    process::exit(0);
}
