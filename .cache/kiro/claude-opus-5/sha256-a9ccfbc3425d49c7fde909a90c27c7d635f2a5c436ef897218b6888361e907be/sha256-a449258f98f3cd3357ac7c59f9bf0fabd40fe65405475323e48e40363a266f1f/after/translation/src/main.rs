// Rust translation of c_src/src/main.c
//
// Original C: Copyright 2025 MIT Lincoln Laboratory (MIT-style license, see
// c_src/src/main.c for the full notice).
//
// This is a faithful, behaviour-preserving translation. Bugs present in the C
// are reproduced, not fixed:
//
//   * `bad()` validates only `data >= 0` and then performs `buffer[data] = 1`
//     on a 10-element array, so any `data >= 10` is an out-of-bounds write
//     (CWE-129 / CWE-787). The C is compiled by `c_src/CMakeLists.txt` with no
//     optimisation flags, and at `-O0` that stray store lands on specific,
//     predictable slots of the real stack frame. Some of those slots are the
//     saved frame pointer and return addresses, so the C process dies with
//     SIGSEGV for particular values of `data`. `EmulatedFrame` below reproduces
//     that, slot for slot.
//   * The order of reads, validations and prints is preserved exactly.
//   * `fgets` reads at most 13 bytes and never past a newline, so a long input
//     line is split across the two `fgets` call sites, exactly as in C.
//   * stdout is buffered the way glibc buffers it, so output that the C process
//     never flushed before crashing is likewise never flushed here.

use std::io::{BufRead, StdinLock, Write};

/// Size of the C `char inputBuffer[14]`.
const INPUT_BUFFER_LEN: usize = 14;

/// Size of the C `int buffer[10]`.
const BUFFER_LEN: usize = 10;

// ---------------------------------------------------------------------------
// Emulation of the `-O0` stack frame that `bad()` actually writes through
// ---------------------------------------------------------------------------
//
// `objdump -d` of the shipped binary (gcc 11.5, no flags) gives `bad()`:
//
//     push %rbp ; mov %rsp,%rbp ; sub $0x40,%rsp
//     movl $0x1,-0x40(%rbp,%rax,4)      <- buffer[data] = 1
//
// so `buffer` lives at `%rbp-0x40` and `buffer[n]` stores 4 bytes at
// `%rbp - 64 + 4*n`. `main` is `push %rbp ; mov %rsp,%rbp ; sub $0x10,%rsp`,
// and it calls `bad` directly, so `bad`'s `%rbp` sits 32 bytes below `main`'s.
// That fixes the whole neighbourhood:
//
//   offset from bad's %rbp | contents                        | effect if clobbered
//   -----------------------+---------------------------------+--------------------
//   -64 ..-25             | int buffer[10]                   | the intended store
//   -22 ..-9              | char inputBuffer[14]             | dead, already parsed
//   -8                    | int i                            | none: `i = 0` is
//                         |                                  | executed *after*
//                         |                                  | the store
//   -4                    | int data                         | none: never re-read
//    0 .. 7               | saved %rbp (main's)              | CRASH in `leave`
//    8 ..15               | return address into main         | CRASH in `ret`
//   16 ..23               | main's argv                      | dead
//   24 ..27               | padding in main's frame          | dead
//   28 ..31               | main's argc                      | dead
//   32 ..39               | main's saved %rbp                | none: libc's caller
//                         |                                  | does not rely on it
//   40 ..47               | main's return address            | CRASH in main's `ret`
//   48 ..                 | libc frames, then the argv/env   | absorbed until the
//                         | block, then the top of the stack | stack region ends
//
// Reading that table off gives crashing indices {16,17,18,19} (bad's own saved
// %rbp and return address) and {26,27} (main's return address), which is
// exactly the set the real binary segfaults on. Everything from 28 up is
// absorbed by the argv/env block until the store passes the end of the stack
// mapping, at which point the C faults on the store itself.

/// Byte offset of `buffer` from `bad`'s `%rbp`, i.e. `-0x40`.
const BUFFER_OFFSET_FROM_RBP: i64 = -64;

/// Distance from the process's initial stack pointer down to `bad`'s `%rbp`.
///
/// Between the two sit the `__libc_start_main` / `__libc_start_call_main`
/// frames, `main`'s frame (`push %rbp` + `sub $0x10,%rsp`) and `bad`'s own
/// `push %rbp`. Those are all fixed sizes, so the distance is a constant of the
/// binary: it does not vary with the environment or with ASLR.
///
/// Measured on the reference binary by running it under `setarch -R` (ASLR off,
/// so the fault threshold is exact) and bisecting: index 1259 survives and 1260
/// faults, which places `stack_end - bad_rbp` in [4976, 4980). With
/// `stack_end - startstack` = 4672 read from `/proc/<pid>/{maps,stat}`, and
/// `bad_rbp` necessarily 16-byte aligned, the distance is exactly 304.
const RBP_BELOW_STARTSTACK: i64 = 304;

/// Used only if `/proc` cannot be read. Measured on the reference binary.
const FALLBACK_HEADROOM: i64 = 4_976;

/// Where the C process dies, if it dies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// The store landed somewhere harmless.
    None,
    /// The store hit unmapped memory: the C faults on the store instruction,
    /// before the print loop runs.
    OnStore,
    /// The store corrupted `bad`'s saved `%rbp` or return address: the C runs
    /// the print loop, then dies in `bad`'s epilogue.
    OnBadReturn,
    /// The store corrupted `main`'s return address: the C runs to the end of
    /// `main`, then dies in `main`'s `ret`.
    OnMainReturn,
}

/// Emulates the C stack frame that `int buffer[10]` lives in.
struct EmulatedFrame {
    buffer: [i32; BUFFER_LEN],
    /// Bytes of stack above `bad`'s `%rbp` that a stray store can reach before
    /// running off the end of the stack mapping.
    headroom: i64,
}

impl EmulatedFrame {
    fn new() -> Self {
        EmulatedFrame {
            // `int buffer[10] = { 0 };`
            buffer: [0; BUFFER_LEN],
            headroom: stack_headroom(),
        }
    }

    /// `buffer[index] = value` for a known-non-negative C `int` index,
    /// including every out-of-bounds case.
    fn store(&mut self, index: i32, value: i32) -> Fault {
        let offset = BUFFER_OFFSET_FROM_RBP + 4 * i64::from(index);

        // Inside the array: an ordinary store.
        if index >= 0 && (index as usize) < BUFFER_LEN {
            self.buffer[index as usize] = value;
            return Fault::None;
        }

        // Past the end of the stack mapping: the store itself faults.
        if offset + 4 > self.headroom {
            return Fault::OnStore;
        }

        match offset {
            // bad's saved %rbp (both halves) and its return address.
            0 | 4 | 8 | 12 => Fault::OnBadReturn,
            // main's return address.
            40 | 44 => Fault::OnMainReturn,
            // Dead locals, padding, saved registers, argv/env: absorbed.
            _ => Fault::None,
        }
    }
}

/// Bytes between `bad`'s frame and the top of the stack mapping.
///
/// The C's stray store is absorbed as long as it stays inside the stack
/// mapping, whose top is just above the argv/env block. That distance depends on
/// the size of the environment and, when ASLR is on, on a per-exec random offset
/// of up to 8 KiB that the kernel subtracts from the initial stack pointer
/// (`arch_align_stack`). Both are read back at run time from `/proc/self`, so
/// this tracks whatever environment the program is actually launched in instead
/// of baking in a threshold.
///
/// Anchoring on the initial stack pointer rather than on a Rust local is what
/// makes this faithful: Rust's frames sit at a different depth than the C's, so
/// a local's address would systematically over-estimate the headroom.
fn stack_headroom() -> i64 {
    match (stack_region_end(), start_stack()) {
        (Some(end), Some(start)) if end > start => {
            (end - start) as i64 + RBP_BELOW_STARTSTACK
        }
        _ => FALLBACK_HEADROOM,
    }
}

/// End address of the `[stack]` mapping, from `/proc/self/maps`.
fn stack_region_end() -> Option<u64> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if !line.ends_with("[stack]") {
            continue;
        }
        let range = line.split_whitespace().next()?;
        let end = range.split('-').nth(1)?;
        return u64::from_str_radix(end, 16).ok();
    }
    None
}

/// The `startstack` field of `/proc/self/stat`: the process's initial `%rsp`.
///
/// `startstack` is field 28. Fields 1 and 2 are skipped by splitting after the
/// final `)` of the `comm` field, since `comm` may itself contain spaces and
/// parentheses; field 3 is then the first token that remains.
fn start_stack() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = &stat[stat.rfind(')')? + 1..];
    after_comm
        .split_whitespace()
        .nth(28 - 3)
        .and_then(|f| f.parse::<u64>().ok())
}

/// Dies the way the C process dies: SIGSEGV, with nothing flushed.
///
/// A real store to an address the kernel will never map (page zero is below
/// `vm.mmap_min_addr`) faults exactly as the C's stray store does, so the
/// process is killed by SIGSEGV and the shell reports 128+11 = 139. Nothing is
/// written to stderr and the pending stdout buffer is discarded, matching the C.
fn die_with_sigsegv() -> ! {
    // The dangling pointer is the point: page zero can never be mapped because
    // it sits below `vm.mmap_min_addr`, so this store faults exactly as the C's
    // stray store does. `write_volatile` keeps the compiler from eliding it.
    #[allow(clippy::manual_dangling_ptr)]
    unsafe {
        std::ptr::write_volatile(1usize as *mut u8, 0u8);
    }
    // Not reached; present so the function can be typed `-> !`.
    std::process::abort();
}

// ---------------------------------------------------------------------------
// glibc-compatible stdout buffering
// ---------------------------------------------------------------------------

/// Stands in for C's `stdout` `FILE *`.
///
/// glibc picks the buffering mode on first use: line buffered when the stream
/// is a terminal, fully buffered otherwise. That distinction is what makes the
/// crashing runs produce *no* output at all when stdout is a pipe: everything
/// `printf` produced is still sitting in the buffer when the process dies.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        CStdout {
            buf: Vec::new(),
            line_buffered: stdout_is_terminal(),
        }
    }

    fn flush_now(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let out = std::io::stdout();
        let mut lock = out.lock();
        // Errors are ignored, matching C's unchecked printf/fflush.
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

impl Write for CStdout {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        // Fully buffered streams would flush once the buffer filled, but this
        // program never emits more than ~130 bytes, well under any BUFSIZ.
        if self.line_buffered && data.contains(&b'\n') {
            self.flush_now();
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_now();
        Ok(())
    }
}

/// `isatty(1)`, as glibc consults it when choosing the buffering mode.
///
/// Implemented as a character-device test on fd 1 to avoid a libc dependency.
/// The two disagree only for character devices that are not terminals, such as
/// `/dev/null`, where the output is discarded and the mode is unobservable.
fn stdout_is_terminal() -> bool {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::FileTypeExt;

    let fd = std::io::stdout().as_raw_fd();
    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    let is_char_device = file
        .metadata()
        .map(|m| m.file_type().is_char_device())
        .unwrap_or(false);
    // fd 1 is not ours to close.
    std::mem::forget(file);
    is_char_device
}

// ---------------------------------------------------------------------------
// Translated C functions
// ---------------------------------------------------------------------------

/// `printf("%s\n", line)` guarded by a NULL check.
///
/// The C takes `const char *` and skips NULL; `Option` makes that explicit.
fn print_line(out: &mut CStdout, line: Option<&str>) {
    if let Some(line) = line {
        let _ = writeln!(out, "{}", line);
    }
}

/// `printf("%d\n", intNumber)`.
fn print_int_line(out: &mut CStdout, int_number: i32) {
    let _ = writeln!(out, "{}", int_number);
}

/// True for the characters glibc's `isspace` accepts in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// glibc `atoi`, which is `(int) strtol(s, NULL, 10)`.
///
/// Skips leading whitespace, accepts one optional sign, consumes ASCII digits
/// and stops at the first non-digit; the `long` result is then truncated to
/// `int`. `inputBuffer` holds at most 13 characters, so `strtol` itself can
/// never overflow here, but the narrowing to `int` certainly can: e.g.
/// "1234567890123" yields 1912239307.
fn atoi(s: &[u8]) -> i32 {
    // The C string ends at the first NUL byte.
    let s = match s.iter().position(|&b| b == 0) {
        Some(nul) => &s[..nul],
        None => s,
    };

    let mut i = 0;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|a| a.checked_add(digit)) {
                Some(next) => acc = next,
                // Keep scanning digits, as strtol does, but remember the overflow.
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if overflowed {
        // strtol saturates, then the cast to int truncates:
        // (int) LONG_MAX == -1, (int) LONG_MIN == 0.
        return if negative {
            i64::MIN as i32
        } else {
            i64::MAX as i32
        };
    }

    let value = if negative { -acc } else { acc };
    // C's narrowing conversion to `int`: two's-complement truncation.
    value as i32
}

/// Reads one byte, or `None` at EOF / on error.
fn read_byte(reader: &mut StdinLock<'_>) -> Option<u8> {
    loop {
        let byte = match reader.fill_buf() {
            Ok(available) => {
                if available.is_empty() {
                    return None; // EOF
                }
                available[0]
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        reader.consume(1);
        return Some(byte);
    }
}

/// `fgets(dst, dst.len(), stdin)`; returns false where C returns NULL.
///
/// Copies at most `dst.len() - 1` bytes, stops after a newline (which is kept),
/// NUL-terminates, and leaves anything beyond that in the stream for the next
/// call. Returns false only when EOF/error is hit before any byte is read.
fn fgets(dst: &mut [u8], reader: &mut StdinLock<'_>) -> bool {
    let capacity = dst.len() - 1;
    let mut written = 0;
    while written < capacity {
        match read_byte(reader) {
            Some(byte) => {
                dst[written] = byte;
                written += 1;
                if byte == b'\n' {
                    break;
                }
            }
            None => break,
        }
    }
    if written == 0 {
        return false;
    }
    dst[written] = 0;
    true
}

/// Returns the fault that `bad`'s caller still has to act on, if any.
fn bad(out: &mut CStdout, reader: &mut StdinLock<'_>) -> Fault {
    let mut data: i32;
    // Initialize data
    data = -1;
    {
        let mut input_buffer = [0u8; INPUT_BUFFER_LEN];
        if fgets(&mut input_buffer, reader) {
            // Convert to int
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut frame = EmulatedFrame::new();
        if data >= 0 {
            // BUG (reproduced): no upper bound check, so any data >= 10 stores
            // outside the array and into the live stack frame.
            let fault = frame.store(data, 1);
            if fault == Fault::OnStore {
                // The C never reaches the print loop.
                die_with_sigsegv();
            }
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
            if fault == Fault::OnBadReturn {
                // The C dies returning from bad().
                die_with_sigsegv();
            }
            return fault;
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
    Fault::None
}

/// goodG2B uses the GoodSource with the BadSink
// The C has a dead store (`data = -1;` then `data = 7;`); it is kept verbatim.
#[allow(unused_assignments)]
fn good_g2b(out: &mut CStdout) {
    let mut data: i32;
    // Initialize data
    data = -1;
    data = 7;
    {
        let mut frame = EmulatedFrame::new();
        if data >= 0 {
            // `data` is 7 here, so this is always in bounds.
            frame.store(data, 1);
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
}

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g(out: &mut CStdout, reader: &mut StdinLock<'_>) {
    let mut data: i32;
    // Initialize data
    data = -1;
    {
        let mut input_buffer = [0u8; INPUT_BUFFER_LEN];
        if fgets(&mut input_buffer, reader) {
            // Convert to int
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut frame = EmulatedFrame::new();
        if data >= 0 && data < (BUFFER_LEN as i32) {
            // The bound check makes the store always in range.
            frame.store(data, 1);
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is out-of-bounds"));
        }
    }
}

fn good(out: &mut CStdout, reader: &mut StdinLock<'_>) {
    good_g2b(out);
    good_b2g(out, reader);
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut out = CStdout::new();

    print_line(&mut out, Some("Calling good()..."));
    good(&mut out, &mut reader);
    print_line(&mut out, Some("Finished good()"));
    print_line(&mut out, Some("Calling bad()..."));
    let fault = bad(&mut out, &mut reader);
    print_line(&mut out, Some("Finished bad()"));

    if fault == Fault::OnMainReturn {
        // The C dies in main's `ret`, after all printing but before exit()
        // flushes stdout.
        die_with_sigsegv();
    }

    // C returns 0 from main; exit() flushes stdout.
    out.flush_now();
}
