use std::env;
use std::io::{self, Read, Write};
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

struct Parser {
    input: Vec<u8>,
    pos: usize,
}

impl Parser {
    fn new(input: Vec<u8>) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && is_space(self.input[self.pos]) {
            self.pos += 1;
        }
    }

    fn scan_time_stamp(&mut self, time_stamp: &mut u32) -> bool {
        self.skip_ws();
        if self.pos >= self.input.len() {
            return true;
        }

        let token_start = self.pos;
        let mut sign = 1i64;
        if matches!(self.input[self.pos], b'+' | b'-') {
            if self.input[self.pos] == b'-' {
                sign = -1;
            }
            self.pos += 1;
        }

        let digits_start = self.pos;
        let mut value: i64 = 0;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            value = value * 10 + i64::from(self.input[self.pos] - b'0');
            self.pos += 1;
        }

        if self.pos == digits_start {
            self.pos = token_start;
            return false;
        }

        let signed = (value * sign) as i32;
        *time_stamp = signed as u32;
        self.skip_ws();
        false
    }

    fn scan_luggage_and_flight(
        &mut self,
        luggage_id: &mut Vec<u8>,
        flight_id: &mut Vec<u8>,
    ) -> bool {
        if self.pos >= self.input.len() {
            return true;
        }

        let first = self.scan_scanset(LUGGAGE_ID_LENGTH, is_upper_alnum);
        let Some(first) = first else {
            return false;
        };
        *luggage_id = first;

        self.skip_ws();
        if self.pos >= self.input.len() {
            return false;
        }

        let second = self.scan_scanset(FLIGHT_ID_LENGTH, is_upper_alnum);
        let Some(second) = second else {
            return false;
        };
        *flight_id = second;

        self.skip_ws();
        false
    }

    fn scan_departure_and_arrival(
        &mut self,
        departure: &mut Vec<u8>,
        arrival: &mut Vec<u8>,
    ) -> bool {
        if self.pos >= self.input.len() {
            return true;
        }

        let first = self.scan_scanset(AIRPORT_CODE_LENGTH, is_upper_alpha);
        let Some(first) = first else {
            return false;
        };
        *departure = first;

        self.skip_ws();
        if self.pos >= self.input.len() {
            return false;
        }

        let second = self.scan_scanset(AIRPORT_CODE_LENGTH, is_upper_alpha);
        let Some(second) = second else {
            return false;
        };
        *arrival = second;
        false
    }

    fn scan_comments(&mut self, comments: &mut Vec<u8>) -> bool {
        if self.pos >= self.input.len() {
            return true;
        }
        if self.input[self.pos] == b'\n' {
            return false;
        }

        let start = self.pos;
        while self.pos < self.input.len()
            && self.input[self.pos] != b'\n'
            && self.pos - start < COMMENTS_LENGTH
        {
            self.pos += 1;
        }
        *comments = self.input[start..self.pos].to_vec();
        false
    }

    fn scan_scanset<F>(&mut self, width: usize, predicate: F) -> Option<Vec<u8>>
    where
        F: Fn(u8) -> bool,
    {
        if self.pos >= self.input.len() {
            return None;
        }
        if !predicate(self.input[self.pos]) {
            return None;
        }

        let start = self.pos;
        while self.pos < self.input.len()
            && predicate(self.input[self.pos])
            && self.pos - start < width
        {
            self.pos += 1;
        }
        Some(self.input[start..self.pos].to_vec())
    }
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | 0x0b | 0x0c)
}

fn is_upper_alnum(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn is_upper_alpha(byte: u8) -> bool {
    byte.is_ascii_uppercase()
}

fn add_routing_directive_to_list(
    directives: &mut Vec<RoutingDirective>,
    new_directive: RoutingDirective,
) {
    let insert_at = directives
        .iter()
        .position(|directive| directive.time_stamp > new_directive.time_stamp)
        .unwrap_or(directives.len());
    directives.insert(insert_at, new_directive);
}

fn supersedes(directives: &[RoutingDirective], luggage_id: &[u8], departure: &[u8]) -> bool {
    if directives.is_empty() {
        return false;
    }
    if directives[0].luggage_id != luggage_id {
        return supersedes(&directives[1..], luggage_id, departure);
    }
    if directives[0].departure == departure {
        return true;
    }
    false
}

fn superseded(directives: &[RoutingDirective], index: usize) -> bool {
    supersedes(
        &directives[index + 1..],
        &directives[index].luggage_id,
        &directives[index].departure,
    )
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

fn print_matching_directives(
    directives: &[RoutingDirective],
    expected_luggage_id: &[u8],
    expected_flight_id: &[u8],
    expected_departure: &[u8],
    expected_arrival: &[u8],
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for (index, directive) in directives.iter().enumerate() {
        if !superseded(directives, index)
            && matches(expected_luggage_id, &directive.luggage_id)
            && matches(expected_flight_id, &directive.flight_id)
            && matches(expected_departure, &directive.departure)
            && matches(expected_arrival, &directive.arrival)
        {
            write!(out, "{:010} ", directive.time_stamp)?;
            out.write_all(&directive.luggage_id)?;
            out.write_all(b" ")?;
            out.write_all(&directive.flight_id)?;
            out.write_all(b" ")?;
            out.write_all(&directive.departure)?;
            out.write_all(b" ")?;
            out.write_all(&directive.arrival)?;
            out.write_all(b" ")?;
            out.write_all(&directive.comments)?;
            out.write_all(b"\n")?;
        }
    }

    out.flush()
}

fn main() {
    let args: Vec<Vec<u8>> = env::args_os()
        .map(|arg| std::os::unix::ffi::OsStringExt::into_vec(arg))
        .collect();

    if args.len() != 5 {
        let _ = io::stderr().write_all(b"Command line error: 4 arguments expected\n");
        process::exit(1);
    }

    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        process::exit(1);
    }

    let mut parser = Parser::new(input);
    let mut directives = Vec::new();

    loop {
        let mut time_stamp = 0u32;
        let mut luggage_id = Vec::new();
        let mut flight_id = Vec::new();
        let mut departure = Vec::new();
        let mut arrival = Vec::new();
        let mut comments = Vec::new();

        if parser.scan_time_stamp(&mut time_stamp) {
            break;
        }
        if parser.scan_luggage_and_flight(&mut luggage_id, &mut flight_id) {
            break;
        }
        if parser.scan_departure_and_arrival(&mut departure, &mut arrival) {
            break;
        }
        if parser.scan_comments(&mut comments) {
            break;
        }

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

    if print_matching_directives(&directives, &args[1], &args[2], &args[3], &args[4]).is_err() {
        process::exit(1);
    }
}
