// Rust translation of luggage.c — produces byte-identical output for the same inputs.
//
// scanf semantics (C99/glibc) we replicate:
//   * `%d` skips leading whitespace, then reads digits; returns EOF only if the
//     stream is at EOF before any non-whitespace was seen, otherwise on a non-
//     digit it's a matching failure (returns 0) and the value is unchanged.
//   * `%N[set]` does NOT skip leading whitespace; it returns EOF only if the
//     stream is at EOF before the directive starts; otherwise zero or more
//     matching characters are read. Zero matches → matching failure (the
//     destination is unchanged).
//   * A literal space in the format consumes zero or more whitespace bytes.
//
// The C program only breaks the read loop on EOF, never on matching failure.
// On matching failure the destination keeps its previous value (which is the
// stack value from the previous iteration in the C program — undefined for the
// first iteration). We emulate by reusing buffers across iterations.

use std::env;
use std::io::{self, Read, Write};
use std::process::exit;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(buf: Vec<u8>) -> Self {
        Self { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    // Matches C's isspace() for the "C" locale.
    fn is_ws(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if Self::is_ws(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    // Read up to `max` chars matching `pred`, writing them into `dest`.
    // If at least one char is read, `dest` is replaced with the new bytes.
    // If zero chars are read (matching failure), `dest` is left unchanged
    // (mirroring scanf's behaviour on a matching failure).
    fn read_set<F: Fn(u8) -> bool>(&mut self, max: usize, pred: F, dest: &mut Vec<u8>) -> bool {
        let start = self.pos;
        while self.pos - start < max {
            match self.peek() {
                Some(c) if pred(c) => self.advance(),
                _ => break,
            }
        }
        if self.pos == start {
            false
        } else {
            dest.clear();
            dest.extend_from_slice(&self.buf[start..self.pos]);
            true
        }
    }

    // Parse a signed decimal int into `dest`. Returns true if at least one
    // digit was consumed (success); false otherwise. On false, `dest` is
    // unchanged. Position is rewound to before any sign character on failure.
    fn parse_int(&mut self, dest: &mut u32) -> bool {
        let start_pos = self.pos;
        let mut neg = false;
        if let Some(c) = self.peek() {
            if c == b'-' {
                neg = true;
                self.advance();
            } else if c == b'+' {
                self.advance();
            }
        }
        let digit_start = self.pos;
        let mut val: i64 = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                val = val.wrapping_mul(10).wrapping_add((c - b'0') as i64);
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == digit_start {
            self.pos = start_pos;
            return false;
        }
        if neg {
            val = val.wrapping_neg();
        }
        // Truncate to int (i32) like C's `%d`, then bit-cast to u32 for storage.
        *dest = (val as i32) as u32;
        true
    }
}

fn matches_field(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

// Recursive `supersedes` matching the C implementation's structure.
fn supersedes(list: &[RoutingDirective], idx: usize, luggage_id: &[u8], departure: &[u8]) -> bool {
    if idx >= list.len() {
        return false;
    }
    let d = &list[idx];
    if d.luggage_id != luggage_id {
        return supersedes(list, idx + 1, luggage_id, departure);
    }
    d.departure == departure
}

fn superseded(list: &[RoutingDirective], idx: usize) -> bool {
    let d = &list[idx];
    supersedes(list, idx + 1, &d.luggage_id, &d.departure)
}

fn print_matching_directives<W: Write>(
    out: &mut W,
    list: &[RoutingDirective],
    expected_luggage_id: &[u8],
    expected_flight_id: &[u8],
    expected_departure: &[u8],
    expected_arrival: &[u8],
) {
    for i in 0..list.len() {
        let d = &list[i];
        if !superseded(list, i)
            && matches_field(expected_luggage_id, &d.luggage_id)
            && matches_field(expected_flight_id, &d.flight_id)
            && matches_field(expected_departure, &d.departure)
            && matches_field(expected_arrival, &d.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
            write!(out, "{:010} ", d.time_stamp).unwrap();
            out.write_all(&d.luggage_id).unwrap();
            out.write_all(b" ").unwrap();
            out.write_all(&d.flight_id).unwrap();
            out.write_all(b" ").unwrap();
            out.write_all(&d.departure).unwrap();
            out.write_all(b" ").unwrap();
            out.write_all(&d.arrival).unwrap();
            out.write_all(b" ").unwrap();
            out.write_all(&d.comments).unwrap();
            out.write_all(b"\n").unwrap();
        }
    }
}

// Insert into list using the same comparison the recursive C insertion uses:
// the new element ends up after every existing element whose time_stamp is
// <= new element's time_stamp (because the C code only short-circuits on a
// strict `>`).
fn add_directive(list: &mut Vec<RoutingDirective>, new_dir: RoutingDirective) {
    let mut i = 0;
    while i < list.len() && list[i].time_stamp <= new_dir.time_stamp {
        i += 1;
    }
    list.insert(i, new_dir);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        let stderr = io::stderr();
        let mut err = stderr.lock();
        err.write_all(b"Command line error: 4 arguments expected\n").unwrap();
        exit(1);
    }

    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let mut scanner = Scanner::new(input);

    let mut list: Vec<RoutingDirective> = Vec::new();

    // Persistent buffers across iterations to mimic stack-slot reuse in C.
    // Initial values are zero/empty; with malformed input the C code reads
    // uninitialized stack memory, which is undefined behaviour, so on the
    // very first failing match these will simply stay empty/zero.
    let mut time_stamp: u32 = 0;
    let mut luggage_id: Vec<u8> = Vec::new();
    let mut flight_id: Vec<u8> = Vec::new();
    let mut departure: Vec<u8> = Vec::new();
    let mut arrival: Vec<u8> = Vec::new();
    let mut comments: Vec<u8>;

    loop {
        // The C code resets `comments[0] = 0;` at the top of each iteration.
        comments = Vec::new();

        // scanf("%d ", &time_stamp): %d skips leading whitespace; if EOF is
        // reached during that skip, scanf returns EOF and the C code breaks.
        scanner.skip_ws();
        if scanner.at_eof() {
            break;
        }
        // %d itself: matching failure leaves time_stamp unchanged.
        let _ = scanner.parse_int(&mut time_stamp);
        // Trailing space in the format consumes any following whitespace.
        scanner.skip_ws();

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id):
        // No leading whitespace skip; only EOF here triggers the break.
        if scanner.at_eof() {
            break;
        }
        let _ = scanner.read_set(LUGGAGE_ID_LENGTH, |c| c.is_ascii_uppercase() || c.is_ascii_digit(), &mut luggage_id);
        scanner.skip_ws();
        let _ = scanner.read_set(FLIGHT_ID_LENGTH, |c| c.is_ascii_uppercase() || c.is_ascii_digit(), &mut flight_id);
        scanner.skip_ws();

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival):
        // No leading whitespace skip; only EOF triggers the break.
        if scanner.at_eof() {
            break;
        }
        let _ = scanner.read_set(AIRPORT_CODE_LENGTH, |c| c.is_ascii_uppercase(), &mut departure);
        scanner.skip_ws();
        let _ = scanner.read_set(AIRPORT_CODE_LENGTH, |c| c.is_ascii_uppercase(), &mut arrival);

        // scanf("%80[^\n]", comments): no leading ws skip; only EOF triggers
        // the break. On matching failure (e.g., the next char is already \n)
        // the buffer is left unchanged — and because we just reset
        // `comments` at the top of the iteration, that means an empty string,
        // exactly like the C code (`comments[0] = 0`).
        if scanner.at_eof() {
            break;
        }
        let _ = scanner.read_set(COMMENTS_LENGTH, |c| c != b'\n', &mut comments);

        let new_dir = RoutingDirective {
            time_stamp,
            luggage_id: luggage_id.clone(),
            flight_id: flight_id.clone(),
            departure: departure.clone(),
            arrival: arrival.clone(),
            comments: comments.clone(),
        };
        add_directive(&mut list, new_dir);
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    print_matching_directives(
        &mut out,
        &list,
        args[1].as_bytes(),
        args[2].as_bytes(),
        args[3].as_bytes(),
        args[4].as_bytes(),
    );
    out.flush().unwrap();
    exit(0);
}
