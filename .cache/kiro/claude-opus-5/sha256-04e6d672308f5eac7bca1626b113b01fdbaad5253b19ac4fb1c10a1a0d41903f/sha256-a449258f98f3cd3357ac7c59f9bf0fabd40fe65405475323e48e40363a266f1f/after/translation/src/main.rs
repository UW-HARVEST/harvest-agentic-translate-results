// Rust translation of c_src/src/luggage.c (original C by Jan Wrobel <wrr@mixedbit.org>).
//
// The goal is byte-identical behaviour with the C program, including its quirks:
//   * `scanf` semantics (whitespace skipping, field widths, EOF vs. matching
//     failure, partial assignment) are emulated faithfully.
//   * Only `== EOF` is treated as a reason to stop reading; a matching failure
//     leaves the destination buffers untouched, exactly like the C code where
//     the loop-local stack buffers keep whatever the previous iteration left in
//     them.
//   * The odd `supersedes()` logic (it stops at the *first* later directive with
//     a matching luggage id) is preserved as-is.
//   * `%80[^\n]` does not skip whitespace, so comments keep their leading blank,
//     which yields a double space in the output. That is reproduced.

use std::io::{self, BufRead, Write};

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

/// Contents of main's uninitialised `luggage_id[9]` stack slot in the reference
/// `gcc -O0` build of `c_src/src/luggage.c`. See the comment in `main()`.
const UNINIT_LUGGAGE_ID: &[u8] = b"\x03";

/// One parsed routing directive. The C version stored NUL-terminated char
/// arrays; here the strings are byte vectors holding the same contents.
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

// ---------------------------------------------------------------------------
// scanf emulation
// ---------------------------------------------------------------------------

/// Reason a conversion did not produce a value.
#[derive(PartialEq, Eq, Debug)]
enum ScanErr {
    /// Input failure: end of input before any character of the item was read.
    Eof,
    /// Matching failure: characters were available but did not match.
    Fail,
}

/// `isspace()` in the C locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Byte-oriented input with a one-character lookahead, mirroring the way
/// `scanf` consumes stdin (lazily, pushing back at most one character).
struct Input<R: BufRead> {
    reader: R,
}

impl<R: BufRead> Input<R> {
    fn new(reader: R) -> Self {
        Input { reader }
    }

    /// Look at the next byte without consuming it. `None` means end of input
    /// (read errors are treated like EOF, as stdio would report them here).
    fn peek(&mut self) -> Option<u8> {
        loop {
            let res = self.reader.fill_buf();
            match res {
                Ok(buf) => return buf.first().copied(),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn bump(&mut self) {
        self.reader.consume(1);
    }

    /// A whitespace directive in a format string: consume zero or more
    /// whitespace characters. Never fails.
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if is_space(c) {
                self.bump();
            } else {
                break;
            }
        }
    }
}

/// `%d` into an `unsigned int*` (as the C code does). Leading whitespace is
/// skipped, an optional sign is accepted, then decimal digits. Out-of-range
/// values clamp like `strtol` and are then truncated to `int`, matching glibc
/// on a 64-bit platform.
fn scan_int<R: BufRead>(inp: &mut Input<R>) -> Result<u32, ScanErr> {
    inp.skip_ws();
    let first = match inp.peek() {
        None => return Err(ScanErr::Eof),
        Some(c) => c,
    };

    let mut negative = false;
    if first == b'+' || first == b'-' {
        negative = first == b'-';
        inp.bump();
    }

    let mut digits = 0usize;
    let mut acc: u128 = 0;
    let mut overflow = false;
    while let Some(c) = inp.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        inp.bump();
        digits += 1;
        if !overflow {
            acc = acc * 10 + u128::from(c - b'0');
            if acc > (i64::MAX as u128) + 1 {
                overflow = true;
            }
        }
    }

    if digits == 0 {
        // Matching failure. The offending character stays in the stream; the
        // sign, if any, has already been consumed (same as glibc).
        return Err(ScanErr::Fail);
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        if acc > i64::MAX as u128 {
            i64::MIN
        } else {
            -(acc as i64)
        }
    } else if acc > i64::MAX as u128 {
        i64::MAX
    } else {
        acc as i64
    };

    Ok(value as i32 as u32)
}

/// `%<width>[set]`. No leading whitespace is skipped. At least one character
/// must match, otherwise the destination is left untouched.
fn scan_set<R: BufRead, F: Fn(u8) -> bool>(
    inp: &mut Input<R>,
    width: usize,
    in_set: F,
    out: &mut Vec<u8>,
) -> Result<(), ScanErr> {
    let mut collected: Vec<u8> = Vec::with_capacity(width);
    while collected.len() < width {
        match inp.peek() {
            None => break,
            Some(c) => {
                if in_set(c) {
                    inp.bump();
                    collected.push(c);
                } else {
                    break;
                }
            }
        }
    }

    if collected.is_empty() {
        // No character was stored: end of input is an input failure, anything
        // else is a matching failure.
        return if inp.peek().is_none() {
            Err(ScanErr::Eof)
        } else {
            Err(ScanErr::Fail)
        };
    }

    out.clear();
    out.extend_from_slice(&collected);
    Ok(())
}

fn is_alnum_upper(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

fn is_not_newline(c: u8) -> bool {
    c != b'\n'
}

/// `strcpy(dst, src)` semantics: the copy stops at the first NUL byte.
///
/// This matters for the `comments` field, whose scanset is `[^\n]` and therefore
/// happily stores a NUL byte read from stdin. `scanf` counts that byte against
/// the field width (so the stream position depends on the full run of matched
/// bytes), but the subsequent `strcpy` into the heap-allocated directive only
/// copies up to the first NUL. Everything downstream (`strcmp`, `printf("%s")`)
/// then sees the truncated string.
fn strcpy_bytes(src: &[u8]) -> Vec<u8> {
    let end = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    src[..end].to_vec()
}

// ---------------------------------------------------------------------------
// Directive list
// ---------------------------------------------------------------------------

/// Equivalent of `addRoutingDirectiveToList`: insert before the first directive
/// with a strictly greater time stamp, so equal time stamps keep insertion
/// order.
fn add_routing_directive_to_list(list: &mut Vec<RoutingDirective>, new_directive: RoutingDirective) {
    let pos = list
        .iter()
        .position(|d| d.time_stamp > new_directive.time_stamp)
        .unwrap_or(list.len());
    list.insert(pos, new_directive);
}

/// Equivalent of `supersedes`: walk forward to the first directive with the
/// same luggage id and report whether its departure matches. The search stops
/// there even when it does not match.
fn supersedes(list: &[RoutingDirective], start: usize, luggage_id: &[u8], departure: &[u8]) -> bool {
    for directive in &list[start..] {
        if directive.luggage_id != luggage_id {
            continue;
        }
        return directive.departure == departure;
    }
    false
}

/// Equivalent of `superseded`: start the search after the given directive.
fn superseded(list: &[RoutingDirective], index: usize) -> bool {
    let directive = &list[index];
    supersedes(
        list,
        index + 1,
        &directive.luggage_id,
        &directive.departure,
    )
}

/// Equivalent of `matches`: a leading '-' in the expected value is a wildcard.
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

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which the C
/// program does not do: a C `printf` to a pipe whose reader has gone away kills
/// the process with `SIGPIPE` (wait status 141 from a shell), whereas Rust would
/// see `EPIPE`, ignore it and exit 0. Restoring the default disposition keeps
/// the exit status identical when stdout is a closed pipe.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

fn main() {
    restore_default_sigpipe();

    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if argv.len() != 5 {
        let mut stderr = io::stderr();
        let _ = stderr.write_all(b"Command line error: 4 arguments expected\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let mut inp = Input::new(stdin.lock());

    let mut directive_list: Vec<RoutingDirective> = Vec::new();

    // The C code declares these inside the loop without initialising them, so a
    // failed conversion leaves the previous iteration's bytes in place. Keeping
    // them outside the loop reproduces that; `comments` is explicitly cleared
    // every iteration just like `comments[0] = 0`.
    //
    // On the *first* iteration those buffers still hold whatever the stack
    // happened to contain, which the C standard leaves undefined. The reference
    // build (`c_src/CMakeLists.txt`, no CMAKE_BUILD_TYPE, i.e. `gcc -O0`)
    // reproducibly leaves `time_stamp == 0`, `luggage_id == "\x03"` and
    // `flight_id`/`departure`/`arrival` empty in main's frame -- leftovers from
    // libc start-up code that ran in the same stack region. Those values are
    // observable whenever a conversion fails during the first iteration (e.g.
    // stdin starting with a lowercase word), so they are reproduced here to keep
    // the output byte-identical. See translation/ERRORS.md.
    let mut time_stamp: u32 = 0;
    let mut luggage_id: Vec<u8> = UNINIT_LUGGAGE_ID.to_vec();
    let mut flight_id: Vec<u8> = Vec::with_capacity(FLIGHT_ID_LENGTH + 1);
    let mut departure: Vec<u8> = Vec::with_capacity(AIRPORT_CODE_LENGTH + 1);
    let mut arrival: Vec<u8> = Vec::with_capacity(AIRPORT_CODE_LENGTH + 1);
    let mut comments: Vec<u8> = Vec::with_capacity(COMMENTS_LENGTH + 1);

    loop {
        comments.clear(); // comments are optional.

        // scanf("%d ", &time_stamp)
        match scan_int(&mut inp) {
            Err(ScanErr::Eof) => break,
            Err(ScanErr::Fail) => {}
            Ok(value) => {
                time_stamp = value;
                inp.skip_ws();
            }
        }

        // scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)
        match scan_set(&mut inp, LUGGAGE_ID_LENGTH, is_alnum_upper, &mut luggage_id) {
            // Input failure with no assignment yet: scanf returns EOF.
            Err(ScanErr::Eof) => break,
            Err(ScanErr::Fail) => {}
            Ok(()) => {
                inp.skip_ws();
                // A failure here still leaves one assignment done, so scanf
                // returns 1 and the C code does not break.
                if scan_set(&mut inp, FLIGHT_ID_LENGTH, is_alnum_upper, &mut flight_id).is_ok() {
                    inp.skip_ws();
                }
            }
        }

        // scanf("%3[A-Z] %3[A-Z]", departure, arrival)
        match scan_set(&mut inp, AIRPORT_CODE_LENGTH, is_upper, &mut departure) {
            Err(ScanErr::Eof) => break,
            Err(ScanErr::Fail) => {}
            Ok(()) => {
                inp.skip_ws();
                let _ = scan_set(&mut inp, AIRPORT_CODE_LENGTH, is_upper, &mut arrival);
            }
        }

        // scanf("%80[^\n]", comments)
        match scan_set(&mut inp, COMMENTS_LENGTH, is_not_newline, &mut comments) {
            Err(ScanErr::Eof) => break,
            Err(ScanErr::Fail) => {}
            Ok(()) => {}
        }

        // The five strcpy() calls truncate at the first NUL byte.
        let new_directive = RoutingDirective {
            time_stamp,
            luggage_id: strcpy_bytes(&luggage_id),
            flight_id: strcpy_bytes(&flight_id),
            departure: strcpy_bytes(&departure),
            arrival: strcpy_bytes(&arrival),
            comments: strcpy_bytes(&comments),
        };
        add_routing_directive_to_list(&mut directive_list, new_directive);
    }

    let expected_luggage_id = arg_bytes(&argv[1]);
    let expected_flight_id = arg_bytes(&argv[2]);
    let expected_departure = arg_bytes(&argv[3]);
    let expected_arrival = arg_bytes(&argv[4]);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let _ = print_matching_directives(
        &mut out,
        &directive_list,
        &expected_luggage_id,
        &expected_flight_id,
        &expected_departure,
        &expected_arrival,
    );
    let _ = out.flush();
    drop(out);
    std::process::exit(0);
}
