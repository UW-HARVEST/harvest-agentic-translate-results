//! Faithful Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) whose only
//! exported (public) symbol is `ima_parse`.  Everything else in the C source is
//! `static` (internal linkage) and therefore not part of the ABI, but it is
//! translated here as well so that the observable behaviour is identical.
//!
//! Layout notes (x86-64 / LP64 System V, matching the C compiler):
//!
//! ```text
//! struct caf_header            size  8, align 4   type@0  version@4  flags@6
//! struct caf_chunk             size 16, align 8   type@0  size@8      (4 bytes padding @4)
//! struct caf_audio_description size 32, align 8   sample_rate@0 format_id@8 format_flags@12
//!                                                 bytes_per_packet@16 frames_per_packet@20
//!                                                 channels_per_frame@24 bits_per_channel@28
//! struct caf_packet_table      size 24, align 8   packet_count@0 frame_count@8
//!                                                 priming_frames@16 remainder_frames@20
//! struct caf_data              size  4, align 4   edit_count@0
//! struct ima_block             size 34, align 2   preamble@0 data@2
//! struct ima_info              size 40, align 8   blocks@0 size@8 sample_rate@16
//!                                                 frame_count@24 channel_count@32
//! ```
//!
//! The C code contains a number of oddities (a chunk header that is 16 bytes
//! instead of the 12 bytes the CAF format actually uses because of struct
//! padding, an unguarded infinite chunk scan, unchecked `desc`/`pakt` NULL
//! pointers, and a `double` -> `unsigned long long` *value* conversion of the
//! raw sample-rate bits before the byte swap).  None of these are "fixed"
//! here: they are reproduced bit for bit.

#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;

/// `struct ima_block` from `include/lib.h` (size 34, align 2).
#[repr(C)]
pub struct ima_block {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; 32],
}

/// `struct ima_info` from `include/lib.h` (size 40, align 8).
#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}

// Compile-time confirmation that the public structures have the exact layout
// the C compiler gives them (verified against `sizeof`/`offsetof` output of the
// original library).
const _: () = {
    assert!(core::mem::size_of::<ima_block>() == 34);
    assert!(core::mem::align_of::<ima_block>() == 2);
    assert!(core::mem::size_of::<ima_info>() == 40);
    assert!(core::mem::align_of::<ima_info>() == 8);
    assert!(core::mem::offset_of!(ima_info, blocks) == 0);
    assert!(core::mem::offset_of!(ima_info, size) == 8);
    assert!(core::mem::offset_of!(ima_info, sample_rate) == 16);
    assert!(core::mem::offset_of!(ima_info, frame_count) == 24);
    assert!(core::mem::offset_of!(ima_info, channel_count) == 32);
};

// ---------------------------------------------------------------------------
// src/lib.c -- private types
// ---------------------------------------------------------------------------

pub type ima_s32_t = i32;
pub type ima_s64_t = i64;

// Sizes of the private CAF structures, including the tail padding the C
// compiler inserts.  These are what the pointer arithmetic in `ima_parse`
// (`&header[1]`, `&chunk[1]`, ...) is scaled by.
const SIZEOF_CAF_HEADER: usize = 8;
const SIZEOF_CAF_CHUNK: usize = 16;
const SIZEOF_CAF_DATA: usize = 4;

// Field offsets within the private CAF structures.
const CAF_HEADER_TYPE: usize = 0;
const CAF_HEADER_VERSION: usize = 4;

const CAF_CHUNK_TYPE: usize = 0;
const CAF_CHUNK_SIZE: usize = 8;

const CAF_DESC_SAMPLE_RATE: usize = 0;
const CAF_DESC_FORMAT_ID: usize = 8;
const CAF_DESC_CHANNELS_PER_FRAME: usize = 24;

const CAF_PAKT_FRAME_COUNT: usize = 8;

// FourCC helper mirroring the open-coded expressions in the C source:
//   ((ima_u32_t)(ima_u8_t)(a) | ((ima_u32_t)(ima_u8_t)(b) << 8) |
//    ((ima_u32_t)(ima_u8_t)(c) << 16) | ((ima_u32_t)(ima_u8_t)(d) << 24))
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t) | ((b as ima_u32_t) << 8) | ((c as ima_u32_t) << 16) | ((d as ima_u32_t) << 24)
}

// `'f' | 'f' << 8 | 'a' << 16 | 'c' << 24` == 0x63616666
const CAF_TYPE_CAFF: ima_u32_t = fourcc(b'f', b'f', b'a', b'c');
// `'c' | 's' << 8 | 'e' << 16 | 'd' << 24` == 0x64657363
const CAF_TYPE_DESC: ima_u32_t = fourcc(b'c', b's', b'e', b'd');
// `'t' | 'k' << 8 | 'a' << 16 | 'p' << 24` == 0x70616b74
const CAF_TYPE_PAKT: ima_u32_t = fourcc(b't', b'k', b'a', b'p');
// `'a' | 't' << 8 | 'a' << 16 | 'd' << 24` == 0x64617461
const CAF_TYPE_DATA: ima_u32_t = fourcc(b'a', b't', b'a', b'd');
// `'4' | 'a' << 8 | 'm' << 16 | 'i' << 24` == 0x696d6134
const CAF_FORMAT_IMA4: ima_u32_t = fourcc(b'4', b'a', b'm', b'i');

// ---------------------------------------------------------------------------
// src/lib.c -- static byte-swap helpers
// ---------------------------------------------------------------------------

/// `static ima_u16_t ima_bswap16(ima_u16_t v)`
#[inline]
fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    ((v as u32) << 0x08 & 0xff00u32) as ima_u16_t | (v >> 0x08 & 0x00ffu16)
}

/// `static ima_u32_t ima_bswap32(ima_u32_t v)`
#[inline]
fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    (v << 0x18 & 0xff000000u32)
        | (v << 0x08 & 0x00ff0000u32)
        | (v >> 0x08 & 0x0000ff00u32)
        | (v >> 0x18 & 0x000000ffu32)
}

/// `static ima_u64_t ima_bswap64(ima_u64_t v)`
#[inline]
fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    (v << 0x38 & 0xff00000000000000u64)
        | (v << 0x28 & 0x00ff000000000000u64)
        | (v << 0x18 & 0x0000ff0000000000u64)
        | (v << 0x08 & 0x000000ff00000000u64)
        | (v >> 0x08 & 0x00000000ff000000u64)
        | (v >> 0x18 & 0x0000000000ff0000u64)
        | (v >> 0x28 & 0x000000000000ff00u64)
        | (v >> 0x38 & 0x00000000000000ffu64)
}

/// `static ima_u16_t ima_btoh16(ima_u16_t v)` -- big-endian to host.
///
/// The C code always byte-swaps (it is only correct on little-endian hosts);
/// that unconditional swap is preserved here.
#[inline]
fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

/// `static ima_u32_t ima_btoh32(ima_u32_t v)`
#[inline]
fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    ima_bswap32(v)
}

/// `static ima_u64_t ima_btoh64(ima_u64_t v)`
#[inline]
fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    ima_bswap64(v)
}

// ---------------------------------------------------------------------------
// Helpers that are not present in the C source
// ---------------------------------------------------------------------------

// The C code reads the CAF structures through pointers that are cast straight
// from the caller's buffer, so the accesses may be misaligned.  These helpers
// perform the same loads without relying on alignment.

#[inline]
unsafe fn load_u16(base: *const u8, offset: usize) -> ima_u16_t {
    core::ptr::read_unaligned(base.add(offset) as *const ima_u16_t)
}

#[inline]
unsafe fn load_u32(base: *const u8, offset: usize) -> ima_u32_t {
    core::ptr::read_unaligned(base.add(offset) as *const ima_u32_t)
}

#[inline]
unsafe fn load_u64(base: *const u8, offset: usize) -> ima_u64_t {
    core::ptr::read_unaligned(base.add(offset) as *const ima_u64_t)
}

#[inline]
unsafe fn load_f64(base: *const u8, offset: usize) -> ima_f64_t {
    core::ptr::read_unaligned(base.add(offset) as *const ima_f64_t)
}

/// x86-64 `cvttsd2si` with a 64-bit destination: truncate toward zero, and
/// yield the "integer indefinite" value `0x8000000000000000` whenever the
/// result is not representable (NaN, infinity, out-of-range magnitude).
#[inline]
fn cvttsd2si64(x: ima_f64_t) -> ima_s64_t {
    if x.is_nan() {
        return ima_s64_t::MIN;
    }
    let t = x.trunc();
    // -2^63 is exactly representable as a double; +2^63 is not in range.
    if t >= -9223372036854775808.0 && t < 9223372036854775808.0 {
        t as ima_s64_t
    } else {
        ima_s64_t::MIN
    }
}

/// The C expression `(ima_u64_t)some_double`, as implemented by the C compiler
/// on x86-64:
///
/// ```text
///     comisd xmm0, 2^63
///     jae    .big              ; taken only when the compare is ordered and x >= 2^63
///     cvttsd2si rax, xmm0
///     ...
/// .big:
///     subsd  xmm0, 2^63
///     cvttsd2si rax, xmm0
///     btc    rax, 63
/// ```
///
/// Rust's `as` cast saturates instead, so the hardware behaviour (which is
/// what the C library exhibits for out-of-range and negative values) has to be
/// spelled out.
#[inline]
fn double_to_u64(x: ima_f64_t) -> ima_u64_t {
    const TWO_POW_63: ima_f64_t = 9223372036854775808.0;
    if x >= TWO_POW_63 {
        // `x` is ordered and >= 2^63 here (a NaN compares false).
        (cvttsd2si64(x - TWO_POW_63) as ima_u64_t) ^ (1u64 << 63)
    } else {
        cvttsd2si64(x) as ima_u64_t
    }
}

// ---------------------------------------------------------------------------
// src/lib.c -- public entry point
// ---------------------------------------------------------------------------

/// `int ima_parse(struct ima_info *info, const void *data)`
// `blocks`/`desc`/`pakt` are initialised to NULL exactly as in the C source,
// even though the NULL value of `blocks` can never be observed.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header: *const u8 = data as *const u8;
    // const struct caf_chunk *chunk = (const struct caf_chunk *)&header[1];
    let mut chunk: *const u8 = header.wrapping_add(SIZEOF_CAF_HEADER);
    let mut desc: *const u8 = core::ptr::null();
    let mut pakt: *const u8 = core::ptr::null();
    let mut blocks: *const ima_block = core::ptr::null();
    // union { ima_f64_t f; ima_u64_t u; } conv64;
    let mut conv64_u: ima_u64_t;
    let mut chunk_size: ima_s64_t;
    let mut chunk_type: ima_u32_t;

    if ima_btoh32(load_u32(header, CAF_HEADER_TYPE)) != CAF_TYPE_CAFF {
        return -1;
    }
    if ima_btoh16(load_u16(header, CAF_HEADER_VERSION)) != 1 {
        return -2;
    }
    loop {
        chunk_type = ima_btoh32(load_u32(chunk, CAF_CHUNK_TYPE));
        chunk_size = ima_btoh64(load_u64(chunk, CAF_CHUNK_SIZE)) as ima_s64_t;
        if chunk_type == CAF_TYPE_DESC {
            // desc = (const struct caf_audio_description *)&chunk[1];
            desc = chunk.wrapping_add(SIZEOF_CAF_CHUNK);
        } else if chunk_type == CAF_TYPE_PAKT {
            // pakt = (const struct caf_packet_table *)&chunk[1];
            pakt = chunk.wrapping_add(SIZEOF_CAF_CHUNK);
        } else if chunk_type == CAF_TYPE_DATA {
            // blocks = (const struct ima_block *)&((const struct caf_data *)&chunk[1])[1];
            blocks = chunk
                .wrapping_add(SIZEOF_CAF_CHUNK)
                .wrapping_add(SIZEOF_CAF_DATA) as *const ima_block;
            break;
        }
        // chunk = (const struct caf_chunk *)((const ima_u8_t *)&chunk[1] + chunk_size);
        chunk = chunk
            .wrapping_add(SIZEOF_CAF_CHUNK)
            .wrapping_offset(chunk_size as isize);
    }
    if ima_btoh32(load_u32(desc, CAF_DESC_FORMAT_ID)) != CAF_FORMAT_IMA4 {
        return -3;
    }
    (*info).blocks = blocks;
    (*info).size = chunk_size as ima_u64_t;
    (*info).frame_count = ima_btoh64(load_u64(pakt, CAF_PAKT_FRAME_COUNT));
    (*info).channel_count = ima_btoh32(load_u32(desc, CAF_DESC_CHANNELS_PER_FRAME));
    // conv64.u = desc->sample_rate;  <-- floating point *value* conversion
    conv64_u = double_to_u64(load_f64(desc, CAF_DESC_SAMPLE_RATE));
    // conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);
    conv64_u = ima_btoh64(conv64_u);
    // info->sample_rate = conv64.f;  <-- read back through the other member
    (*info).sample_rate = ima_f64_t::from_bits(conv64_u);
    0
}
