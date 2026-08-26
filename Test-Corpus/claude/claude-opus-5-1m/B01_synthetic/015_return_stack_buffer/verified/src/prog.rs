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

//! Faithful Rust translation of `c_src/src/main.c`.
//!
//! The original C program is a CWE-562 ("Return of Stack Variable Address")
//! demonstration.  `helperBad()` returns the address of a function-local
//! `char` array, which is undefined behavior.  GCC (the reference compiler,
//! see `c_src/CMakeLists.txt`) diagnoses this with `-Wreturn-local-addr` and
//! emits a function that unconditionally returns a null pointer:
//!
//! ```text
//! 000000000000116b <helperBad>:
//!   116f: movabs $0x61427265706c6568,%rax   # "helperBa"
//!   1179: movabs $0x676e697274732064,%rdx   # "d string"
//!   1183: mov    %rax,-0x20(%rbp)
//!   1187: mov    %rdx,-0x18(%rbp)
//!   118b: movb   $0x0,-0x10(%rbp)
//!   118f: mov    $0x0,%eax                  # <-- returns NULL
//!   1194: pop    %rbp
//!   1195: ret
//! ```
//!
//! Because `printLine()` skips null pointers, the `bad()` path produces *no*
//! output at all.  That observed behavior is reproduced here verbatim; the
//! "bug" is deliberately preserved rather than fixed.

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Buffered byte-at-a-time stdin reader with one byte of push-back, mirroring
// the `inchar()` / `ungetc()` primitives glibc's `vfscanf` is built on.
// ---------------------------------------------------------------------------

const EOF: i32 = -1;

pub struct Stdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    pushed_back: Option<u8>,
}

impl Default for Stdin {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdin {
    pub fn new() -> Self {
        Stdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            pushed_back: None,
        }
    }

    /// Equivalent of glibc's `inchar()`: returns the next byte, or `EOF`.
    fn inchar(&mut self) -> i32 {
        if let Some(c) = self.pushed_back.take() {
            return c as i32;
        }
        if self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            self.pos += 1;
            return c as i32;
        }
        if self.eof {
            return EOF;
        }
        // Refill from the real stdin.
        self.buf.clear();
        self.pos = 0;
        let mut chunk = [0u8; 4096];
        match std::io::stdin().read(&mut chunk) {
            Ok(0) | Err(_) => {
                self.eof = true;
                EOF
            }
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                let c = self.buf[0];
                self.pos = 1;
                c as i32
            }
        }
    }

    /// Equivalent of `ungetc()`; `EOF` push-back is a no-op, as in C.
    fn ungetc(&mut self, c: i32) {
        if c != EOF {
            self.pushed_back = Some(c as u8);
        }
    }
}

/// C `isspace()` for the "C" locale.
fn is_space(c: i32) -> bool {
    c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0B || c == 0x0C || c == 0x0D
}

/// C `isdigit()`.
fn is_digit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}

/// C `tolower()` restricted to ASCII.
fn to_lower(c: i32) -> i32 {
    if (b'A' as i32..=b'Z' as i32).contains(&c) {
        c + 32
    } else {
        c
    }
}

// ---------------------------------------------------------------------------
// `strtol()` semantics over an already-validated digit buffer.
//
// glibc's `%d` conversion collects candidate characters and then hands them to
// `__strtol_internal`, i.e. it accumulates into a `long` (64-bit on the
// reference platform) and *saturates* to `LONG_MAX` / `LONG_MIN` on overflow.
// The resulting `long` is then narrowed with `*ARG (int *) = (int) num.l`,
// which truncates to the low 32 bits.  This is why, for example,
// "4294967296" yields 0 while "99999999999999999999" yields -1.
// ---------------------------------------------------------------------------
fn strtol_base10(buf: &[u8]) -> i64 {
    let mut i = 0usize;
    let mut negative = false;

    if i < buf.len() && (buf[i] == b'+' || buf[i] == b'-') {
        negative = buf[i] == b'-';
        i += 1;
    }

    // Magnitude limit: 2^63 - 1 for positives, 2^63 for negatives.
    let cutoff: u64 = if negative {
        i64::MIN.unsigned_abs()
    } else {
        i64::MAX as u64
    };

    let mut acc: u64 = 0;
    let mut overflowed = false;

    while i < buf.len() {
        let digit = u64::from(buf[i] - b'0');
        i += 1;
        if overflowed {
            continue;
        }
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) if v <= cutoff => acc = v,
            _ => overflowed = true,
        }
    }

    if overflowed {
        // ERANGE: strtol clamps and scanf stores the clamped value anyway.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    }
}

/// Emulates a single `scanf("%d", &x)` directive.
///
/// Returns `Some(value)` when the conversion succeeds (so the caller assigns
/// through the pointer), or `None` on input failure (EOF before any
/// non-whitespace character) or matching failure (no digits available), in
/// which case C leaves the destination object untouched.
pub fn scanf_d(input: &mut Stdin) -> Option<i32> {
    // --- Leading whitespace skip performed for every non-%c/%[/%n directive.
    let mut c;
    loop {
        c = input.inchar();
        if c == EOF {
            return None; // input failure
        }
        if !is_space(c) {
            break;
        }
    }
    input.ungetc(c);

    // --- glibc's `number:` label.
    c = input.inchar();
    if c == EOF {
        return None; // input failure
    }

    let base: i32 = 10;
    let mut charbuf: Vec<u8> = Vec::new();

    // Optional sign.
    if c == b'-' as i32 || c == b'+' as i32 {
        charbuf.push(c as u8);
        c = input.inchar();
    }

    // Leading base indication.  For an explicit base of 10 a following "x" is
    // neither consumed nor treated specially, so the digit loop below stops
    // immediately on it (hence "0x10" parses as just 0).
    if c == b'0' as i32 {
        charbuf.push(c as u8);
        c = input.inchar();
        if to_lower(c) == b'x' as i32 {
            // base is 10: not 0 (would become 16) and not 16, so no action.
        }
    }

    // Digit collection loop.
    while c != EOF {
        if !is_digit(c) || (c - b'0' as i32) >= base {
            break;
        }
        charbuf.push(c as u8);
        c = input.inchar();
    }

    // "There was no number" check: empty buffer, or a lone sign.
    if charbuf.is_empty() || (charbuf.len() == 1 && (charbuf[0] == b'+' || charbuf[0] == b'-')) {
        input.ungetc(c);
        return None; // matching failure
    }

    input.ungetc(c);

    // `*ARG (int *) = (int) num.l;`
    Some(strtol_base10(&charbuf) as i32)
}

// ---------------------------------------------------------------------------
// Translation of the C functions.
// ---------------------------------------------------------------------------

/// `void printLine(const char *line)` — a null pointer prints nothing.
///
/// `None` models a null `const char *`; `Some(bytes)` models the NUL-terminated
/// byte string a non-null pointer refers to (the terminator itself excluded).
/// GCC lowers `printf("%s\n", line)` to `puts(line)`, i.e. the raw bytes
/// followed by a single line feed.
pub fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line);
        let _ = out.write_all(b"\n");
    }
}

/// `static char *helperBad()` — returns the address of a stack array.
///
/// GCC's code generation for this undefined-behavior construct is an
/// unconditional null return (see the module docs), which is what we model.
/// The dead local string is kept for fidelity with the original source.
fn helper_bad() -> Option<&'static [u8]> {
    let char_string: [u8; 17] = *b"helperBad string\0";
    let _ = char_string; // written to the stack frame, then abandoned
    None
}

/// `void bad()`
pub fn bad() {
    print_line(helper_bad());
}

/// `static char *helperGood1()` — returns a pointer to a `static` array,
/// which has static storage duration and therefore remains valid.
fn helper_good1() -> Option<&'static [u8]> {
    static CHAR_STRING: [u8; 19] = *b"helperGood1 string\0";
    // The C caller receives a `char *` to the first element; `printf("%s")`
    // stops at the NUL terminator, which is therefore not part of the output.
    Some(&CHAR_STRING[..CHAR_STRING.len() - 1])
}

/// `void good()`
pub fn good() {
    print_line(helper_good1());
}

/// `int main()`
pub fn c_main() -> i32 {
    let mut input = Stdin::new();

    // int x = 0;
    // scanf("%d", &x);
    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // return 0;
    0
}

/// Flush the standard output stream, mirroring the implicit flush glibc
/// performs when a C program returns from `main`.
pub fn flush_stdout() {
    let _ = std::io::stdout().flush();
}
