// Translated from main.c
mod q_math;

use std::env;
use std::process::exit;

use q_math::{Vec3, VectorNormalizeFast};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        exit(1);
    }

    let mut inputs: Vec3 = [0.0; 3];
    inputs[0] = parse_atof(&args[1]);
    inputs[1] = parse_atof(&args[2]);
    inputs[2] = parse_atof(&args[3]);

    VectorNormalizeFast(&mut inputs);

    println!("{:.6} {:.6} {:.6}", inputs[0], inputs[1], inputs[2]);
}

/// Mimics C's atof: parses leading whitespace and returns 0.0 on failure.
fn parse_atof(s: &str) -> f32 {
    let trimmed = s.trim_start();
    // Find the longest prefix that parses as a float
    let mut end = 0;
    let bytes = trimmed.as_bytes();
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_e = false;
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            seen_digit = true;
            i += 1;
        } else if c == b'.' && !seen_dot && !seen_e {
            seen_dot = true;
            i += 1;
        } else if (c == b'e' || c == b'E') && seen_digit && !seen_e {
            seen_e = true;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
        } else {
            break;
        }
    }
    end = i;
    if end == 0 || !seen_digit {
        return 0.0;
    }
    trimmed[..end].parse::<f32>().unwrap_or(0.0)
}
