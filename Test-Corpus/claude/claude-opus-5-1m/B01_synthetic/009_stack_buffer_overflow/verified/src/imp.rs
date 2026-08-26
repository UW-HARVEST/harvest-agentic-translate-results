// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::os::raw::{c_char, c_int};

// glibc's `stdin`. The `libc` crate does not re-export this FILE pointer on
// linux-gnu, so it is declared here; it resolves to the very same object the C
// translation unit reads from, which is the whole point (see below). Output goes
// through `printf`, which uses `stdout` implicitly.
extern "C" {
    #[link_name = "stdin"]
    static C_STDIN: *mut libc::FILE;
}

// ---------------------------------------------------------------------------
// I/O
//
// This layer deliberately calls the *same* glibc stdio functions the C calls,
// on the *same* `stdin` / `stdout` FILE objects, instead of reimplementing them
// on top of `std::io`. That is not incidental -- four separate observable
// behaviors of the C program are properties of C stdio and cannot be reproduced
// otherwise:
//
//  1. `stdout` is line-buffered on a terminal and block-buffered otherwise. The
//     difference is visible precisely because `bad()` can kill the process: on a
//     tty the C program emits 167 bytes before dying at index 16, but 0 bytes
//     when stdout is a pipe (the block buffer is lost). A `BufWriter` is
//     unconditionally block-buffered and got the tty case wrong.
//  2. glibc repositions a *seekable* `stdin` to the logical read position when
//     the process exits, so `{ driver >/dev/null; cat; } < file` lets `cat` see
//     the unread remainder. Rust's `io::Stdin` slurps 8 KiB into its own
//     `BufReader` and leaves the file descriptor at EOF.
//  3. A C consumer of the shared object shares these FILE buffers, so its
//     `printf` output interleaves with `printLine`'s in call order, and its own
//     `fgets` cooperates with `bad()`'s.
//  4. The buffered-but-unflushed output is discarded when the process dies from
//     the out-of-bounds write, which is what makes the crash produce *no* output
//     on a pipe.
// ---------------------------------------------------------------------------

/// Handle for the process-wide C `stdin` / `stdout` streams.
///
/// Carries no state of its own; the buffering state lives in glibc, exactly as
/// it does for the C program.
pub struct Io {
    _priv: (),
}

impl Default for Io {
    fn default() -> Self {
        Self::new()
    }
}

impl Io {
    pub fn new() -> Self {
        Io { _priv: () }
    }

    /// `fgets(inputBuffer, size, stdin)`.
    ///
    /// Returns the bytes up to the first NUL, or `None` when `fgets` returns
    /// NULL. Truncating at the first NUL loses nothing: the only consumer is
    /// `atoi`, which itself stops at the first NUL, so the result is exactly the
    /// C string `fgets` produced.
    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        // Pre-zeroed, so the scan below finds either an input NUL or fgets's
        // terminator, whichever comes first.
        let mut buf = vec![0u8; size + 1];
        let rc = unsafe {
            libc::fgets(
                buf.as_mut_ptr() as *mut c_char,
                size as c_int,
                C_STDIN,
            )
        };
        if rc.is_null() {
            return None;
        }
        let n = buf.iter().position(|&b| b == 0).unwrap_or(size);
        buf.truncate(n);
        Some(buf)
    }

    /// `printf("%s\n", line)` where `line` is a NUL-terminated C string.
    ///
    /// The format string is a fixed literal, exactly as in the C source, so the
    /// bytes of `line` are data and are never interpreted as a format.
    pub fn print_cstr(&mut self, line: *const c_char) {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }

    /// `printLine(line)` for one of the program's own string literals.
    pub fn print_line(&mut self, line: &'static [u8]) {
        debug_assert_eq!(line.last(), Some(&0), "print_line needs a NUL terminator");
        self.print_cstr(line.as_ptr() as *const c_char);
    }

    /// `printf("%d\n", intNumber)`.
    pub fn print_int_line(&mut self, int_number: i32) {
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number as c_int);
        }
    }
}

/// Emulates glibc's `atoi()`, which is `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is consumed, decimal digits
/// are accumulated, parsing stops at the first non-digit (or the NUL
/// terminator), and out-of-range values saturate to `long` bounds before being
/// truncated to `int`.
pub fn c_atoi(bytes: &[u8]) -> i32 {
    // A C string ends at the first NUL byte.
    let s = match bytes.iter().position(|&b| b == 0) {
        Some(pos) => &bytes[..pos],
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

    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    let as_long: i64 = if overflow {
        // strtol clamps to LONG_MAX / LONG_MIN (long is 64-bit here).
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        value.wrapping_neg()
    } else {
        value
    };

    // (int) cast: implementation-defined truncation of the low 32 bits.
    as_long as i32
}

// ---------------------------------------------------------------------------
// CWE-129 sink: `buffer[data] = 1` with no upper-bound check.
//
// For `data >= 10` the C code writes outside `buffer`, which is undefined
// behavior. The C is the ground truth, so the observable consequences of that
// write have to be reproduced rather than "fixed". They are fully determined by
// the stack frame gcc -O0 emits, read straight out of `objdump -d` of the
// CMake-built binary (the shared-object build is byte-identical in layout):
//
//     bad():  push %rbp; mov %rsp,%rbp; sub $0x40,%rsp
//             buffer      at rbp-0x40      inputBuffer at rbp-0x16
//             i           at rbp-0x08      data        at rbp-0x04
//     main(): push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
//             argc        at main_rbp-0x04 argv        at main_rbp-0x10
//     => main_rbp == bad_rbp + 0x20
//
// `buffer[k]` is a 4-byte store at `&buffer[0] + 4*k`, i.e. `bad_rbp-0x40+4*k`:
//
//   k        byte offset  target                        observable effect
//   -------  -----------  ----------------------------  ------------------------
//   0..9       0..39      buffer[k]                     prints 1 at position k
//   10          40        alignment padding             none (dead storage)
//   11..13    44..52      inputBuffer[2..14]            none (dead after atoi)
//   14          56        i                             none (`i = 0` follows)
//   15          60        data                          none (already in %eax)
//   16..17    64..68      bad()'s saved rbp             death, but only once the
//                                                       *caller* uses rbp again
//   18..19    72..76      bad()'s return address        death at bad()'s `ret`
//   20..23    80..92      main()'s argv / argc          none
//   24..25    96..100     main()'s saved rbp            none (main only leave/ret)
//   26..27   104..108     main()'s return address       death at main()'s `ret`
//   >= 28    >= 112       libc start frames, env block  none, until the store
//                                                       passes the top of the
//                                                       stack mapping -> death
//
// Two things about this matter for byte-identical output and are modelled below.
//
// TIMING. The store itself never faults for k < 28; what faults is a *later*
// `ret`. So the ten `buffer[i]` values are still printed first, and how much
// output survives depends on which return is corrupted. Measured with stdout on
// a tty (where C line-buffers, so the difference is visible):
//     k = 16, 17, 26, 27 -> 167 bytes: everything, including "Finished bad()"
//     k = 18, 19         -> 151 bytes: the ten values, but not "Finished bad()"
//     far / off-stack    -> 121 bytes: the store faults immediately, so none of
//                           the ten values are printed
// With stdout on a pipe all of these collapse to 0 bytes, because the block
// buffer is discarded -- which is why this only shows up on a terminal.
//
// CALLER DEPENDENCE. Indices 16..19 hit `bad()`'s *own* frame, so they behave the
// same no matter who called it. Indices >= 20 hit the caller's frame, so their
// effect is a property of the caller, not of `bad()`. In the executable the
// caller is this program's `main` with the layout above, giving the fatal pair
// 26..27. When `bad` is reached through the shared object's export, the caller is
// some unknown consumer whose frame this code cannot see, so no caller-frame
// index is treated as fatal -- inventing a crash there would be strictly worse
// than declining to, because the vast majority of indices >= 20 really are benign.
// See CONFIGS.md for the full discussion.
//
// Verified empirically: for k in 0..=1300 (under the test environment) the CMake
// binary is 100% reproducible over repeated runs and dies for exactly
// k in {16,17,18,19,26,27}. Far beyond that the outcome stops being reproducible
// *in C itself*, because stack ASLR moves the top of the stack relative to the
// frame; measured over 40 runs per index, k = 1500..2200 dies only some of the
// time, and for `4*k` in the gigabyte range even the *signal* is random (C was
// measured at 33% SIGBUS / 67% SIGSEGV for k = i32::MAX). Those regions cannot be
// matched by construction; `far_write_is_fatal` derives the boundary from the
// real stack mapping so that the overwhelmingly more common very large indices
// die the way C's do.
// ---------------------------------------------------------------------------

/// Which frame sits above `bad()`'s, and therefore what the out-of-bounds store
/// can reach beyond index 19.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Caller {
    /// `bad()` was called from this program's own `main`, laid out exactly as
    /// gcc -O0 lays it out. Indices 26..27 hit `main`'s return address.
    CMain,
    /// `bad()` was entered through the shared object's export. The caller's frame
    /// is unknowable, so only `bad()`'s own frame is modelled.
    Unknown,
}

/// A fault that the corrupted stack has armed but that has not gone off yet,
/// because the corrupted return address is only used later.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Death {
    /// Nothing was corrupted.
    None,
    /// `bad()`'s return address is clobbered: it dies at its own `ret`, after
    /// printing the ten values but before the caller can print anything more.
    AtBadReturn,
    /// A frame pointer or return address further up is clobbered: everything
    /// `main` prints still comes out, and the fault happens as `main` returns.
    AtMainReturn,
}

/// `buffer[k]` for these `k` overwrites `bad()`'s own **return address**, so the
/// `ret` at the end of `bad()` jumps to garbage. Fatal for every caller, because
/// nothing about the caller can save it.
const FATAL_OWN_RETURN_ADDRESS: [i32; 2] = [18, 19];

/// `buffer[k]` for these `k` overwrites `bad()`'s saved rbp (16, 17) or the
/// return address of the `main` above it (26, 27).
///
/// Only fatal when the frame above really is this program's `main` as gcc -O0
/// emits it: a caller that keeps a frame pointer in `rbp` and later returns
/// through it. That is true of `main` here, and of a `gcc -O0` consumer of the
/// shared object, but *not* of an optimized caller that never reloads `rbp` --
/// such a caller survives indices 16 and 17 untouched. Since the export cannot
/// see its caller's code generation, [`Caller::Unknown`] deliberately treats
/// these as benign rather than fabricating a fault that need not happen.
const FATAL_CMAIN_FRAME: [i32; 4] = [16, 17, 26, 27];

/// End address of the mapping that contains `addr` -- i.e. the first address past
/// it, where a store starts faulting.
///
/// The mapping containing the current frame is looked up rather than hardcoding
/// the `[stack]` label, so this stays correct when the exported `bad()` is called
/// on a secondary thread, whose stack is an ordinary anonymous mapping.
fn mapping_end_containing(addr: usize) -> Option<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        let range = match line.split_whitespace().next() {
            Some(r) => r,
            None => continue,
        };
        let (lo, hi) = match range.split_once('-') {
            Some(p) => p,
            None => continue,
        };
        let lo = match usize::from_str_radix(lo, 16) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let hi = match usize::from_str_radix(hi, 16) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if addr >= lo && addr < hi {
            return Some(hi);
        }
    }
    None
}

/// True when the 4-byte store `buffer[k] = 1` would land past the end of the
/// mapping holding the current frame, and therefore fault, as it does in C for
/// large `k`.
///
/// If `/proc` cannot be read the store is treated as benign: that is the
/// behavior for every `k` that stays inside the stack, and guessing a fault
/// without evidence would invent a crash the C need not have.
fn far_write_is_fatal(frame_probe: usize, k: i32) -> bool {
    let end = match mapping_end_containing(frame_probe) {
        Some(e) => e,
        None => return false,
    };
    let offset = 4usize.saturating_mul(k as usize);
    frame_probe.saturating_add(offset) >= end
}

/// Reproduces the death of the C process from the out-of-bounds store: a fault
/// with whatever is still sitting unflushed in C's `stdout` buffer discarded.
///
/// A store to the unmapped zero page raises SIGSEGV exactly as the corrupted
/// return address / past-the-stack store does in C. `fflush` is deliberately not
/// called, so the buffered output is lost just as C's is.
fn die_like_c_oob_write() -> ! {
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    }
    // Not reached; keeps the function diverging even if the store above were
    // ever optimized away.
    std::process::abort();
}

/// The unchecked sink: `buffer[data] = 1` followed by printing `buffer[0..10]`.
/// Returns the fault the store has armed, if any.
fn bad_sink(io: &mut Io, data: i32, caller: Caller) -> Death {
    let mut buffer = [0i32; 10];
    let probe: u32 = 0;
    let frame_probe = &probe as *const u32 as usize;

    if data >= 0 {
        // buffer[data] = 1;
        let k = data;
        let armed = if k < 10 {
            buffer[k as usize] = 1;
            Death::None
        } else if FATAL_OWN_RETURN_ADDRESS.contains(&k) {
            // bad()'s own `ret` is now poisoned: fatal no matter who called.
            Death::AtBadReturn
        } else if caller == Caller::CMain && FATAL_CMAIN_FRAME.contains(&k) {
            // 16,17 poison bad()'s saved rbp and 26,27 main()'s return address;
            // either way the fault lands as main returns, after all its output.
            Death::AtMainReturn
        } else if far_write_is_fatal(frame_probe, k) {
            // The store itself faults, before anything else is printed.
            die_like_c_oob_write();
        } else {
            // The store lands in dead storage of this frame or of a caller's
            // frame; it cannot change anything that is printed below.
            Death::None
        };

        /* Print the array values */
        for i in 0..10 {
            io.print_int_line(buffer[i]);
        }
        armed
    } else {
        io.print_line(b"ERROR: Array index is negative.\0");
        Death::None
    }
}

pub fn bad(io: &mut Io, caller: Caller) -> Death {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        // char inputBuffer[14] = "";
        match io.fgets(14) {
            Some(input_buffer) => {
                /* Convert to int */
                data = c_atoi(&input_buffer);
            }
            None => {
                io.print_line(b"fgets() failed.\0");
            }
        }
    }
    match bad_sink(io, data, caller) {
        // bad()'s own `ret` is the faulting instruction, so nothing after this
        // call in the caller ever runs.
        Death::AtBadReturn => die_like_c_oob_write(),
        other => other,
    }
}

/* goodG2B uses the GoodSource with the BadSink */
#[allow(unused_assignments)] // mirrors the dead store in the C original
pub fn good_g2b(io: &mut Io) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    data = 7;
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10 {
                io.print_int_line(buffer[i]);
            }
        } else {
            io.print_line(b"ERROR: Array index is negative.\0");
        }
    }
}

/* goodB2G uses the BadSource with the GoodSink */
pub fn good_b2g(io: &mut Io) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        // char inputBuffer[14] = "";
        match io.fgets(14) {
            Some(input_buffer) => {
                /* Convert to int */
                data = c_atoi(&input_buffer);
            }
            None => {
                io.print_line(b"fgets() failed.\0");
            }
        }
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10 {
                io.print_int_line(buffer[i]);
            }
        } else {
            io.print_line(b"ERROR: Array index is out-of-bounds\0");
        }
    }
}

pub fn good(io: &mut Io) {
    good_g2b(io);
    good_b2g(io);
}

/// The body of the C `main()`. Returns the value C returns (0).
pub fn run_main(io: &mut Io, caller: Caller) -> i32 {
    io.print_line(b"Calling good()...\0");
    good(io);
    io.print_line(b"Finished good()\0");
    io.print_line(b"Calling bad()...\0");
    let death = bad(io, caller);
    io.print_line(b"Finished bad()\0");
    if death == Death::AtMainReturn {
        // A saved frame pointer or return address at or above main's frame was
        // clobbered; the fault happens as main returns, after all of the above
        // has been printed.
        die_like_c_oob_write();
    }
    0
}
