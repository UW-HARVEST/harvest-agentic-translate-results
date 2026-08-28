// Translation of c_src/src/luggage.c ("By Jan Wrobel <wrr@mixedbit.org>") to Rust.
//
// The goal is byte-identical output for identical input, so the original
// behaviour -- including its quirks -- is reproduced rather than corrected:
//
//   * `scanf("%d ", &time_stamp)` reads a *signed* `int` through a pointer to
//     `unsigned int`, so negative values wrap around when printed with `%010u`.
//   * `%[...]` conversions do not skip leading whitespace, only the explicit
//     spaces in the format strings (and the leading skip built into `%d`) do.
//   * The last conversion, `%80[^\n]`, is not preceded by a space directive, so
//     a comment keeps the blank that separates it from the arrival code. Since
//     `printf` adds another blank, comments are printed with two spaces before
//     them.
//   * A matching failure (as opposed to end of file) does not break the loop:
//     the record is still appended, reusing whatever the (stack allocated)
//     buffers happened to contain from a previous iteration.
//   * `supersedes` stops at the *first* later directive carrying the same
//     luggage id, so only that one directive decides whether this one is
//     superseded.

use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

/// Value `scanf` returns on input failure before the first conversion.
const EOF: i32 = -1;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

/// Byte oriented stand-in for a `FILE*` positioned on stdin. Holding the whole
/// input allows unlimited push back, which the `scanf` emulation below needs;
/// the program only produces output after reaching end of file anyway.
struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// A whitespace directive in a format string: consume any run of
    /// whitespace. It never fails, not even at end of file.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if is_c_space(c) {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// `%d` followed by a whitespace directive, i.e. `scanf("%d ", ...)`.
    ///
    /// Returns the number of assignments (1) or `EOF`/0 exactly like `scanf`.
    /// The stored value mirrors glibc: the digits are converted as a `long`
    /// (saturating on overflow, as `strtol` does) and then truncated to `int`.
    fn scan_int_then_space(&mut self, out: &mut i32) -> i32 {
        self.skip_whitespace();
        if self.at_eof() {
            return EOF;
        }

        let start = self.pos;
        let mut negative = false;
        match self.peek() {
            Some(b'+') => self.bump(),
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            _ => {}
        }

        let mut digits = 0usize;
        let mut magnitude: i128 = 0;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            digits += 1;
            // Stop accumulating once the value is far beyond `long`; the clamp
            // below yields the same result either way.
            if magnitude <= (1i128 << 70) {
                magnitude = magnitude * 10 + i128::from(c - b'0');
            }
            self.bump();
        }

        if digits == 0 {
            // Matching failure: the offending characters stay in the stream.
            self.pos = start;
            return 0;
        }

        let signed = if negative { -magnitude } else { magnitude };
        let as_long = signed.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        *out = as_long as i32;

        self.skip_whitespace();
        1
    }

    /// A `%<width>[...]` conversion. `out` must have room for `width` bytes
    /// plus the terminating NUL. Returns 1 on success, 0 on a matching failure
    /// and `EOF` when the stream is already exhausted.
    fn scan_set<F>(&mut self, width: usize, accepts: F, out: &mut [u8]) -> i32
    where
        F: Fn(u8) -> bool,
    {
        if self.at_eof() {
            return EOF;
        }

        let mut len = 0usize;
        while len < width {
            match self.peek() {
                Some(c) if accepts(c) => {
                    out[len] = c;
                    len += 1;
                    self.bump();
                }
                _ => break,
            }
        }

        if len == 0 {
            return 0;
        }
        out[len] = 0;
        1
    }

    /// `scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)`
    fn scan_ids(&mut self, luggage_id: &mut [u8], flight_id: &mut [u8]) -> i32 {
        let mut assigned = 0;

        match self.scan_set(LUGGAGE_ID_LENGTH, is_upper_alnum, luggage_id) {
            EOF => return EOF,
            0 => return assigned,
            _ => assigned += 1,
        }
        self.skip_whitespace();

        match self.scan_set(FLIGHT_ID_LENGTH, is_upper_alnum, flight_id) {
            EOF | 0 => return assigned,
            _ => assigned += 1,
        }
        self.skip_whitespace();

        assigned
    }

    /// `scanf("%3[A-Z] %3[A-Z]", departure, arrival)`
    fn scan_airports(&mut self, departure: &mut [u8], arrival: &mut [u8]) -> i32 {
        let mut assigned = 0;

        match self.scan_set(AIRPORT_CODE_LENGTH, is_upper, departure) {
            EOF => return EOF,
            0 => return assigned,
            _ => assigned += 1,
        }
        self.skip_whitespace();

        match self.scan_set(AIRPORT_CODE_LENGTH, is_upper, arrival) {
            EOF | 0 => assigned,
            _ => assigned + 1,
        }
    }

    /// `scanf("%80[^\n]", comments)`
    fn scan_comments(&mut self, comments: &mut [u8]) -> i32 {
        self.scan_set(COMMENTS_LENGTH, |c| c != b'\n', comments)
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

fn is_upper_alnum(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

/// `strcpy` source semantics: everything up to (excluding) the first NUL.
fn c_str(buffer: &[u8]) -> Vec<u8> {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    buffer[..end].to_vec()
}

/// Insertion point used by `addRoutingDirectiveToList`: before the first
/// directive with a strictly greater time stamp, so equal time stamps keep
/// their input order.
fn insertion_index(list: &[RoutingDirective], time_stamp: u32) -> usize {
    list.iter()
        .position(|directive| directive.time_stamp > time_stamp)
        .unwrap_or(list.len())
}

/// `superseded`: consult only the first later directive with the same luggage
/// id; the directive is superseded when that one departs from the same airport.
fn superseded(list: &[RoutingDirective], index: usize) -> bool {
    let current = &list[index];
    for later in &list[index + 1..] {
        if later.luggage_id != current.luggage_id {
            continue;
        }
        return later.departure == current.departure;
    }
    false
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

fn print_matching_directives<W: Write>(
    out: &mut W,
    list: &[RoutingDirective],
    expected_luggage_id: &[u8],
    expected_flight_id: &[u8],
    expected_departure: &[u8],
    expected_arrival: &[u8],
) -> io::Result<()> {
    for (index, directive) in list.iter().enumerate() {
        if !superseded(list, index)
            && matches(expected_luggage_id, &directive.luggage_id)
            && matches(expected_flight_id, &directive.flight_id)
            && matches(expected_departure, &directive.departure)
            && matches(expected_arrival, &directive.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
            write!(out, "{:010}", directive.time_stamp)?;
            for field in [
                &directive.luggage_id,
                &directive.flight_id,
                &directive.departure,
                &directive.arrival,
                &directive.comments,
            ] {
                out.write_all(b" ")?;
                out.write_all(field)?;
            }
            out.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|arg| arg.as_os_str().as_bytes().to_vec())
        .collect();

    if argv.len() != 5 {
        let mut stderr = io::stderr();
        let _ = stderr.write_all(b"Command line error: 4 arguments expected\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    let mut input = Vec::new();
    // A read error leaves the buffer with whatever arrived, mirroring stdio.
    let _ = io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);

    let mut list: Vec<RoutingDirective> = Vec::new();

    // In the C source these buffers are declared inside the loop but live in
    // the same stack slots on every iteration, so contents carry over when a
    // conversion fails. Hoisting them reproduces that.
    let mut time_stamp: i32 = 0;
    let mut luggage_id = [0u8; LUGGAGE_ID_LENGTH + 1];
    let mut flight_id = [0u8; FLIGHT_ID_LENGTH + 1];
    let mut departure = [0u8; AIRPORT_CODE_LENGTH + 1];
    let mut arrival = [0u8; AIRPORT_CODE_LENGTH + 1];
    let mut comments = [0u8; COMMENTS_LENGTH + 1];

    loop {
        comments[0] = 0; // comments are optional.

        if scanner.scan_int_then_space(&mut time_stamp) == EOF {
            break;
        }
        if scanner.scan_ids(&mut luggage_id, &mut flight_id) == EOF {
            break;
        }
        if scanner.scan_airports(&mut departure, &mut arrival) == EOF {
            break;
        }
        if scanner.scan_comments(&mut comments) == EOF {
            break;
        }

        let new_directive = RoutingDirective {
            time_stamp: time_stamp as u32,
            luggage_id: c_str(&luggage_id),
            flight_id: c_str(&flight_id),
            departure: c_str(&departure),
            arrival: c_str(&arrival),
            comments: c_str(&comments),
        };

        let index = insertion_index(&list, new_directive.time_stamp);
        list.insert(index, new_directive);
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let result = print_matching_directives(
        &mut out, &list, &argv[1], &argv[2], &argv[3], &argv[4],
    )
    .and_then(|()| out.flush());
    drop(result);

    std::process::exit(0);
}
