// Rust translation of c_src/src/main.c
//
// Behavior-preserving port of a CWE-129 style test driver
// (unvalidated array index used as a write offset).
//
// The original C is reproduced exactly, including its defects:
//   * `bad()` only checks `data >= 0` before `buffer[data] = 1`, so an index
//     >= 10 is an out-of-bounds stack write in C.  See `oob_write` below for
//     how that write's observable consequences are modelled.
//   * `goodG2B()` hard-codes data = 7 and never touches stdin, so exactly two
//     `fgets()` reads happen over the program's life: `goodB2G()` consumes the
//     first line, `bad()` the second.

use std::io::{self, BufRead, BufWriter, Write};

extern "C" {
    /// libc `signal(2)` and `raise(3)`.  Usable without an external crate
    /// because the `*-linux-gnu` targets already link libc.
    fn signal(sig: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
}

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

/// Terminate the way the C program does when its stack frame has been
/// corrupted: killed by SIGSEGV, with the stdio buffer never flushed.
///
/// Nothing is flushed on purpose.  The C program's stdout is fully buffered
/// when it is a pipe (which is how it is compared), the whole output is far
/// below one buffer, and the corrupted frame is only detected on `ret` -- so
/// `exit()` never runs and every byte written so far is discarded.
///
/// The default disposition has to be restored first: Rust's runtime installs a
/// SIGSEGV handler to turn stack overflow into a clean abort, and for a fault
/// outside the guard page that handler just returns, so a bare `raise` would
/// not terminate the process.
fn die_segv() -> ! {
    unsafe {
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
    }
    // Not reached: SIGSEGV's default disposition terminates the process.
    std::process::abort();
}

const BUFFER_LEN: usize = 10;
const INPUT_BUFFER_LEN: usize = 14;

/// `printf("%s\n", line)` -- the C version skips NULL, which cannot happen
/// here because every call site passes a string literal.
fn print_line(out: &mut impl Write, line: &str) {
    let _ = writeln!(out, "{}", line);
}

/// `printf("%d\n", intNumber)`
fn print_int_line(out: &mut impl Write, int_number: i32) {
    let _ = writeln!(out, "{}", int_number);
}

/// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping after a
/// newline (which is retained).  Returns `None` for the NULL case, i.e. EOF
/// with nothing read, or a read error.  Anything past the limit stays in the
/// stream for the next call, matching C's buffered stdin.
fn fgets<R: BufRead>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let max = size - 1;
    let mut out: Vec<u8> = Vec::with_capacity(max);

    while out.len() < max {
        let available = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        if available.is_empty() {
            // EOF
            return if out.is_empty() { None } else { Some(out) };
        }
        let take = available.len().min(max - out.len());
        match available[..take].iter().position(|&c| c == b'\n') {
            Some(pos) => {
                out.extend_from_slice(&available[..=pos]);
                reader.consume(pos + 1);
                return Some(out);
            }
            None => {
                out.extend_from_slice(&available[..take]);
                reader.consume(take);
            }
        }
    }
    Some(out)
}

/// `atoi()` on a NUL-terminated buffer: skip leading whitespace, accept an
/// optional sign, consume digits.  glibc implements it as `(int)strtol(...)`,
/// so out-of-range values saturate at the `long` boundary and are then
/// truncated to `int`.  Parsing stops at the first non-digit, NUL included.
fn atoi(bytes: &[u8]) -> i32 {
    let s: &[u8] = match bytes.iter().position(|&c| c == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    value as i32 // truncating conversion, as in (int)strtol(...)
}

/// Reads a line via `fgets(inputBuffer, 14, stdin)` and converts it with
/// `atoi()`; leaves `data` at -1 and reports failure when `fgets` returns NULL.
fn bad_source<R: BufRead>(out: &mut impl Write, reader: &mut R) -> i32 {
    let mut data: i32 = -1;
    match fgets(reader, INPUT_BUFFER_LEN) {
        Some(input_buffer) => {
            data = atoi(&input_buffer);
        }
        None => {
            print_line(out, "fgets() failed.");
        }
    }
    data
}

/// Address one past the end of the process's `[stack]` mapping, per
/// `/proc/self/maps`.  Writing at or above it faults, which is the mechanism
/// behind the C program's crashes on far out-of-bounds indices.
fn stack_mapping_top() -> Option<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if !line.ends_with("[stack]") {
            continue;
        }
        let range = line.split_whitespace().next()?;
        let end = range.split('-').nth(1)?;
        return usize::from_str_radix(end, 16).ok();
    }
    None
}

/// Models the observable effect of C's `buffer[data] = 1` in `bad()` when
/// `data` is out of bounds (`data >= 10`), returning `true` when the write
/// corrupts control flow badly enough that the process dies.
///
/// This is undefined behavior in C, so it is decided entirely by the frame
/// layout gcc emits at -O0 for this source (the layout the CMakeLists builds).
/// From `objdump -d` of `bad()` and `main()`:
///
/// ```text
///   bad():  sub $0x40,%rsp     buffer  @ rbp-0x40 (10 ints, rbp-0x40..rbp-0x19)
///                              <2 byte gap>       (rbp-0x18..rbp-0x17)
///                              inputBuffer[14] @ rbp-0x16
///                              i       @ rbp-0x8
///                              data    @ rbp-0x4
///                              saved rbp @ rbp+0x0
///                              return address @ rbp+0x8
///   main(): sub $0x10,%rsp  => main's rbp == bad's rbp + 0x20
///                              main's locals  @ rbp+0x10..rbp+0x1f (argc/argv)
///                              main's saved rbp @ rbp+0x20
///                              main's return address @ rbp+0x28
/// ```
///
/// `buffer[i]` sits at `bad_rbp - 0x40 + 4*i`, which gives:
///
/// | index   | what the 4-byte write lands on        | effect          |
/// |---------|---------------------------------------|-----------------|
/// | 0..=9   | `buffer` itself                       | in bounds       |
/// | 10..=13 | 2-byte gap + `inputBuffer`, both dead  | unobservable    |
/// | 14      | `i`, re-initialised to 0 by the loop   | unobservable    |
/// | 15      | `data`, never read again               | unobservable    |
/// | 16..=17 | `bad()`'s saved rbp                    | SIGSEGV         |
/// | 18..=19 | `bad()`'s return address               | SIGSEGV         |
/// | 20..=23 | `main()`'s argc/argv, both dead        | unobservable    |
/// | 24..=25 | `main()`'s saved rbp, not dereferenced | unobservable    |
/// | 26..=27 | `main()`'s return address              | SIGSEGV         |
/// | >= 28   | above `main()`'s frame -- see below     |                 |
///
/// Verified against the C binary: 16..=19 and 26..=27 exit 139 (SIGSEGV) on
/// every run; every other index below the stack top exits 0 and prints ten
/// zeros.
///
/// Past `main()`'s frame the write walks up through the argv/envp block towards
/// the top of the stack mapping and faults once it leaves it, so the first
/// faulting index scales with the size of that block: ~250 under an empty
/// environment, ~1200 under a normal one, past ~16000 with 32 KiB of padding.
/// `probe` is the address of the Rust `buffer`, standing in for C's, so
/// comparing `probe + 4*index` against the top of `[stack]` reproduces that
/// dependence instead of hard-coding a threshold, and makes large indices such
/// as `INT_MAX` fault as C's do.
///
/// Immediately around the boundary the C program is not reproducible by *any*
/// program: stack ASLR shifts the frame relative to the mapping, so a single
/// input alternates between exit 0 and exit 139 across runs of the same binary.
/// See ERRORS.md.
fn oob_write_is_fatal(index: usize, probe: usize) -> bool {
    if matches!(index, 16..=19 | 26..=27) {
        return true;
    }
    if index < 28 {
        return false;
    }
    match stack_mapping_top() {
        Some(top) => match index
            .checked_mul(4)
            .and_then(|offset| probe.checked_add(offset))
        {
            // The write is 4 bytes wide, so it faults as soon as any of it
            // lands at or above the top of the mapping.
            Some(target) => target.saturating_add(4) > top,
            // The address computation itself left the address space.
            None => true,
        },
        None => false,
    }
}

fn print_buffer(out: &mut impl Write, buffer: &[i32; BUFFER_LEN]) {
    // Indexed rather than iterated to mirror the C's `for(i = 0; i < 10; i++)`,
    // whose loop counter `i` is one of the locals the out-of-bounds write in
    // `bad()` can land on.
    #[allow(clippy::needless_range_loop)]
    for i in 0..BUFFER_LEN {
        print_int_line(out, buffer[i]);
    }
}

fn bad<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    let data = bad_source(out, reader);

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        // The missing upper-bound check is the defect being demonstrated.
        let index = data as usize;
        let fatal = if index < BUFFER_LEN {
            buffer[index] = 1;
            false
        } else {
            // Stands in for C's `buffer`, so the out-of-bounds target address
            // can be computed the way the C program's would be.
            let probe = std::hint::black_box(&buffer) as *const _ as usize;
            oob_write_is_fatal(index, probe)
        };

        // The write happens before the loop, so C prints all ten slots and only
        // then trips over the damaged frame on return.
        print_buffer(out, &buffer);

        if fatal {
            die_segv();
        }
    } else {
        print_line(out, "ERROR: Array index is negative.");
    }
}

/// goodG2B uses the GoodSource with the BadSink
fn good_g2b(out: &mut impl Write) {
    let data: i32 = 7;

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        // `data` is hard-coded to 7, so the unchecked sink is always in bounds.
        buffer[data as usize] = 1;
        print_buffer(out, &buffer);
    } else {
        print_line(out, "ERROR: Array index is negative.");
    }
}

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    let data = bad_source(out, reader);

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 && data < BUFFER_LEN as i32 {
        buffer[data as usize] = 1;
        print_buffer(out, &buffer);
    } else {
        print_line(out, "ERROR: Array index is out-of-bounds");
    }
}

fn good<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    good_g2b(out);
    good_b2g(out, reader);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    print_line(&mut out, "Calling good()...");
    good(&mut out, &mut reader);
    print_line(&mut out, "Finished good()");
    print_line(&mut out, "Calling bad()...");
    bad(&mut out, &mut reader);
    print_line(&mut out, "Finished bad()");

    let _ = out.flush();
}
