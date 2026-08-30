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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C code builds a zero-initialized `house_t`, fills in its three fields,
//! `memcpy`s the struct into a raw `char` buffer, and hex-dumps that buffer.
//! The observable output is therefore the little-endian in-memory image of the
//! struct, including any padding bytes (which `= {0}` clears).

use std::ffi::c_int;
use std::io::Write;
use std::mem::{offset_of, size_of};

/// Mirrors:
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// `printf("%02x", p[i])` for each byte, then a single newline.
fn print_hex(p: &[u8]) {
    // len is `int` in C; the buffer here is always well within that range.
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for &byte in p {
        // "%02x": lowercase, zero padded to two digits.
        out.push_str(&format!("{byte:02x}"));
    }
    out.push('\n');

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    // Ignore write errors, as C's printf return value is ignored here.
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

/// `void driver(int floors)`
///
/// The header declares the symbol plainly as `driver` (no namespacing macros),
/// so the exported linker symbol is `driver`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // `house_t house = {0};` -- every byte, padding included, starts as zero.
    let mut raw = [0u8; size_of::<House>()];

    // house.floors = floors;
    write_bytes(&mut raw, offset_of!(House, floors), &floors.to_ne_bytes());
    // house.bedrooms = 3;
    write_bytes(
        &mut raw,
        offset_of!(House, bedrooms),
        &(3 as c_int).to_ne_bytes(),
    );
    // house.bathrooms = 2.;
    write_bytes(
        &mut raw,
        offset_of!(House, bathrooms),
        &(2.0f64).to_ne_bytes(),
    );

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    // print_hex((unsigned char *)&raw, sizeof(raw));
    print_hex(&raw);
}

/// Copy `src` into `buf` at `offset`, emulating the field stores that the C
/// compiler performs into the struct's memory image.
fn write_bytes(buf: &mut [u8], offset: usize, src: &[u8]) {
    buf[offset..offset + src.len()].copy_from_slice(src);
}
