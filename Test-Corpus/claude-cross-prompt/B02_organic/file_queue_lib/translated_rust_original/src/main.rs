// Translation of C library in c_src/ to Rust.
// The C code is a library exposing `driver(day, month, year, timeout, flags)`.
// This Rust executable wraps that library: it reads parameters from stdin
// using scanf-style parsing, invokes the driver logic, and prints the parsed
// alert data fields to stdout.

mod shared;
mod read_alert;
mod file_queue;
mod driver_lib;

use std::io::{self, Read, Write};

use crate::file_queue::AlertSource;

// Flags (mirroring read-alert.h)
pub const CRALERT_MAIL_SET: i32 = 0x001;
pub const CRALERT_EXEC_SET: i32 = 0x002;
pub const CRALERT_READ_ALL: i32 = 0x004;
pub const CRALERT_READ_FAILED: i32 = 0x008;
pub const CRALERT_FP_SET: i32 = 0x010;

fn print_field(out: &mut dyn Write, name: &str, value: &Option<String>) {
    match value {
        Some(s) => writeln!(out, "{}={}", name, s).unwrap(),
        None => writeln!(out, "{}=(null)", name).unwrap(),
    }
}

fn main() {
    // Read all of stdin into memory; parse parameters from the start using
    // scanf-style behavior (whitespace-separated integers, crossing newlines).
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    // scanf("%d %d %d %u %d", &day, &month, &year, &timeout, &flags)
    let mut tokens = input.split_ascii_whitespace();
    let day = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => std::process::exit(1),
    };
    let month = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => std::process::exit(1),
    };
    let year = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => std::process::exit(1),
    };
    let timeout = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
        Some(v) => v,
        None => std::process::exit(1),
    };
    let flags = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => std::process::exit(1),
    };

    // Determine the byte offset of the start of the next line after the 5th
    // token, so the rest of stdin is available as the "alert log" content.
    let bytes = input.as_bytes();
    let mut tok_idx = 0;
    let mut i = 0usize;
    while i < bytes.len() && tok_idx < 5 {
        // skip whitespace
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        // consume token
        while i < bytes.len() && !(bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        tok_idx += 1;
    }
    // Skip a single trailing whitespace (newline) after the 5 tokens, like
    // scanf would leave the file pointer after the last digit.
    let consumed = i;

    let remaining = &bytes[consumed..];

    // Construct alert source: stdin (in-memory remaining bytes) when
    // CRALERT_FP_SET is set; otherwise attempt to open "alerts.log".
    let source = if (flags & CRALERT_FP_SET) != 0 {
        AlertSource::new_in_memory(remaining.to_vec())
    } else {
        match AlertSource::new_from_path("alerts.log") {
            Some(s) => s,
            None => {
                // No file available; mimic C's behavior of returning NULL
                let stdout = io::stdout();
                let mut out = stdout.lock();
                writeln!(out, "(null)").unwrap();
                return;
            }
        }
    };

    // Mirror driver.c: build a struct tm and a file_queue, call Init/Read.
    let alert = driver_lib::driver(day, month, year, timeout, flags, source);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    match alert {
        Some(a) => {
            writeln!(out, "rule={}", a.rule).unwrap();
            writeln!(out, "level={}", a.level).unwrap();
            print_field(&mut out, "alertid", &a.alertid);
            print_field(&mut out, "date", &a.date);
            print_field(&mut out, "location", &a.location);
            print_field(&mut out, "comment", &a.comment);
            print_field(&mut out, "group", &a.group);
            print_field(&mut out, "srcip", &a.srcip);
            writeln!(out, "srcport={}", a.srcport).unwrap();
            print_field(&mut out, "dstip", &a.dstip);
            writeln!(out, "dstport={}", a.dstport).unwrap();
            print_field(&mut out, "user", &a.user);
            print_field(&mut out, "filename", &a.filename);
        }
        None => {
            writeln!(out, "(null)").unwrap();
        }
    }
}
