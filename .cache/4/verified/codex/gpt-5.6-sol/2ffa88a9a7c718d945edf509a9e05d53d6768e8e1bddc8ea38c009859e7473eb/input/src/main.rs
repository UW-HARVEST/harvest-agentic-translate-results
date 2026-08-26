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

use std::fmt::Write as _;
use std::io::{self, BufReader, Bytes, Read, StdinLock, Write};
use std::iter::Peekable;

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn next_byte(input: &mut Peekable<Bytes<BufReader<StdinLock<'_>>>>) -> Option<u8> {
    input.next()?.ok()
}

fn peek_byte(input: &mut Peekable<Bytes<BufReader<StdinLock<'_>>>>) -> Option<u8> {
    input.peek()?.as_ref().ok().copied()
}

fn scan_decimal(input: &mut Peekable<Bytes<BufReader<StdinLock<'_>>>>) -> Option<i32> {
    while peek_byte(input).is_some_and(is_c_whitespace) {
        next_byte(input);
    }

    let negative = match peek_byte(input) {
        Some(b'+') => {
            next_byte(input);
            false
        }
        Some(b'-') => {
            next_byte(input);
            true
        }
        _ => false,
    };

    if !peek_byte(input).is_some_and(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;
    while let Some(byte) = peek_byte(input).filter(u8::is_ascii_digit) {
        let digit = u64::from(byte - b'0');
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add(digit))
            .unwrap_or(limit)
            .min(limit);
        next_byte(input);
    }

    let value = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };
    Some(value as i32)
}

fn main() {
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock()).bytes().peekable();

    let mut output = String::new();
    for _ in 0..100 {
        let Some(value) = scan_decimal(&mut input) else {
            break;
        };
        let result = value.wrapping_mul(value).wrapping_add(value);
        writeln!(output, "{result}").unwrap();
    }

    io::stdout().write_all(output.as_bytes()).unwrap();
}
