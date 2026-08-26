// Rust translation of the C driver. The original C is shipped as a shared
// library exposing `driver(width_a, height_a, matrix_a, width_b, height_b,
// matrix_b)`. To produce an executable we wrap that function with a small
// main that reads its six arguments from stdin: two integers (width_a,
// height_a), then `height_a` matrix rows, then two integers (width_b,
// height_b), then `height_b` matrix rows.

mod driver;
mod matrix;
mod write;

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::from(1);
    }

    // Split the input into lines preserving order.
    let mut lines = input.split('\n');

    // Helper: pull next non-empty line (or empty string if exhausted).
    fn next_line<'a, I: Iterator<Item = &'a str>>(it: &mut I) -> &'a str {
        it.next().unwrap_or("")
    }

    // Read width_a, height_a (two integers separated by whitespace, possibly
    // on the same line).
    let header_a = next_line(&mut lines);
    let mut nums_a = header_a.split_whitespace();
    let width_a: i32 = match nums_a.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return ExitCode::from(1),
    };
    let height_a: i32 = match nums_a.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return ExitCode::from(1),
    };

    let mut matrix_a = String::new();
    for i in 0..height_a {
        if i > 0 {
            matrix_a.push('\n');
        }
        matrix_a.push_str(next_line(&mut lines));
    }

    let header_b = next_line(&mut lines);
    let mut nums_b = header_b.split_whitespace();
    let width_b: i32 = match nums_b.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return ExitCode::from(1),
    };
    let height_b: i32 = match nums_b.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return ExitCode::from(1),
    };

    let mut matrix_b = String::new();
    for i in 0..height_b {
        if i > 0 {
            matrix_b.push('\n');
        }
        matrix_b.push_str(next_line(&mut lines));
    }

    let rc = driver::driver(
        width_a, height_a, &matrix_a, width_b, height_b, &matrix_b,
    );
    ExitCode::from(rc as u8)
}
