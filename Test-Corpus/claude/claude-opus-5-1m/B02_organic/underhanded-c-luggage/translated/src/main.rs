// Rust translation of c_src/src/luggage.c
// Original C by Jan Wrobel <wrr@mixedbit.org>
//
// The translation reproduces the observable behaviour of the C program
// byte-for-byte, including the quirks of the original `scanf()` usage
// (buffers that keep stale values when a conversion fails, records that are
// silently dropped when EOF is reached in the middle of a line, `%d` being
// used for an `unsigned int`, ...).

use std::io::{self, Read, Write};

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// Emulates `strcpy()` from a scanf destination buffer: the C buffers are NUL
/// terminated character arrays, so copying stops at the first NUL byte.
fn c_str(bytes: &[u8]) -> Vec<u8> {
    match bytes.iter().position(|&b| b == 0) {
        Some(i) => bytes[..i].to_vec(),
        None => bytes.to_vec(),
    }
}

/// Emulates `strcmp(a, b) == 0` for NUL terminated strings.
fn c_str_eq(a: &[u8], b: &[u8]) -> bool {
    c_str(a) == c_str(b)
}

// ---------------------------------------------------------------------------
// scanf() emulation
// ---------------------------------------------------------------------------

/// Result of a single conversion directive.
enum Conv {
    /// The conversion succeeded and a value was assigned.
    Assigned,
    /// Matching failure: nothing was assigned, the destination is untouched.
    MatchFail,
    /// Input failure: end of input was reached before anything was matched.
    InputFail,
}

/// C's `EOF` value as returned by `scanf()`.
const EOF: i32 = -1;

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Scanner { data, pos: 0 }
    }

    /// `getc()`
    fn getc(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            // Keep pos at end; EOF is sticky for our purposes.
            None
        }
    }

    /// `ungetc()` of the character that was just read.
    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// A whitespace directive in the format string: consumes any amount of
    /// whitespace (possibly none) and never fails, not even at end of input.
    fn skip_ws_directive(&mut self) {
        loop {
            match self.getc() {
                None => return,
                Some(c) => {
                    if !is_c_space(c) {
                        self.ungetc();
                        return;
                    }
                }
            }
        }
    }

    /// `%d` into an `unsigned int` (as the original C code does).
    fn scan_d(&mut self, out: &mut u32) -> Conv {
        // Leading whitespace is skipped as part of the conversion; reaching
        // end of input while doing so is an input failure.
        let mut c = loop {
            match self.getc() {
                None => return Conv::InputFail,
                Some(c) => {
                    if !is_c_space(c) {
                        break c;
                    }
                }
            }
        };

        let mut negative = false;
        if c == b'-' || c == b'+' {
            negative = c == b'-';
            match self.getc() {
                None => {
                    // Only a sign was read: no number, matching failure.
                    return Conv::MatchFail;
                }
                Some(next) => c = next,
            }
        }

        // Accumulate the digits the way glibc does: the value is computed as a
        // `long int` (saturating on overflow) and then truncated to `int`.
        let mut digits = 0usize;
        let mut mag: u128 = 0;
        let mut overflow = false;
        loop {
            if !c.is_ascii_digit() {
                self.ungetc();
                break;
            }
            digits += 1;
            if !overflow {
                mag = mag * 10 + u128::from(c - b'0');
                if mag > (i64::MAX as u128) + 1 {
                    overflow = true;
                }
            }
            match self.getc() {
                None => break,
                Some(next) => c = next,
            }
        }

        if digits == 0 {
            return Conv::MatchFail;
        }

        let as_long: i64 = if negative {
            if overflow || mag > (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                (-(mag as i128)) as i64
            }
        } else if overflow || mag > i64::MAX as u128 {
            i64::MAX
        } else {
            mag as i64
        };

        // `long` -> `int` -> stored through an `unsigned int *`.
        *out = as_long as u32;
        Conv::Assigned
    }

    /// `%<width>[...]` scan set conversion. `in_set` decides membership.
    fn scan_set<F>(&mut self, width: usize, in_set: F, out: &mut Vec<u8>) -> Conv
    where
        F: Fn(u8) -> bool,
    {
        // No leading whitespace is skipped for scan sets.
        let mut c = match self.getc() {
            None => return Conv::InputFail,
            Some(c) => c,
        };

        let mut collected: Vec<u8> = Vec::new();
        loop {
            if !in_set(c) {
                self.ungetc();
                break;
            }
            collected.push(c);
            if collected.len() == width {
                // Width reached: no further character is consumed.
                break;
            }
            match self.getc() {
                None => break,
                Some(next) => c = next,
            }
        }

        if collected.is_empty() {
            // Matching failure: the destination buffer is left untouched.
            return Conv::MatchFail;
        }

        *out = collected;
        Conv::Assigned
    }
}

/// `isspace()` in the "C" locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_upper_alnum(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

// scanf("%d ", &time_stamp)
fn scanf_time_stamp(sc: &mut Scanner, time_stamp: &mut u32) -> i32 {
    match sc.scan_d(time_stamp) {
        Conv::InputFail => return EOF,
        Conv::MatchFail => return 0,
        Conv::Assigned => {}
    }
    sc.skip_ws_directive();
    1
}

// scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
fn scanf_ids(sc: &mut Scanner, luggage_id: &mut Vec<u8>, flight_id: &mut Vec<u8>) -> i32 {
    let mut assigned = 0;
    match sc.scan_set(LUGGAGE_ID_LENGTH, is_upper_alnum, luggage_id) {
        Conv::InputFail => return EOF,
        Conv::MatchFail => return assigned,
        Conv::Assigned => assigned = 1,
    }
    sc.skip_ws_directive();
    match sc.scan_set(FLIGHT_ID_LENGTH, is_upper_alnum, flight_id) {
        Conv::InputFail => return assigned,
        Conv::MatchFail => return assigned,
        Conv::Assigned => assigned = 2,
    }
    sc.skip_ws_directive();
    assigned
}

// scanf("%3[A-Z] %3[A-Z]", departure, arrival)
fn scanf_airports(sc: &mut Scanner, departure: &mut Vec<u8>, arrival: &mut Vec<u8>) -> i32 {
    let mut assigned = 0;
    match sc.scan_set(AIRPORT_CODE_LENGTH, is_upper, departure) {
        Conv::InputFail => return EOF,
        Conv::MatchFail => return assigned,
        Conv::Assigned => assigned = 1,
    }
    sc.skip_ws_directive();
    match sc.scan_set(AIRPORT_CODE_LENGTH, is_upper, arrival) {
        Conv::InputFail => return assigned,
        Conv::MatchFail => return assigned,
        Conv::Assigned => assigned = 2,
    }
    assigned
}

// scanf("%80[^\n]", comments)
fn scanf_comments(sc: &mut Scanner, comments: &mut Vec<u8>) -> i32 {
    match sc.scan_set(COMMENTS_LENGTH, |c| c != b'\n', comments) {
        Conv::InputFail => EOF,
        Conv::MatchFail => 0,
        Conv::Assigned => 1,
    }
}

// ---------------------------------------------------------------------------
// Routing directive list
// ---------------------------------------------------------------------------

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
    next_directive: Option<usize>,
}

/// The linked list is modelled with an arena; index 0 is the list head that the
/// C code keeps on the stack.
struct Arena {
    nodes: Vec<RoutingDirective>,
}

impl Arena {
    fn new() -> Self {
        Arena {
            nodes: vec![RoutingDirective {
                time_stamp: 0,
                luggage_id: Vec::new(),
                flight_id: Vec::new(),
                departure: Vec::new(),
                arrival: Vec::new(),
                comments: Vec::new(),
                next_directive: None,
            }],
        }
    }
}

fn add_routing_directive_to_list(arena: &mut Arena, previous: usize, new_directive: usize) {
    let mut previous_directive = previous;
    loop {
        let next_directive = arena.nodes[previous_directive].next_directive;
        let insert_here = match next_directive {
            None => true,
            Some(n) => arena.nodes[n].time_stamp > arena.nodes[new_directive].time_stamp,
        };
        if insert_here {
            arena.nodes[new_directive].next_directive = next_directive;
            arena.nodes[previous_directive].next_directive = Some(new_directive);
            return;
        }
        previous_directive = next_directive.unwrap();
    }
}

fn supersedes(arena: &Arena, directive: Option<usize>, luggage_id: &[u8], departure: &[u8]) -> bool {
    let mut current = directive;
    loop {
        let idx = match current {
            None => return false,
            Some(i) => i,
        };
        if !c_str_eq(&arena.nodes[idx].luggage_id, luggage_id) {
            current = arena.nodes[idx].next_directive;
            continue;
        }
        return c_str_eq(&arena.nodes[idx].departure, departure);
    }
}

fn superseded(arena: &Arena, directive: usize) -> bool {
    let luggage_id = arena.nodes[directive].luggage_id.clone();
    let departure = arena.nodes[directive].departure.clone();
    supersedes(
        arena,
        arena.nodes[directive].next_directive,
        &luggage_id,
        &departure,
    )
}

fn matches(expected: &[u8], actual: &[u8]) -> bool {
    // `expected[0] == '-'` in C reads the first byte of a NUL terminated
    // string, which is '\0' for an empty argument.
    let first = expected.first().copied().unwrap_or(0);
    first == b'-' || c_str_eq(expected, actual)
}

fn print_matching_directives(
    out: &mut impl Write,
    arena: &Arena,
    first_directive: Option<usize>,
    expected_luggage_id: &[u8],
    expected_flight_id: &[u8],
    expected_departure: &[u8],
    expected_arrival: &[u8],
) {
    let mut directive = first_directive;
    while let Some(idx) = directive {
        let node = &arena.nodes[idx];
        if !superseded(arena, idx)
            && matches(expected_luggage_id, &node.luggage_id)
            && matches(expected_flight_id, &node.flight_id)
            && matches(expected_departure, &node.departure)
            && matches(expected_arrival, &node.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
            let mut line: Vec<u8> = Vec::new();
            line.extend_from_slice(format!("{:010}", node.time_stamp).as_bytes());
            for field in [
                &node.luggage_id,
                &node.flight_id,
                &node.departure,
                &node.arrival,
                &node.comments,
            ] {
                line.push(b' ');
                line.extend_from_slice(&c_str(field));
            }
            line.push(b'\n');
            let _ = out.write_all(&line);
        }
        directive = arena.nodes[idx].next_directive;
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().as_bytes().to_vec()
    }
}

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if argc != 5 {
        let stderr = io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(b"Command line error: 4 arguments expected\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    let mut input = Vec::new();
    let _ = io::stdin().lock().read_to_end(&mut input);
    let mut sc = Scanner::new(input);

    let mut arena = Arena::new();
    let directive_list_head = 0usize;

    // The C code declares these inside the loop body; on a matching failure
    // scanf() leaves them untouched, so they keep whatever the previous
    // iteration left in that stack slot (zeroed on the first iteration).
    let mut time_stamp: u32 = 0;
    let mut luggage_id: Vec<u8> = Vec::new();
    let mut flight_id: Vec<u8> = Vec::new();
    let mut departure: Vec<u8> = Vec::new();
    let mut arrival: Vec<u8> = Vec::new();
    let mut comments: Vec<u8>;

    loop {
        comments = Vec::new(); // comments[0] = 0; comments are optional.

        if scanf_time_stamp(&mut sc, &mut time_stamp) == EOF {
            break;
        }
        if scanf_ids(&mut sc, &mut luggage_id, &mut flight_id) == EOF {
            break;
        }
        if scanf_airports(&mut sc, &mut departure, &mut arrival) == EOF {
            break;
        }
        if scanf_comments(&mut sc, &mut comments) == EOF {
            break;
        }

        arena.nodes.push(RoutingDirective {
            time_stamp,
            luggage_id: c_str(&luggage_id),
            flight_id: c_str(&flight_id),
            departure: c_str(&departure),
            arrival: c_str(&arrival),
            comments: c_str(&comments),
            next_directive: None,
        });
        let new_directive = arena.nodes.len() - 1;

        add_routing_directive_to_list(&mut arena, directive_list_head, new_directive);
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    print_matching_directives(
        &mut out,
        &arena,
        arena.nodes[directive_list_head].next_directive,
        &arg_bytes(&argv[1]),
        &arg_bytes(&argv[2]),
        &arg_bytes(&argv[3]),
        &arg_bytes(&argv[4]),
    );
    let _ = out.flush();
    std::process::exit(0);
}
