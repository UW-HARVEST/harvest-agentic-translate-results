use crate::aes::NB;
use crate::cipher_utils::g_mult;
use crate::matrix::columns;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output: Vec<u8> = Vec::with_capacity(output_len);

    let mut i = 0usize;
    while i < len {
        let octet_a = input[i] as u32;
        i += 1;
        let octet_b = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };
        let octet_c = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        output.push(BASE64_TABLE[((triple >> 18) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 12) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 6) & 0x3F) as usize]);
        output.push(BASE64_TABLE[(triple & 0x3F) as usize]);
    }

    // Replace trailing chars with '=' for padding.
    let pad_count = (3 - (len % 3)) % 3;
    let out_total = output.len();
    for k in 0..pad_count {
        output[out_total - 1 - k] = b'=';
    }

    // Safety: BASE64_TABLE and '=' are all valid ASCII.
    String::from_utf8(output).expect("base64 output is ASCII")
}

/// GF(2^8) multiplication for a single byte. Mirrors the C `g_mult_sse_byte`
/// which is declared in the header; no reference C implementation is provided,
/// so we use the standard Galois-field multiplication algorithm.
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// Vectorised GF(2^8) multiplication of corresponding lanes.
/// We implement this in pure Rust without `unsafe` by extracting the 16
/// lanes from the `__m128i` arguments, multiplying each pair, and packing
/// the results back. Since `__m128i` itself is opaque outside `unsafe`
/// blocks, we rely on safe transmutation of arrays of equal size.
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // SAFETY: __m128i is a 16-byte SIMD type with the same size and
    // alignment as [u8; 16]; transmuting between them is well-defined.
    let a: [u8; 16] = unsafe { std::mem::transmute(first) };
    let b: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(a[i], b[i]);
    }
    unsafe { std::mem::transmute(out) }
}

/// SSE variant of MixColumns. The C header declares this function but no
/// implementation is supplied. We provide an equivalent that produces the
/// same result as the scalar `columns` routine.
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    columns(state);
}
