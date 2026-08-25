use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process;

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
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn new(input: Vec<u8>) -> Self {
        Self { input, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(|byte| is_c_whitespace(*byte))
        {
            self.position += 1;
        }
    }

    fn scan_timestamp(&mut self, value: &mut u32) -> i32 {
        self.skip_whitespace();
        if self.position == self.input.len() {
            return -1;
        }

        let start = self.position;
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
        let digits_start = self.position;
        let mut magnitude = 0_u128;
        while let Some(byte @ b'0'..=b'9') = self.input.get(self.position).copied() {
            magnitude = magnitude
                .saturating_mul(10)
                .saturating_add(u128::from(byte - b'0'));
            self.position += 1;
        }

        if self.position == digits_start {
            self.position = start;
            return 0;
        }

        let signed = if negative {
            if magnitude >= (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };
        *value = signed as u32;

        // The whitespace at the end of "%d " consumes across line boundaries.
        self.skip_whitespace();
        1
    }

    fn scan_set<F>(&mut self, maximum: usize, output: &mut Vec<u8>, accepts: F) -> i32
    where
        F: Fn(u8) -> bool,
    {
        if self.position == self.input.len() {
            return -1;
        }
        if !accepts(self.input[self.position]) {
            return 0;
        }

        output.clear();
        while output.len() < maximum {
            let Some(&byte) = self.input.get(self.position) else {
                break;
            };
            if !accepts(byte) {
                break;
            }
            output.push(byte);
            self.position += 1;
        }
        1
    }

    fn scan_ids(&mut self, luggage_id: &mut Vec<u8>, flight_id: &mut Vec<u8>) -> i32 {
        let first = self.scan_set(LUGGAGE_ID_LENGTH, luggage_id, is_uppercase_or_digit);
        if first <= 0 {
            return first;
        }

        self.skip_whitespace();
        let second = self.scan_set(FLIGHT_ID_LENGTH, flight_id, is_uppercase_or_digit);
        if second <= 0 {
            return 1;
        }

        self.skip_whitespace();
        2
    }

    fn scan_airports(&mut self, departure: &mut Vec<u8>, arrival: &mut Vec<u8>) -> i32 {
        let first = self.scan_set(AIRPORT_CODE_LENGTH, departure, is_uppercase);
        if first <= 0 {
            return first;
        }

        self.skip_whitespace();
        let second = self.scan_set(AIRPORT_CODE_LENGTH, arrival, is_uppercase);
        if second <= 0 {
            return 1;
        }
        2
    }

    fn scan_comments(&mut self, comments: &mut Vec<u8>) -> i32 {
        self.scan_set(COMMENTS_LENGTH, comments, |byte| byte != b'\n')
    }
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_uppercase(byte: u8) -> bool {
    byte.is_ascii_uppercase()
}

fn is_uppercase_or_digit(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn as_c_string(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect()
}

fn add_routing_directive_to_list(
    directives: &mut Vec<RoutingDirective>,
    new_directive: RoutingDirective,
) {
    let insertion_point = directives
        .iter()
        .position(|directive| directive.time_stamp > new_directive.time_stamp)
        .unwrap_or(directives.len());
    directives.insert(insertion_point, new_directive);
}

fn superseded(directives: &[RoutingDirective], index: usize) -> bool {
    let directive = &directives[index];
    for later in &directives[index + 1..] {
        if later.luggage_id == directive.luggage_id {
            return later.departure == directive.departure;
        }
    }
    false
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

fn main() {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 5 {
        let _ = io::stderr().write_all(b"Command line error: 4 arguments expected\n");
        process::exit(1);
    }
    let expected: Vec<&[u8]> = arguments[1..]
        .iter()
        .map(|argument| argument.as_os_str().as_bytes())
        .collect();

    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);
    let mut directives = Vec::new();

    loop {
        let mut time_stamp = 0_u32;
        let mut luggage_id = Vec::new();
        let mut flight_id = Vec::new();
        let mut departure = Vec::new();
        let mut arrival = Vec::new();
        let mut comments = Vec::new();

        if scanner.scan_timestamp(&mut time_stamp) == -1 {
            break;
        }
        if scanner.scan_ids(&mut luggage_id, &mut flight_id) == -1 {
            break;
        }
        if scanner.scan_airports(&mut departure, &mut arrival) == -1 {
            break;
        }
        if scanner.scan_comments(&mut comments) == -1 {
            break;
        }

        add_routing_directive_to_list(
            &mut directives,
            RoutingDirective {
                time_stamp,
                luggage_id: as_c_string(&luggage_id),
                flight_id: as_c_string(&flight_id),
                departure: as_c_string(&departure),
                arrival: as_c_string(&arrival),
                comments: as_c_string(&comments),
            },
        );
    }

    let mut output = Vec::new();
    for (index, directive) in directives.iter().enumerate() {
        if !superseded(&directives, index)
            && matches(expected[0], &directive.luggage_id)
            && matches(expected[1], &directive.flight_id)
            && matches(expected[2], &directive.departure)
            && matches(expected[3], &directive.arrival)
        {
            let mut timestamp = String::new();
            write!(&mut timestamp, "{:010}", directive.time_stamp).unwrap();
            output.extend_from_slice(timestamp.as_bytes());
            output.push(b' ');
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
