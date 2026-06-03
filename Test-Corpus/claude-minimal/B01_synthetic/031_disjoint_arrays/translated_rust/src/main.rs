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

use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out: Vec<i32> = vec![0; len];
    let mut ones: Vec<i32> = vec![0; len];
    let mut zeros: Vec<i32> = vec![0; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let mut data: [i32; 100] = [0; 100];
    let mut count: usize = 0;

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        // ignore - we'll just have no input
    }

    for token in input.split_ascii_whitespace() {
        if count >= 100 {
            break;
        }
        match token.parse::<i32>() {
            Ok(v) => {
                data[count] = v;
                count += 1;
            }
            Err(_) => break,
        }
    }

    let result = call_fma(&data, count);
    println!("{}", result);
}
