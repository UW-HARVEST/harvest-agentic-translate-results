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

//! Rust translation of `src/mdmain.c` -- the `driver` executable.
//!
//! The library target is a `cdylib`, which Cargo cannot link into a Rust binary,
//! so the driver pulls the same module files in directly via `#[path]`. The C
//! build does the equivalent thing: `mdcore.c` and `mdmain.c` are compiled into
//! one `add_executable(driver ...)`.

#[path = "mdmacros.rs"]
mod mdmacros;

#[path = "mdcore.rs"]
mod mdcore;

#[path = "stdio.rs"]
mod stdio;

#[path = "cstdlib.rs"]
mod cstdlib;

use core::ffi::c_int;

use cstdlib::atoi;
use mdcore::{g_op, g_op_name_bytes, helper_call, helper_ptr, use_generated};
use mdmacros::{run_loop, INIT, OP_FN, REPEAT};

/// Collects `argv` as raw byte strings so `%s` reproduces the original bytes
/// even for arguments that are not valid UTF-8.
fn argv_bytes() -> Vec<Vec<u8>> {
    std::env::args_os()
        .map(|arg| {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                arg.as_bytes().to_vec()
            }
            #[cfg(not(unix))]
            {
                arg.to_string_lossy().into_owned().into_bytes()
            }
        })
        .collect()
}

fn main() {
    let argv = argv_bytes();
    let argc = argv.len() as c_int;

    if argc < 3 {
        // fprintf(stderr, "usage: %s A B\n", argv[0]);
        let mut msg = Vec::new();
        msg.extend_from_slice(b"usage: ");
        msg.extend_from_slice(argv.first().map(|v| v.as_slice()).unwrap_or(b""));
        msg.extend_from_slice(b" A B\n");
        stdio::eprint_bytes(&msg);
        stdio::flush_stdout();
        std::process::exit(2);
    }

    let a = atoi(&argv[1]);
    let b = atoi(&argv[2]);

    // int r_call = (OP_FN(OP))(a, b);
    let r_call = OP_FN(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let acc = run_loop(INIT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = g_op()(a, b);

    // printf("op=%s call=%d acc=%d g.call=%d\n", G_OP_NAME, r_call, acc, g);
    let mut line = Vec::new();
    line.extend_from_slice(b"op=");
    line.extend_from_slice(g_op_name_bytes());
    line.extend_from_slice(format!(" call={} acc={} g.call={}\n", r_call, acc, g).as_bytes());
    stdio::print_bytes(&line);

    // printf("summary=%d\n", r_call + acc + x1 + x2 + x3 + g);
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    stdio::print_str(&format!("summary={}\n", summary));

    stdio::flush_stdout();
    std::process::exit(0);
}
