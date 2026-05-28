// Rust translation of luggage.c
// Preserves behavior of original C program byte-identically.

use std::io::{self, Read, Write, BufWriter};

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Default)]
struct Directive {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

enum ScanRes<T> {
    Eof,
    NoMatch,
    Ok(T),
}

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skip whitespace; returns true if non-whitespace remains.
    fn skip_ws(&mut self) -> bool {
        while !self.at_eof() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        !self.at_eof()
    }

    /// Equivalent to scanf("%d") - skips leading whitespace, parses int.
    fn scan_int(&mut self) -> ScanRes<i64> {
        if !self.skip_ws() {
            return ScanRes::Eof;
        }
        let start = self.pos;
        let mut neg = false;
        if self.data[self.pos] == b'-' {
            neg = true;
            self.pos += 1;
        } else if self.data[self.pos] == b'+' {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while !self.at_eof() && self.data[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            // no digits matched; rewind sign and report no match
            self.pos = start;
            return ScanRes::NoMatch;
        }
        let s = std::str::from_utf8(&self.data[digits_start..self.pos]).unwrap_or("0");
        let mut val: i64 = s.parse().unwrap_or(0);
        if neg {
            val = val.wrapping_neg();
        }
        ScanRes::Ok(val)
    }

    /// Equivalent to scanf("%N[set]") - does NOT skip leading whitespace.
    fn scan_set<F: Fn(u8) -> bool>(&mut self, max_len: usize, allowed: F) -> ScanRes<String> {
        if self.at_eof() {
            return ScanRes::Eof;
        }
        let start = self.pos;
        let mut count = 0usize;
        while !self.at_eof() && count < max_len && allowed(self.data[self.pos]) {
            self.pos += 1;
            count += 1;
        }
        if self.pos == start {
            return ScanRes::NoMatch;
        }
        let s = String::from_utf8_lossy(&self.data[start..self.pos]).into_owned();
        ScanRes::Ok(s)
    }
}

fn is_alpha_upper_or_digit(b: u8) -> bool {
    (b >= b'A' && b <= b'Z') || (b >= b'0' && b <= b'9')
}

fn is_alpha_upper(b: u8) -> bool {
    b >= b'A' && b <= b'Z'
}

fn is_not_newline(b: u8) -> bool {
    b != b'\n'
}

fn matches_field(expected: &str, actual: &str) -> bool {
    // Mirrors C: expected[0] == '-' || strcmp(expected, actual) == 0
    if let Some(&first) = expected.as_bytes().first() {
        if first == b'-' {
            return true;
        }
    }
    expected == actual
}

/// Mirrors C's `supersedes` recursion: find the FIRST later directive with
/// matching luggage_id; if its departure also matches, return true; otherwise
/// (no match found, or departure mismatch on the first luggage_id match),
/// return false.
fn is_superseded(directives: &[Directive], idx: usize) -> bool {
    let d = &directives[idx];
    for i in (idx + 1)..directives.len() {
        if directives[i].luggage_id == d.luggage_id {
            return directives[i].departure == d.departure;
        }
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        std::process::exit(1);
    }

    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        std::process::exit(1);
    }

    let mut scanner = Scanner::new(input);
    let mut directives: Vec<Directive> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        let time_stamp_i = match scanner.scan_int() {
            ScanRes::Eof => break,
            ScanRes::NoMatch => 0_i64,
            ScanRes::Ok(v) => v,
        };
        // trailing space in format - skip whitespace
        scanner.skip_ws();

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        let luggage_id = match scanner.scan_set(LUGGAGE_ID_LENGTH, is_alpha_upper_or_digit) {
            ScanRes::Eof => break,
            ScanRes::NoMatch => String::new(),
            ScanRes::Ok(s) => s,
        };
        scanner.skip_ws();
        let flight_id = match scanner.scan_set(FLIGHT_ID_LENGTH, is_alpha_upper_or_digit) {
            ScanRes::Eof => break,
            ScanRes::NoMatch => String::new(),
            ScanRes::Ok(s) => s,
        };
        scanner.skip_ws();

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        let departure = match scanner.scan_set(AIRPORT_CODE_LENGTH, is_alpha_upper) {
            ScanRes::Eof => break,
            ScanRes::NoMatch => String::new(),
            ScanRes::Ok(s) => s,
        };
        scanner.skip_ws();
        let arrival = match scanner.scan_set(AIRPORT_CODE_LENGTH, is_alpha_upper) {
            ScanRes::Eof => break,
            ScanRes::NoMatch => String::new(),
            ScanRes::Ok(s) => s,
        };

        // scanf("%80[^\n]", comments) - does NOT skip leading whitespace
        // C initializes comments[0] = 0, so empty if no match.
        let comments = match scanner.scan_set(COMMENTS_LENGTH, is_not_newline) {
            ScanRes::Eof => break,
            ScanRes::NoMatch => String::new(),
            ScanRes::Ok(s) => s,
        };

        directives.push(Directive {
            // C reads %d into unsigned int -> bit pattern preserved.
            time_stamp: time_stamp_i as u32,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        });
    }

    // C insertion logic: while next is non-NULL and next.time_stamp <= new.time_stamp,
    // recurse. Insert when next is NULL or next.time_stamp > new.time_stamp.
    // This means equal-timestamp entries keep insertion order (stable).
    directives.sort_by(|a, b| a.time_stamp.cmp(&b.time_stamp));

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for i in 0..directives.len() {
        if is_superseded(&directives, i) {
            continue;
        }
        let d = &directives[i];
        if !matches_field(&args[1], &d.luggage_id) {
            continue;
        }
        if !matches_field(&args[2], &d.flight_id) {
            continue;
        }
        if !matches_field(&args[3], &d.departure) {
            continue;
        }
        if !matches_field(&args[4], &d.arrival) {
            continue;
        }
        // printf("%010u %s %s %s %s %s\n", ...)
        writeln!(
            out,
            "{:010} {} {} {} {} {}",
            d.time_stamp, d.luggage_id, d.flight_id, d.departure, d.arrival, d.comments
        )
        .unwrap();
    }

    // Flush before exit
    drop(out);
    std::process::exit(0);
}
