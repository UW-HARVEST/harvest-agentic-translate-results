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

use std::io::{self, Write};

fn print_line<W: Write>(output: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        let _ = writeln!(output, "{line}");
    }
}

#[allow(dead_code)]
fn helper_bad<W: Write>(output: &mut W) {
    print_line(output, Some("helperBad()"));
}

fn bad<W: Write>(output: &mut W) {
    print_line(output, Some("bad()"));
}

fn helper_good<W: Write>(output: &mut W) {
    print_line(output, Some("helperGood()"));
}

fn good<W: Write>(output: &mut W) {
    print_line(output, Some("good()"));
    helper_good(output);
}

fn main() {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    print_line(&mut output, Some("Calling good()..."));
    good(&mut output);
    print_line(&mut output, Some("Finished good()"));
    print_line(&mut output, Some("Calling bad()..."));
    bad(&mut output);
    print_line(&mut output, Some("Finished bad()"));
}
