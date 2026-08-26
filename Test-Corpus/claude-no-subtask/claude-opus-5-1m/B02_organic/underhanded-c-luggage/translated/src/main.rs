// Translated from c_src/src/luggage.c
// Original: By Jan Wrobel <wrr@mixedbit.org>

use std::io::{self, Read, Write};

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Default, Clone)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

fn skip_ws(buf: &[u8], pos: &mut usize) {
    while *pos < buf.len() && buf[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

// Mimics scanf "%d ":
// - Returns None on EOF (input failure before any conversion).
// - Returns Some(value) on success or matching failure (matching failure
//   yields 0 since the C code only checks for EOF and would proceed with
//   uninitialized data on matching failure; we use a defined value here).
fn scan_int(buf: &[u8], pos: &mut usize) -> Option<u32> {
    skip_ws(buf, pos);
    if *pos >= buf.len() {
        return None;
    }
    let start = *pos;
    let mut sign: i64 = 1;
    if buf[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    } else if buf[*pos] == b'+' {
        *pos += 1;
    }
    if *pos >= buf.len() || !buf[*pos].is_ascii_digit() {
        // Matching failure (not EOF as a whole scanf call); restore pos to start.
        *pos = start;
        return Some(0);
    }
    let mut val: i64 = 0;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((buf[*pos] - b'0') as i64);
        *pos += 1;
    }
    Some(val.wrapping_mul(sign) as u32)
}

// Mimics scanf "%N[A-Z0-9]"
fn scan_set_alnum(buf: &[u8], pos: &mut usize, max: usize) -> Vec<u8> {
    let mut result = Vec::new();
    while *pos < buf.len() && result.len() < max {
        let c = buf[*pos];
        let in_set = (b'A'..=b'Z').contains(&c) || (b'0'..=b'9').contains(&c);
        if !in_set {
            break;
        }
        result.push(c);
        *pos += 1;
    }
    result
}

// Mimics scanf "%N[A-Z]"
fn scan_set_alpha(buf: &[u8], pos: &mut usize, max: usize) -> Vec<u8> {
    let mut result = Vec::new();
    while *pos < buf.len() && result.len() < max {
        let c = buf[*pos];
        if !(b'A'..=b'Z').contains(&c) {
            break;
        }
        result.push(c);
        *pos += 1;
    }
    result
}

// Mimics scanf "%N[^\n]"
fn scan_not_newline(buf: &[u8], pos: &mut usize, max: usize) -> Vec<u8> {
    let mut result = Vec::new();
    while *pos < buf.len() && result.len() < max {
        let c = buf[*pos];
        if c == b'\n' {
            break;
        }
        result.push(c);
        *pos += 1;
    }
    result
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    if !expected.is_empty() && expected[0] == b'-' {
        return true;
    }
    expected == actual
}

// Mirrors `supersedes` in the C code: walks from `start` index forward,
// finds the FIRST directive with matching luggage_id. Returns true iff
// that first match also has the same departure airport.
fn supersedes(directives: &[RoutingDirective], start: usize, luggage_id: &[u8], departure: &[u8]) -> bool {
    let mut i = start;
    while i < directives.len() {
        if directives[i].luggage_id.as_slice() != luggage_id {
            i += 1;
            continue;
        }
        return directives[i].departure.as_slice() == departure;
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        let stderr = io::stderr();
        let mut e = stderr.lock();
        let _ = e.write_all(b"Command line error: 4 arguments expected\n");
        std::process::exit(1);
    }

    // Read all of stdin into a buffer to emulate scanf parsing.
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).expect("failed to read stdin");
    let mut pos: usize = 0;

    let mut directives: Vec<RoutingDirective> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        let time_stamp = match scan_int(&buf, &mut pos) {
            None => break,
            Some(v) => v,
        };
        // The trailing ' ' in "%d " consumes any whitespace.
        skip_ws(&buf, &mut pos);

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        if pos >= buf.len() {
            break;
        }
        let luggage_id = scan_set_alnum(&buf, &mut pos, LUGGAGE_ID_LENGTH);
        skip_ws(&buf, &mut pos);
        if pos >= buf.len() && luggage_id.is_empty() {
            break;
        }
        let flight_id = scan_set_alnum(&buf, &mut pos, FLIGHT_ID_LENGTH);
        skip_ws(&buf, &mut pos);

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        if pos >= buf.len() {
            break;
        }
        let departure = scan_set_alpha(&buf, &mut pos, AIRPORT_CODE_LENGTH);
        skip_ws(&buf, &mut pos);
        if pos >= buf.len() && departure.is_empty() {
            break;
        }
        let arrival = scan_set_alpha(&buf, &mut pos, AIRPORT_CODE_LENGTH);

        // scanf("%80[^\n]", comments)
        if pos >= buf.len() {
            break;
        }
        let comments = scan_not_newline(&buf, &mut pos, COMMENTS_LENGTH);

        directives.push(RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        });
    }

    // The C code uses an in-order insertion that sorts by time_stamp ascending,
    // with ties broken by insertion order (newer entries placed AFTER older
    // ones of equal time_stamp). This is equivalent to a stable ascending sort.
    directives.sort_by_key(|d| d.time_stamp);

    let expected_luggage = args[1].as_bytes();
    let expected_flight = args[2].as_bytes();
    let expected_departure = args[3].as_bytes();
    let expected_arrival = args[4].as_bytes();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    for (i, d) in directives.iter().enumerate() {
        if !supersedes(&directives, i + 1, &d.luggage_id, &d.departure)
            && matches(expected_luggage, &d.luggage_id)
            && matches(expected_flight, &d.flight_id)
            && matches(expected_departure, &d.departure)
            && matches(expected_arrival, &d.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
            let _ = write!(out, "{:010} ", d.time_stamp);
            let _ = out.write_all(&d.luggage_id);
            let _ = out.write_all(b" ");
            let _ = out.write_all(&d.flight_id);
            let _ = out.write_all(b" ");
            let _ = out.write_all(&d.departure);
            let _ = out.write_all(b" ");
            let _ = out.write_all(&d.arrival);
            let _ = out.write_all(b" ");
            let _ = out.write_all(&d.comments);
            let _ = out.write_all(b"\n");
        }
    }

    std::process::exit(0);
}
