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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C translation unit is an executable whose only external linkage is
//! `void run(int)` (everything else is `static`).  This crate exposes the same
//! surface: it builds a `driver` binary that reproduces the C `main()` byte for
//! byte, and a `cdylib` that exports `run` under the C ABI so the two can be
//! compared through the FFI boundary.
//!
//! Behaviour that is faithfully reproduced:
//!
//! * `fgets(in, sizeof(in), stdin)` semantics over `char in[100] = ""`: at most
//!   99 bytes are consumed, reading stops after the first `'\n'` (kept in the
//!   buffer), and an immediate EOF leaves the buffer as the empty string.
//! * The buffer is a C string, so an embedded NUL truncates what `strtol` sees.
//! * `strtol(str, &endp, 10)` semantics, including `ERANGE` saturation and the
//!   "no conversion" case where `endp` is reset to `str`.
//! * `parse_val`'s exact acceptance order: `endp != str`, `errno == 0`,
//!   `tmp >= INT_MIN`, `tmp <= INT_MAX`.
//! * The file-scope `the_house` is process-global and is **not** reset between
//!   the two `run()` calls, so the second call continues from the mutated state.
//! * `floors++` / `bedrooms += extra_bedrooms` are signed-overflow UB in C; the
//!   emitted code wraps, so `wrapping_add` is used.
//! * `SIGPIPE` keeps its default disposition (Rust's runtime otherwise ignores
//!   it), so a closed stdout kills the process with signal 13 like the C does.

pub mod house;
pub mod parse;

pub use house::{run_global, House, THE_HOUSE_INIT};
pub use parse::{fgets_line, parse_val};

use std::io::Write;

/// The body of the C `int main()`.
///
/// ```c
/// int main() {
///     char in[100] = "";
///     fgets(in, sizeof(in), stdin);
///     int x;
///     if (parse_val(in, &x)) {
///         run(x);
///         run(x);
///     } else {
///         printf("An error occurred\n");
///     }
///     return 0;
/// }
/// ```
///
/// Returns the C exit status (always `0`).
///
/// `run` is taken as a parameter so that each leaf target can hand in its own
/// `#[no_mangle] extern "C" fn run`, mirroring the fact that the C `main()`
/// reaches `run()` through external linkage.
pub fn c_main_with(run: extern "C" fn(std::os::raw::c_int)) -> i32 {
    // The C program inherits the default SIGPIPE disposition; Rust's runtime
    // installs SIG_IGN, so undo that before writing anything.
    restore_default_sigpipe();

    // char in[100] = ""; fgets(in, sizeof(in), stdin);
    let input = fgets_line(100);

    // int x; if (parse_val(in, &x)) { run(x); run(x); } else { ... }
    match parse_val(&input) {
        Some(x) => {
            run(x);
            run(x);
        }
        None => {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let _ = write!(out, "An error occurred\n");
            let _ = out.flush();
        }
    }

    // return 0;
    0
}

/// `c_main_with` using the library's own private `run` shim.
pub fn c_main() -> i32 {
    extern "C" fn run_shim(extra_bedrooms: std::os::raw::c_int) {
        run_global(extra_bedrooms);
    }
    c_main_with(run_shim)
}

/// Restore the default `SIGPIPE` disposition.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which the C
/// program never does.  Without this, a C program that dies from `SIGPIPE` while
/// `printf`-ing to a closed pipe (exit status 141) would instead be a Rust
/// program that silently ignores the write error and exits 0.
#[cfg(unix)]
pub fn restore_default_sigpipe() {
    // SIGPIPE == 13 and SIG_DFL == 0 on every Unix target this program targets.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    // SAFETY: `signal` with `SIG_DFL` is async-signal-safe and always valid.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
pub fn restore_default_sigpipe() {}
