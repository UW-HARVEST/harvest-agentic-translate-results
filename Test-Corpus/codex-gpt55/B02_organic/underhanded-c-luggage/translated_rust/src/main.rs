use std::env;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Clone)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

struct Scanner {
    input: Vec<u8>,
    pos: usize,
}

enum ScanResult<T> {
    Ok(T),
    Eof,
    Mismatch,
}

impl Scanner {
    fn new(input: Vec<u8>) -> Self {
        Self { input, pos: 0 }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn is_space(byte: u8) -> bool {
        matches!(byte, b' ' | b'\n' | b'\t' | 0x0b | 0x0c | b'\r')
    }

    fn consume_space_directive(&mut self) {
        while !self.at_eof() && Self::is_space(self.input[self.pos]) {
            self.pos += 1;
        }
    }

    fn scan_i32_with_trailing_space(&mut self) -> ScanResult<u32> {
        self.consume_space_directive();
        if self.at_eof() {
            return ScanResult::Eof;
        }

        let start = self.pos;
        let mut sign = 1i64;
        if matches!(self.input[self.pos], b'+' | b'-') {
            if self.input[self.pos] == b'-' {
                sign = -1;
            }
            self.pos += 1;
        }

        let digits_start = self.pos;
        while !self.at_eof() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos == digits_start {
            self.pos = start;
            return ScanResult::Mismatch;
        }

        let limit = if sign < 0 {
            (i64::MAX as u128) + 1
        } else {
            i64::MAX as u128
        };
        let mut value = 0u128;
        for &byte in &self.input[digits_start..self.pos] {
            value = value
                .saturating_mul(10)
                .saturating_add((byte - b'0') as u128)
                .min(limit);
        }
        let signed = if sign < 0 {
            if value == limit {
                i64::MIN
            } else {
                -(value as i64)
            }
        } else {
            value as i64
        } as i32;
        self.consume_space_directive();
        ScanResult::Ok(signed as u32)
    }

    fn scan_set<F>(&mut self, max_len: usize, accepts: F) -> ScanResult<Vec<u8>>
    where
        F: Fn(u8) -> bool,
    {
        if self.at_eof() {
            return ScanResult::Eof;
        }
        if !accepts(self.input[self.pos]) {
            return ScanResult::Mismatch;
        }

        let start = self.pos;
        let mut len = 0usize;
        while !self.at_eof() && len < max_len && accepts(self.input[self.pos]) {
            self.pos += 1;
            len += 1;
        }
        ScanResult::Ok(c_string_bytes(&self.input[start..self.pos]))
    }
}

fn c_string_bytes(bytes: &[u8]) -> Vec<u8> {
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    bytes[..len].to_vec()
}

fn upper_alnum(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn upper_alpha(byte: u8) -> bool {
    byte.is_ascii_uppercase()
}

fn not_newline(byte: u8) -> bool {
    byte != b'\n'
}

fn add_routing_directive_to_list(
    directives: &mut Vec<RoutingDirective>,
    new_directive: RoutingDirective,
) {
    let index = directives
        .iter()
        .position(|directive| directive.time_stamp > new_directive.time_stamp)
        .unwrap_or(directives.len());
    directives.insert(index, new_directive);
}

fn supersedes(directives: &[RoutingDirective], luggage_id: &[u8], departure: &[u8]) -> bool {
    for directive in directives {
        if directive.luggage_id != luggage_id {
            continue;
        }
        return directive.departure == departure;
    }
    false
}

fn matches_expected(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

fn main() {
    let args: Vec<Vec<u8>> = env::args_os()
        .map(|arg| OsStr::new(&arg).as_bytes().to_vec())
        .collect();

    if args.len() != 5 {
        let _ = io::stderr().write_all(b"Command line error: 4 arguments expected\n");
        process::exit(1);
    }

    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);
    let mut directives = Vec::new();

    loop {
        let time_stamp = match scanner.scan_i32_with_trailing_space() {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => 0,
        };

        let luggage_id = match scanner.scan_set(LUGGAGE_ID_LENGTH, upper_alnum) {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => Vec::new(),
        };
        if !luggage_id.is_empty() {
            scanner.consume_space_directive();
        }

        let flight_id = match scanner.scan_set(FLIGHT_ID_LENGTH, upper_alnum) {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => Vec::new(),
        };
        if !flight_id.is_empty() {
            scanner.consume_space_directive();
        }

        let departure = match scanner.scan_set(AIRPORT_CODE_LENGTH, upper_alpha) {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => Vec::new(),
        };
        if !departure.is_empty() {
            scanner.consume_space_directive();
        }

        let arrival = match scanner.scan_set(AIRPORT_CODE_LENGTH, upper_alpha) {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => Vec::new(),
        };

        let comments = match scanner.scan_set(COMMENTS_LENGTH, not_newline) {
            ScanResult::Ok(value) => value,
            ScanResult::Eof => break,
            ScanResult::Mismatch => Vec::new(),
        };

        add_routing_directive_to_list(
            &mut directives,
            RoutingDirective {
                time_stamp,
                luggage_id,
                flight_id,
                departure,
                arrival,
                comments,
            },
        );
    }

    let expected_luggage_id = &args[1];
    let expected_flight_id = &args[2];
    let expected_departure = &args[3];
    let expected_arrival = &args[4];

    let mut stdout = io::BufWriter::new(io::stdout());
    for (index, directive) in directives.iter().enumerate() {
        let later_directives = directives.get(index + 1..).unwrap_or(&[]);
        if !supersedes(
            later_directives,
            &directive.luggage_id,
            &directive.departure,
        ) && matches_expected(expected_luggage_id, &directive.luggage_id)
            && matches_expected(expected_flight_id, &directive.flight_id)
            && matches_expected(expected_departure, &directive.departure)
            && matches_expected(expected_arrival, &directive.arrival)
        {
            let _ = write!(stdout, "{:010} ", directive.time_stamp);
            let _ = stdout.write_all(&directive.luggage_id);
            let _ = stdout.write_all(b" ");
            let _ = stdout.write_all(&directive.flight_id);
            let _ = stdout.write_all(b" ");
            let _ = stdout.write_all(&directive.departure);
            let _ = stdout.write_all(b" ");
            let _ = stdout.write_all(&directive.arrival);
            let _ = stdout.write_all(b" ");
            let _ = stdout.write_all(&directive.comments);
            let _ = stdout.write_all(b"\n");
        }
    }
}
