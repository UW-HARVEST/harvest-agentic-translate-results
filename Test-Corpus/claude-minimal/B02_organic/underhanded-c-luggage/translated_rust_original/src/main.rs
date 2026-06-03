// Translated from c_src/src/luggage.c
// Original by Jan Wrobel <wrr@mixedbit.org>

use std::env;
use std::io::{self, BufWriter, Read, Write};
use std::process::exit;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
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

    fn peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if (b as char).is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Mimics scanf %d: skips leading whitespace, then parses a signed integer.
    /// Returns None on EOF or matching failure (so the main loop terminates).
    fn scan_int(&mut self) -> Option<i64> {
        self.skip_whitespace();
        if self.at_eof() {
            return None;
        }
        let start = self.pos;
        let mut s = String::new();
        if let Some(b) = self.peek() {
            if b == b'-' || b == b'+' {
                s.push(b as char);
                self.pos += 1;
            }
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                s.push(b as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        if s.is_empty() || s == "-" || s == "+" {
            self.pos = start;
            return None;
        }
        s.parse::<i64>().ok()
    }

    /// Mimics scanf %N[<set>]: reads up to `max` characters that match `valid`.
    /// Does NOT skip leading whitespace (matching C scanf %[ behavior).
    /// Returns None on EOF or zero-length match.
    fn scan_set<F: Fn(u8) -> bool>(&mut self, max: usize, valid: F) -> Option<String> {
        if self.at_eof() {
            return None;
        }
        let mut s = String::new();
        while s.len() < max {
            match self.peek() {
                Some(b) if valid(b) => {
                    s.push(b as char);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        if s.is_empty() {
            return None;
        }
        Some(s)
    }
}

fn matches_pattern(expected: &str, actual: &str) -> bool {
    // C: expected[0] == '-' || strcmp(expected, actual) == 0
    expected.as_bytes().first() == Some(&b'-') || expected == actual
}

/// Returns true if some later directive in the list has the same luggage_id and
/// the same departure (replicates the recursive `supersedes`/`superseded` pair).
fn is_superseded(directives: &[RoutingDirective], idx: usize) -> bool {
    let d = &directives[idx];
    for later in directives.iter().skip(idx + 1) {
        if later.luggage_id != d.luggage_id {
            continue;
        }
        return later.departure == d.departure;
    }
    false
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        exit(1);
    }

    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        exit(1);
    }
    let mut scanner = Scanner::new(input);

    let mut directives: Vec<RoutingDirective> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        let time_stamp_signed = match scanner.scan_int() {
            Some(n) => n,
            None => break,
        };
        // The trailing ' ' in the format consumes whitespace.
        scanner.skip_whitespace();

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        let luggage_id = match scanner.scan_set(LUGGAGE_ID_LENGTH, |b| {
            b.is_ascii_uppercase() || b.is_ascii_digit()
        }) {
            Some(s) => s,
            None => break,
        };
        scanner.skip_whitespace();
        let flight_id = match scanner.scan_set(FLIGHT_ID_LENGTH, |b| {
            b.is_ascii_uppercase() || b.is_ascii_digit()
        }) {
            Some(s) => s,
            None => break,
        };
        scanner.skip_whitespace();

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        let departure = match scanner.scan_set(AIRPORT_CODE_LENGTH, |b| b.is_ascii_uppercase()) {
            Some(s) => s,
            None => break,
        };
        scanner.skip_whitespace();
        let arrival = match scanner.scan_set(AIRPORT_CODE_LENGTH, |b| b.is_ascii_uppercase()) {
            Some(s) => s,
            None => break,
        };

        // scanf("%80[^\n]", comments) - does NOT skip leading whitespace,
        // and comments are optional (default empty).
        let mut comments = String::new();
        while comments.len() < COMMENTS_LENGTH {
            match scanner.peek() {
                Some(b) if b != b'\n' => {
                    comments.push(b as char);
                    scanner.pos += 1;
                }
                _ => break,
            }
        }

        // C uses %d into unsigned int -> reinterpret bits as u32.
        let time_stamp = (time_stamp_signed as i32) as u32;

        directives.push(RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        });
    }

    // The C code inserts into a singly-linked list maintained in ascending
    // time_stamp order. For equal time_stamps, the recursion pushes new
    // directives past existing ones, so insertion order is preserved among
    // ties. A stable sort by time_stamp produces the equivalent ordering.
    directives.sort_by(|a, b| a.time_stamp.cmp(&b.time_stamp));

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for i in 0..directives.len() {
        let d = &directives[i];
        if is_superseded(&directives, i) {
            continue;
        }
        if !matches_pattern(&args[1], &d.luggage_id) {
            continue;
        }
        if !matches_pattern(&args[2], &d.flight_id) {
            continue;
        }
        if !matches_pattern(&args[3], &d.departure) {
            continue;
        }
        if !matches_pattern(&args[4], &d.arrival) {
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

    let _ = out.flush();
    exit(0);
}
