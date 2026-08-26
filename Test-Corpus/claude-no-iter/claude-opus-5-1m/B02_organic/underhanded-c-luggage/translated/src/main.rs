// Translation of c_src/src/luggage.c by Jan Wrobel <wrr@mixedbit.org>

use std::env;
use std::io::{self, BufWriter, Read, Write};
use std::process::ExitCode;

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

/// Result of an attempted scanf-style conversion.
enum ScanResult<T> {
    /// Conversion succeeded.
    Ok(T),
    /// Matching failure: the input did not match the conversion specifier
    /// but at least some bytes are still available. (C's scanf returns 0
    /// for the failed item.)
    Failure,
    /// Input failure / EOF before any conversion could happen. (C's scanf
    /// returns EOF in this case.)
    Eof,
}

/// Minimal scanf-style parser over a byte buffer. Tracks position so we can
/// distinguish "EOF before any conversion" (C scanf returns EOF) from
/// "matching failure with bytes still available" (C scanf returns 0).
struct Parser {
    data: Vec<u8>,
    pos: usize,
}

impl Parser {
    fn new(data: Vec<u8>) -> Self {
        Parser { data, pos: 0 }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Mirror C `isspace` for the "C" locale.
    fn is_ws(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    }

    /// Skip any whitespace; returns true if at EOF afterward.
    fn skip_ws_check_eof(&mut self) -> bool {
        while let Some(c) = self.peek() {
            if Self::is_ws(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.at_eof()
    }

    /// Implements `%d` for scanf. Skips leading whitespace.
    fn scan_int(&mut self) -> ScanResult<u32> {
        // %d performs an internal whitespace skip.
        if self.skip_ws_check_eof() {
            return ScanResult::Eof;
        }
        let start = self.pos;
        if let Some(c) = self.peek() {
            if c == b'+' || c == b'-' {
                self.pos += 1;
            }
        }
        let digits_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == digits_start {
            // No digits matched: scanf reports a matching failure (return 0)
            // and leaves the input position at the offending character.
            self.pos = start;
            return ScanResult::Failure;
        }
        let s = std::str::from_utf8(&self.data[start..self.pos]).unwrap();
        // C `%d` is signed, but the destination is `unsigned int`. Parse as
        // i64 (covers all 32-bit signed values) and reinterpret as u32 to
        // mirror typical C platform behavior used by the original program.
        match s.parse::<i64>() {
            Ok(v) => ScanResult::Ok(v as u32),
            Err(_) => ScanResult::Failure,
        }
    }

    /// Implements `%N[set]` for scanf. Does NOT skip leading whitespace.
    /// Returns Eof only if we are already at EOF before reading anything;
    /// otherwise returns Failure on matching failure or Ok with the matched
    /// bytes.
    fn scan_charset<F: Fn(u8) -> bool>(&mut self, max: usize, pred: F) -> ScanResult<String> {
        if self.at_eof() {
            return ScanResult::Eof;
        }
        let mut result = Vec::new();
        while result.len() < max {
            match self.peek() {
                Some(c) if pred(c) => {
                    result.push(c);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        if result.is_empty() {
            return ScanResult::Failure;
        }
        // The predicates we use only accept ASCII bytes.
        ScanResult::Ok(String::from_utf8(result).unwrap())
    }
}

fn is_alnum_upper(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

/// Equivalent of `supersedes` in the C source: walks the (already sorted)
/// remaining directives, skipping ones with a different luggage_id, and
/// returns true if the FIRST directive with the same luggage_id has the
/// same departure airport.
fn supersedes(later: &[RoutingDirective], luggage_id: &str, departure: &str) -> bool {
    for d in later {
        if d.luggage_id != luggage_id {
            continue;
        }
        return d.departure == departure;
    }
    false
}

/// Equivalent of `matches`: a leading `-` in the expected argv argument is
/// treated as a wildcard.
fn matches_arg(expected: &str, actual: &str) -> bool {
    expected.as_bytes().first() == Some(&b'-') || expected == actual
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        // Match the C program's stderr message exactly.
        let stderr = io::stderr();
        let mut err = stderr.lock();
        let _ = err.write_all(b"Command line error: 4 arguments expected\n");
        return ExitCode::from(1);
    }

    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        // Treat read error like EOF; the original C program would simply
        // stop reading on error too.
    }

    let mut parser = Parser::new(input);
    let mut directives: Vec<RoutingDirective> = Vec::new();

    // The C program declares these as local variables inside the loop, so
    // they are technically uninitialized at the start of each iteration. On
    // matching failure, scanf leaves the variable unchanged, which means it
    // reads the previous iteration's value out of the same stack slot. To
    // mirror that behavior, we keep them across iterations here. For the
    // first iteration the C values are truly undefined; we use zero / empty
    // string, which is what a freshly-mapped stack page typically holds.
    let mut time_stamp: u32 = 0;
    let mut luggage_id: String = String::new();
    let mut flight_id: String = String::new();
    let mut departure: String = String::new();
    let mut arrival: String = String::new();

    'outer: loop {
        // The C source explicitly clears `comments[0] = 0;` each iteration
        // because comments are optional.
        let mut comments = String::new();

        // scanf("%d ", &time_stamp). The C program only breaks on EOF (the
        // return value `EOF`); a matching failure (`0`) is silently ignored
        // and leaves `time_stamp` with its previous value.
        match parser.scan_int() {
            ScanResult::Ok(v) => {
                time_stamp = v;
                // The trailing space in `"%d "` consumes any whitespace.
                parser.skip_ws_check_eof();
            }
            ScanResult::Failure => {
                // Leave `time_stamp` unchanged, mirroring scanf returning 0.
            }
            ScanResult::Eof => break 'outer,
        }

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        match parser.scan_charset(LUGGAGE_ID_LENGTH, is_alnum_upper) {
            ScanResult::Ok(s) => {
                luggage_id = s;
            }
            ScanResult::Failure => {
                // scanf returns 0 here without writing to luggage_id.
                // C also stops processing further conversions in the same
                // call and the trailing space directive is not reached.
            }
            ScanResult::Eof => break 'outer,
        }
        // The space directive between %8[..] and %6[..] always runs as long
        // as the first conversion succeeded -- if it failed, scanf already
        // returned. We approximate by skipping whitespace unconditionally;
        // if the first conversion failed we typically aren't on whitespace
        // anyway.
        parser.skip_ws_check_eof();
        match parser.scan_charset(FLIGHT_ID_LENGTH, is_alnum_upper) {
            ScanResult::Ok(s) => {
                flight_id = s;
            }
            ScanResult::Failure => {}
            ScanResult::Eof => break 'outer,
        }
        // Trailing space in the format consumes whitespace after flight_id.
        parser.skip_ws_check_eof();

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)  -- no trailing space
        match parser.scan_charset(AIRPORT_CODE_LENGTH, is_upper) {
            ScanResult::Ok(s) => {
                departure = s;
            }
            ScanResult::Failure => {}
            ScanResult::Eof => break 'outer,
        }
        parser.skip_ws_check_eof();
        match parser.scan_charset(AIRPORT_CODE_LENGTH, is_upper) {
            ScanResult::Ok(s) => {
                arrival = s;
            }
            ScanResult::Failure => {}
            ScanResult::Eof => break 'outer,
        }
        // No whitespace skip here -- the C format string ends right after
        // the second %3[A-Z].

        // scanf("%80[^\n]", comments)
        match parser.scan_charset(COMMENTS_LENGTH, |c| c != b'\n') {
            ScanResult::Ok(s) => {
                comments = s;
            }
            ScanResult::Failure => {
                // Comments stay empty (the C code already initialized
                // comments[0] = 0 at the top of the iteration).
            }
            ScanResult::Eof => break 'outer,
        }

        directives.push(RoutingDirective {
            time_stamp,
            luggage_id: luggage_id.clone(),
            flight_id: flight_id.clone(),
            departure: departure.clone(),
            arrival: arrival.clone(),
            comments,
        });
    }

    // The C code performs a stable insertion sort by `time_stamp`: a new
    // directive is inserted *after* all existing directives with a smaller
    // or equal time_stamp, preserving insertion order for ties. Rust's
    // `sort_by_key` is stable, so it produces the same ordering.
    directives.sort_by_key(|d| d.time_stamp);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for i in 0..directives.len() {
        let d = &directives[i];
        let later = &directives[i + 1..];
        if !supersedes(later, &d.luggage_id, &d.departure)
            && matches_arg(&args[1], &d.luggage_id)
            && matches_arg(&args[2], &d.flight_id)
            && matches_arg(&args[3], &d.departure)
            && matches_arg(&args[4], &d.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
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

    let _ = out.flush();
    ExitCode::from(0)
}
