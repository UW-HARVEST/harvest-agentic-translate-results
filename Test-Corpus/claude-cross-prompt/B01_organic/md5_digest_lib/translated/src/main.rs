// Translation of c_src/src/lib.c to Rust.
//
// The original C code is a shared library that exposes a single function,
// `md5_digest`, which serializes the four 32-bit fields of a `tflac_md5`
// struct into a 16-byte output buffer in little-endian order.
//
// Since the C code has no `main` function (it is a library), the binary
// produces no output. This Rust program mirrors that: an empty `main` and
// the translated function preserved for behavioral equivalence.

#![allow(dead_code)]

pub type TflacU8 = u8;
pub type TflacU32 = u32;

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct TflacMd5 {
    pub a: TflacU32,
    pub b: TflacU32,
    pub c: TflacU32,
    pub d: TflacU32,
}

pub fn md5_digest(m: &TflacMd5, out: &mut [TflacU8; 16]) {
    out[0] = m.a as TflacU8;
    out[1] = (m.a >> 8) as TflacU8;
    out[2] = (m.a >> 16) as TflacU8;
    out[3] = (m.a >> 24) as TflacU8;
    out[4] = m.b as TflacU8;
    out[5] = (m.b >> 8) as TflacU8;
    out[6] = (m.b >> 16) as TflacU8;
    out[7] = (m.b >> 24) as TflacU8;
    out[8] = m.c as TflacU8;
    out[9] = (m.c >> 8) as TflacU8;
    out[10] = (m.c >> 16) as TflacU8;
    out[11] = (m.c >> 24) as TflacU8;
    out[12] = m.d as TflacU8;
    out[13] = (m.d >> 8) as TflacU8;
    out[14] = (m.d >> 16) as TflacU8;
    out[15] = (m.d >> 24) as TflacU8;
}

fn main() {}
