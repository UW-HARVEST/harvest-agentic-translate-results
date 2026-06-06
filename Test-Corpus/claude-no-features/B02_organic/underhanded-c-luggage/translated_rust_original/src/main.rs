// By Jan Wrobel <wrr@mixedbit.org>
// Translated from C to Rust.

use std::env;
use std::io::{self, Read, Write, BufWriter};
use std::process::exit;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Default, Clone)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

/// Insert `new_directive` into the singly-linked list (kept sorted by
/// ascending time_stamp). The list is represented as a `Vec` here for
/// safe-Rust simplicity, but the insertion logic mirrors the recursive
/// linked-list insertion in the C version.
fn add_routing_directive_to_list(list: &mut Vec<RoutingDirective>, new_directive: RoutingDirective) {
    // Mirrors the C:
    //   if next == NULL || next.time_stamp > new.time_stamp { insert before next }
    //   else recurse(next, new)
    // i.e. insert before the first existing entry whose time_stamp is
    // strictly greater than the new directive's time_stamp; otherwise
    // append at the end.
    let mut idx = 0usize;
    while idx < list.len() {
        if list[idx].time_stamp > new_directive.time_stamp {
            break;
        }
        idx += 1;
    }
    list.insert(idx, new_directive);
}

fn supersedes_from(list: &[RoutingDirective], start: usize, luggage_id: &str, departure: &str) -> bool {
    // Iterative equivalent of the recursive C `supersedes` function.
    let mut i = start;
    while i < list.len() {
        let directive = &list[i];
        if directive.luggage_id != luggage_id {
            i += 1;
            continue;
        }
        return directive.departure == departure;
    }
    false
}

fn superseded(list: &[RoutingDirective], idx: usize) -> bool {
    let directive = &list[idx];
    supersedes_from(list, idx + 1, &directive.luggage_id, &directive.departure)
}

fn matches(expected: &str, actual: &str) -> bool {
    // expected[0] == '-'  OR  expected == actual
    expected.as_bytes().first().copied() == Some(b'-') || expected == actual
}

fn print_matching_directives<W: Write>(
    out: &mut W,
    list: &[RoutingDirective],
    expected_luggage_id: &str,
    expected_flight_id: &str,
    expected_departure: &str,
    expected_arrival: &str,
) {
    for i in 0..list.len() {
        let d = &list[i];
        if !superseded(list, i)
            && matches(expected_luggage_id, &d.luggage_id)
            && matches(expected_flight_id, &d.flight_id)
            && matches(expected_departure, &d.departure)
            && matches(expected_arrival, &d.arrival)
        {
            // C: printf("%010u %s %s %s %s %s\n", ...)
            let _ = writeln!(
                out,
                "{:010} {} {} {} {} {}",
                d.time_stamp,
                d.luggage_id,
                d.flight_id,
                d.departure,
                d.arrival,
                d.comments
            );
        }
    }
}

/// Minimal scanf-style scanner over raw bytes from stdin.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(Self { data, pos: 0 })
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Skip ASCII whitespace (matches C `isspace`: space, \t, \n, \v, \f, \r).
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Mimic `scanf("%d ", &x)`. Returns Some(value) on success, None on EOF
    /// (no digits could be read because input was exhausted).
    fn scan_uint_with_trailing_ws(&mut self) -> Option<u32> {
        self.skip_ws();
        if self.at_eof() {
            return None;
        }
        // Optional sign.
        let mut neg = false;
        match self.peek() {
            Some(b'-') => {
                neg = true;
                self.pos += 1;
            }
            Some(b'+') => {
                self.pos += 1;
            }
            _ => {}
        }
        let start = self.pos;
        let mut value: i64 = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            // Matching failure — treat like EOF for our purposes (loop should
            // not spin forever on malformed input).
            return None;
        }
        let final_value: i64 = if neg { value.wrapping_neg() } else { value };
        // The C code reads with %d into an unsigned int*, so the bit pattern
        // is reinterpreted as u32.
        let u: u32 = final_value as u32;
        // Trailing whitespace directive in "%d ".
        self.skip_ws();
        Some(u)
    }

    /// Read up to `max` bytes that satisfy `pred` (mirrors `%N[set]`).
    fn scan_charset<F: Fn(u8) -> bool>(&mut self, max: usize, pred: F) -> String {
        let mut out = Vec::with_capacity(max);
        while out.len() < max {
            match self.peek() {
                Some(c) if pred(c) => {
                    out.push(c);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        // Inputs are ASCII per the conversion specifiers used.
        String::from_utf8(out).unwrap_or_default()
    }
}

fn is_upper_or_digit(c: u8) -> bool {
    (b'A'..=b'Z').contains(&c) || c.is_ascii_digit()
}

fn is_upper(c: u8) -> bool {
    (b'A'..=b'Z').contains(&c)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        exit(1);
    }

    let mut scanner = match Scanner::new() {
        Ok(s) => s,
        Err(_) => {
            // If stdin can't be read, behave as if EOF immediately.
            Scanner { data: Vec::new(), pos: 0 }
        }
    };

    let mut directives: Vec<RoutingDirective> = Vec::new();

    loop {
        // scanf("%d ", &time_stamp)
        let time_stamp = match scanner.scan_uint_with_trailing_ws() {
            Some(v) => v,
            None => break, // EOF
        };

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        if scanner.at_eof() {
            break;
        }
        let luggage_id = scanner.scan_charset(LUGGAGE_ID_LENGTH, is_upper_or_digit);
        scanner.skip_ws();
        let flight_id = scanner.scan_charset(FLIGHT_ID_LENGTH, is_upper_or_digit);
        scanner.skip_ws();

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        if scanner.at_eof() {
            break;
        }
        let departure = scanner.scan_charset(AIRPORT_CODE_LENGTH, is_upper);
        scanner.skip_ws();
        let arrival = scanner.scan_charset(AIRPORT_CODE_LENGTH, is_upper);

        // scanf("%80[^\n]", comments)
        // NOTE: no leading whitespace skip here, so a leading space (between
        // arrival and the comments text) is included in the comments field —
        // this matches the original C behavior exactly.
        if scanner.at_eof() {
            // scanf would return EOF; the C code breaks without adding this
            // directive.
            break;
        }
        // `comments[0] = 0` was set before the scanf in C, so a matching
        // failure (e.g., immediate '\n') leaves comments empty.
        let comments = scanner.scan_charset(COMMENTS_LENGTH, |c| c != b'\n');

        let new_directive = RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        };
        add_routing_directive_to_list(&mut directives, new_directive);
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    print_matching_directives(
        &mut out,
        &directives,
        &args[1],
        &args[2],
        &args[3],
        &args[4],
    );
    let _ = out.flush();
    exit(0);
}
