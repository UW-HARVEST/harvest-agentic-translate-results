// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

// Rust translation of c_src/src/main.c
//
// Original C:
//     int main() {
//         printf("Hello World!\n");
//         return 0;
//     }

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    // printf("Hello World!\n");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Write raw bytes so the output is byte-identical to the C program's.
    let _ = out.write_all(b"Hello World!\n");
    // C's exit from main flushes stdio streams before terminating.
    let _ = out.flush();

    // return 0;
    ExitCode::SUCCESS
}
