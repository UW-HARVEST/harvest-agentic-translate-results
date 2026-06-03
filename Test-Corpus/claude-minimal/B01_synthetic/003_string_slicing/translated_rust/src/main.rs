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

use std::env;
use std::process::ExitCode;

/*
Index into a passed string
and print the substring indexed by [start, stop).
If there is no start, use 0.
If there is no stop, use the end of the string.
*/
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if (argc > 4) || (argc == 1) {
        println!("Error: there should be one to three arguments passed:");
        println!("<string> [start] [stop]");
        return ExitCode::from(1);
    }

    // Operate on bytes to mirror C's char-based indexing.
    let s = args[1].as_bytes();
    let len: usize = s.len();
    let start: usize;
    let stop: usize;

    if argc >= 3 {
        match args[2].parse::<i64>() {
            Ok(v) => {
                if v > len as i64 {
                    println!("Error: start is off the end of the string!");
                    return ExitCode::from(1);
                }
                if v < 0 {
                    // Match C behavior: comparison with size_t would treat
                    // negative as a huge unsigned, triggering the off-the-end
                    // check.
                    println!("Error: start is off the end of the string!");
                    return ExitCode::from(1);
                }
                start = v as usize;
            }
            Err(_) => {
                print!("Second argument must be an integer!");
                return ExitCode::from(1);
            }
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        match args[3].parse::<i64>() {
            Ok(v) => {
                if v > len as i64 {
                    println!("Error: stop is off the end of the string!");
                    return ExitCode::from(1);
                }
                if v < 0 {
                    println!("Error: stop must come after start!");
                    return ExitCode::from(1);
                }
                let v_us = v as usize;
                if v_us <= start {
                    println!("Error: stop must come after start!");
                    return ExitCode::from(1);
                }
                stop = v_us;
            }
            Err(_) => {
                print!("Third argument must be an integer!");
                return ExitCode::from(1);
            }
        }
    } else {
        stop = len;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    let slice = &s[start..stop];
    // Print the slice as bytes, matching C's `%.*s` which writes raw bytes.
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(slice);
    let _ = handle.write_all(b"\n");

    ExitCode::from(0)
}
