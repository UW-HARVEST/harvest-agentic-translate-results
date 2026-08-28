use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::process;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Debug)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

enum Scan<T> {
    Value(T),
    Eof,
    ConversionFailure,
}

struct Scanner {
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn new(input: Vec<u8>) -> Self {
        Self { input, position: 0 }
    }

    fn is_eof(&self) -> bool {
        self.position == self.input.len()
    }

    fn skip_whitespace(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
        {
            self.position += 1;
        }
    }

    // Mirrors scanf("%d "): decimal conversion followed by a whitespace directive.
    fn scan_time_stamp(&mut self) -> Scan<u32> {
        self.skip_whitespace();
        if self.is_eof() {
            return Scan::Eof;
        }

        let negative = match self.input[self.position] {
            b'-' => {
                self.position += 1;
                true
            }
            b'+' => {
                self.position += 1;
                false
            }
            _ => false,
        };

        let digit_start = self.position;
        let limit = if negative {
            (i64::MAX as u64) + 1
        } else {
            i64::MAX as u64
        };
        let mut magnitude = 0_u64;
        while let Some(byte @ b'0'..=b'9') = self.input.get(self.position) {
            magnitude = magnitude
                .saturating_mul(10)
                .saturating_add(u64::from(*byte - b'0'))
                .min(limit);
            self.position += 1;
        }

        if self.position == digit_start {
            return Scan::ConversionFailure;
        }

        self.skip_whitespace();
        let value = if negative {
            0_u32.wrapping_sub(magnitude as u32)
        } else {
            magnitude as u32
        };
        Scan::Value(value)
    }

    fn scan_set<F>(&mut self, width: usize, accepted: F) -> Scan<Vec<u8>>
    where
        F: Fn(u8) -> bool,
    {
        if self.is_eof() {
            return Scan::Eof;
        }

        let start = self.position;
        while self.position - start < width
            && self
                .input
                .get(self.position)
                .is_some_and(|byte| accepted(*byte))
        {
            self.position += 1;
        }

        if self.position == start {
            Scan::ConversionFailure
        } else {
            Scan::Value(self.input[start..self.position].to_vec())
        }
    }

    // Mirrors scanf("%8[A-Z0-9] %6[A-Z0-9] ").
    fn scan_ids(&mut self) -> Scan<(Vec<u8>, Vec<u8>)> {
        let luggage_id = match self.scan_set(LUGGAGE_ID_LENGTH, is_uppercase_or_digit) {
            Scan::Value(value) => value,
            Scan::Eof => return Scan::Eof,
            Scan::ConversionFailure => return Scan::ConversionFailure,
        };

        self.skip_whitespace();
        let flight_id = match self.scan_set(FLIGHT_ID_LENGTH, is_uppercase_or_digit) {
            Scan::Value(value) => value,
            // scanf returns one successful conversion here, not EOF. The C then
            // reads uninitialized storage, so no deterministic behavior exists.
            Scan::Eof | Scan::ConversionFailure => return Scan::ConversionFailure,
        };
        self.skip_whitespace();

        Scan::Value((luggage_id, flight_id))
    }

    // Mirrors scanf("%3[A-Z] %3[A-Z]").
    fn scan_airports(&mut self) -> Scan<(Vec<u8>, Vec<u8>)> {
        let departure = match self.scan_set(AIRPORT_CODE_LENGTH, is_uppercase) {
            Scan::Value(value) => value,
            Scan::Eof => return Scan::Eof,
            Scan::ConversionFailure => return Scan::ConversionFailure,
        };

        self.skip_whitespace();
        let arrival = match self.scan_set(AIRPORT_CODE_LENGTH, is_uppercase) {
            Scan::Value(value) => value,
            Scan::Eof | Scan::ConversionFailure => return Scan::ConversionFailure,
        };

        Scan::Value((departure, arrival))
    }

    // Mirrors scanf("%80[^\n]"). An immediate newline is a conversion failure,
    // but the caller's preinitialized empty comment remains valid.
    fn scan_comments(&mut self) -> Scan<Vec<u8>> {
        self.scan_set(COMMENTS_LENGTH, |byte| byte != b'\n')
    }
}

fn is_uppercase(byte: u8) -> bool {
    byte.is_ascii_uppercase()
}

fn is_uppercase_or_digit(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn c_string(bytes: Vec<u8>) -> Vec<u8> {
    match bytes.iter().position(|byte| *byte == 0) {
        Some(end) => bytes[..end].to_vec(),
        None => bytes,
    }
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

fn superseded(directives: &[RoutingDirective], index: usize) -> bool {
    let directive = &directives[index];
    for later in &directives[index + 1..] {
        if later.luggage_id != directive.luggage_id {
            continue;
        }
        return later.departure == directive.departure;
    }
    false
}

fn main() {
    let arguments: Vec<Vec<u8>> = env::args_os().map(OsString::into_vec).collect();
    if arguments.len() != 5 {
        let _ = io::stderr().write_all(b"Command line error: 4 arguments expected\n");
        process::exit(1);
    }

    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let mut scanner = Scanner::new(input);
    let mut directives = Vec::new();

    loop {
        let time_stamp = match scanner.scan_time_stamp() {
            Scan::Value(value) => value,
            Scan::Eof => break,
            Scan::ConversionFailure => break,
        };
        let (luggage_id, flight_id) = match scanner.scan_ids() {
            Scan::Value(value) => value,
            Scan::Eof => break,
            Scan::ConversionFailure => break,
        };
        let (departure, arrival) = match scanner.scan_airports() {
            Scan::Value(value) => value,
            Scan::Eof => break,
            Scan::ConversionFailure => break,
        };
        let comments = match scanner.scan_comments() {
            Scan::Value(value) => value,
            Scan::Eof => break,
            Scan::ConversionFailure => Vec::new(),
        };

        let directive = RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments: c_string(comments),
        };
        let insertion_index = directives
            .iter()
            .position(|existing: &RoutingDirective| existing.time_stamp > time_stamp)
            .unwrap_or(directives.len());
        directives.insert(insertion_index, directive);
    }

    let mut output = Vec::new();
    for (index, directive) in directives.iter().enumerate() {
        if !superseded(&directives, index)
            && matches(&arguments[1], &directive.luggage_id)
            && matches(&arguments[2], &directive.flight_id)
            && matches(&arguments[3], &directive.departure)
            && matches(&arguments[4], &directive.arrival)
        {
            write!(&mut output, "{:010} ", directive.time_stamp).unwrap();
            output.extend_from_slice(&directive.luggage_id);
            output.push(b' ');
            output.extend_from_slice(&directive.flight_id);
            output.push(b' ');
            output.extend_from_slice(&directive.departure);
            output.push(b' ');
            output.extend_from_slice(&directive.arrival);
            output.push(b' ');
            output.extend_from_slice(&directive.comments);
            output.push(b'\n');
        }
    }
    let _ = io::stdout().write_all(&output);
}
