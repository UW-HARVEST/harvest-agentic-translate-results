use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64-encode a slice. Mirrors the reference C implementation, including
/// the use of `'='` characters as padding when `len` is not a multiple of 3.
pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];

    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < len {
        let octet_a: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;
        let octet_b: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;
        let octet_c: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        output[j] = BASE64_TABLE[((triple >> 18) & 0x3F) as usize];
        output[j + 1] = BASE64_TABLE[((triple >> 12) & 0x3F) as usize];
        output[j + 2] = BASE64_TABLE[((triple >> 6) & 0x3F) as usize];
        output[j + 3] = BASE64_TABLE[(triple & 0x3F) as usize];
        j += 4;
    }

    let pad_count = (3 - (len % 3)) % 3;
    for _ in 0..pad_count {
        j -= 1;
        output[j] = b'=';
    }

    // The C function returns a NUL-terminated C string; in Rust we return a
    // String containing only the base64-encoded bytes.
    String::from_utf8(output).expect("base64 alphabet is ASCII")
}

/// Multiply two bytes in GF(2^8) — same semantics as `cipher_utils::g_mult`.
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// Lane-wise GF(2^8) multiplication of two 16-byte vectors. The SSE-using
/// signature is preserved from the reference header; we implement it by
/// transferring the lanes through arrays so the heavy lifting is done with
/// safe Rust.
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // SAFETY: `__m128i` is plain 128-bit data; transmuting between it and
    // `[u8; 16]` is well-defined because both have the same size and
    // alignment representation in the standard library.
    let a: [u8; 16] = unsafe { std::mem::transmute(first) };
    let b: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(a[i], b[i]);
    }
    unsafe { std::mem::transmute(out) }
}

/// SSE-style version of MixColumns that produces the same result as
/// `matrix::columns`.
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    crate::matrix::columns(state);
}
