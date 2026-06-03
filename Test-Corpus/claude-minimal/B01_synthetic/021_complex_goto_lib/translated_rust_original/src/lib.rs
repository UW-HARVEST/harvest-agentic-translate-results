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

pub fn driver(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");

        // If `x == 1 && y == 4`, the original C code does `goto label2`,
        // skipping the `label1` block. We model that by tracking whether
        // to skip the label1 block on the first iteration of the inner loop.
        let mut skip_label1 = x == 1 && y == 4;

        // Inner loop models the `goto label1` cycle inside the outer body.
        loop {
            // label1
            if !skip_label1 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2
            if y == 0 {
                // Equivalent to `continue` in the outer while loop.
                break;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // Equivalent to `goto label1`.
                continue;
            }
            // Fall through to the bottom of the outer while body.
            break;
        }
    }
}
