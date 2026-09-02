//! Rust translation of the C `ima` library (`c_src/`).
//!
//! The C library consists of a single translation unit (`src/lib.c`) with a
//! single public header (`include/lib.h`). It exports exactly one public
//! symbol: `ima_parse`. Everything else in the C source is `static` (internal
//! linkage) and therefore not part of the ABI, but is reproduced here because
//! `ima_parse` depends on it.
//!
//! Behaviour is reproduced bit-for-bit, including the quirks of the original:
//!
//! * `struct caf_*` layouts (and hence the pointer arithmetic that walks the
//!   chunk list) follow the C ABI exactly.
//! * The `desc`/`pakt` pointers may still be NULL when the `data` chunk is
//!   reached; the C code dereferences them unconditionally afterwards. That is
//!   reproduced (it faults, just like the C).
//! * `conv64.u = desc->sample_rate;` in the C is an *arithmetic* conversion
//!   from `double` to `unsigned long long` (not a bit reinterpretation). The
//!   result is then byte-swapped and finally read back as a `double`. This is
//!   reproduced faithfully, including the out-of-range / NaN behaviour of the
//!   x86-64 code GCC generates for that conversion.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;

/// `struct ima_block` — size 34, align 2.
#[repr(C)]
pub struct ima_block {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; 32],
}

/// `struct ima_info` — size 40, align 8.
#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}

// ---------------------------------------------------------------------------
// src/lib.c — private types
// ---------------------------------------------------------------------------

type ima_s32_t = i32;
type ima_s64_t = i64;

/// `struct caf_header` — size 8, align 4.
#[repr(C)]
struct caf_header {
    r#type: ima_u32_t,
    version: ima_u16_t,
    #[allow(dead_code)]
    flags: ima_u16_t,
}

/// `struct caf_chunk` — size 16, align 8 (`size` at offset 8).
#[repr(C)]
struct caf_chunk {
    r#type: ima_u32_t,
    size: ima_s64_t,
}

/// `struct caf_audio_description` — size 32, align 8.
#[repr(C)]
struct caf_audio_description {
    sample_rate: ima_f64_t,
    format_id: ima_u32_t,
    #[allow(dead_code)]
    format_flags: ima_u32_t,
    #[allow(dead_code)]
    bytes_per_packet: ima_u32_t,
    #[allow(dead_code)]
    frames_per_packet: ima_u32_t,
    channels_per_frame: ima_u32_t,
    #[allow(dead_code)]
    bits_per_channel: ima_u32_t,
}

/// `struct caf_packet_table` — size 24, align 8.
#[repr(C)]
struct caf_packet_table {
    #[allow(dead_code)]
    packet_count: ima_s64_t,
    frame_count: ima_s64_t,
    #[allow(dead_code)]
    priming_frames: ima_s32_t,
    #[allow(dead_code)]
    remainder_frames: ima_s32_t,
}

/// `struct caf_data` — size 4, align 4.
#[repr(C)]
struct caf_data {
    #[allow(dead_code)]
    edit_count: ima_u32_t,
}

// Layout assertions mirroring the C ABI (verified against the C compiler).
const _: () = {
    use core::mem::{align_of, size_of};
    assert!(size_of::<caf_header>() == 8 && align_of::<caf_header>() == 4);
    assert!(size_of::<caf_chunk>() == 16 && align_of::<caf_chunk>() == 8);
    assert!(size_of::<caf_audio_description>() == 32 && align_of::<caf_audio_description>() == 8);
    assert!(size_of::<caf_packet_table>() == 24 && align_of::<caf_packet_table>() == 8);
    assert!(size_of::<caf_data>() == 4 && align_of::<caf_data>() == 4);
    assert!(size_of::<ima_block>() == 34 && align_of::<ima_block>() == 2);
    assert!(size_of::<ima_info>() == 40 && align_of::<ima_info>() == 8);
};

// ---------------------------------------------------------------------------
// Byte-swap helpers (`static` in the C — reproduced verbatim)
// ---------------------------------------------------------------------------

#[inline]
fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    (v << 0x08 & 0xff00u16) | (v >> 0x08 & 0x00ffu16)
}

#[inline]
fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    (v << 0x18 & 0xff000000u32)
        | (v << 0x08 & 0x00ff0000u32)
        | (v >> 0x08 & 0x0000ff00u32)
        | (v >> 0x18 & 0x000000ffu32)
}

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

#[inline]
fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

#[inline]
fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    ima_bswap32(v)
}

#[inline]
fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    ima_bswap64(v)
}

// ---------------------------------------------------------------------------
// FourCC constants, spelled exactly as in the C source
// ---------------------------------------------------------------------------

#[inline]
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t) | ((b as ima_u32_t) << 8) | ((c as ima_u32_t) << 16) | ((d as ima_u32_t) << 24)
}

/// `'f' | 'f' << 8 | 'a' << 16 | 'c' << 24`
const CAF_HEADER_TYPE: ima_u32_t = fourcc(b'f', b'f', b'a', b'c');
/// `'c' | 's' << 8 | 'e' << 16 | 'd' << 24`
const CAF_CHUNK_DESC: ima_u32_t = fourcc(b'c', b's', b'e', b'd');
/// `'t' | 'k' << 8 | 'a' << 16 | 'p' << 24`
const CAF_CHUNK_PAKT: ima_u32_t = fourcc(b't', b'k', b'a', b'p');
/// `'a' | 't' << 8 | 'a' << 16 | 'd' << 24`
const CAF_CHUNK_DATA: ima_u32_t = fourcc(b'a', b't', b'a', b'd');
/// `'4' | 'a' << 8 | 'm' << 16 | 'i' << 24`
const CAF_FORMAT_IMA4: ima_u32_t = fourcc(b'4', b'a', b'm', b'i');

// ---------------------------------------------------------------------------
// `double` -> `unsigned long long` conversion, x86-64 semantics
// ---------------------------------------------------------------------------

/// `cvttsd2si` (64-bit form): truncate toward zero; produce the "integer
/// indefinite" value `0x8000_0000_0000_0000` for NaN, infinities and any value
/// outside the signed 64-bit range.
#[inline]
fn cvttsd2si64(x: ima_f64_t) -> ima_s64_t {
    const MIN: ima_f64_t = -9223372036854775808.0; // -2^63, exactly representable
    const LIMIT: ima_f64_t = 9223372036854775808.0; //  2^63, exactly representable
    if x.is_nan() {
        return ima_s64_t::MIN;
    }
    let t = x.trunc();
    if t >= LIMIT || t < MIN {
        return ima_s64_t::MIN;
    }
    t as ima_s64_t
}

/// The C expression `(ima_u64_t)some_double`.
///
/// This is *not* a bit reinterpretation. GCC/Clang emit, on x86-64:
///
/// ```text
///     comisd 2^63, x        ; unordered (NaN) leaves CF set
///     jae    big
///     cvttsd2si x -> rax
///     jmp    done
/// big:
///     subsd  2^63, x
///     cvttsd2si x -> rax
///     xor    0x8000000000000000, rax
/// done:
/// ```
///
/// which is reproduced here verbatim, including the out-of-range results the
/// C standard leaves undefined.
#[inline]
fn double_to_u64(x: ima_f64_t) -> ima_u64_t {
    const TWO63: ima_f64_t = 9223372036854775808.0;
    // `jae` is taken only when the comparison is ordered and x >= 2^63.
    if x >= TWO63 {
        (cvttsd2si64(x - TWO63) as ima_u64_t) ^ 0x8000_0000_0000_0000u64
    } else {
        cvttsd2si64(x) as ima_u64_t
    }
}

// ---------------------------------------------------------------------------
// Raw, unaligned field reads. x86-64 loads tolerate misalignment, so the C
// compiles to plain loads; `read_unaligned` matches that byte for byte.
// ---------------------------------------------------------------------------

#[inline]
const fn byte_add<T>(p: *const T, n: usize) -> *const u8 {
    (p as *const u8).wrapping_add(n)
}

#[inline]
unsafe fn load<T>(p: *const u8) -> T {
    (p as *const T).read_unaligned()
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// `int ima_parse(struct ima_info *info, const void *data);`
#[unsafe(no_mangle)]
#[allow(unused_assignments)] // the NULL initialisers mirror the C source
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    // const struct caf_header *header = (const struct caf_header *)data;
    let header: *const caf_header = data as *const caf_header;
    // const struct caf_chunk *chunk = (const struct caf_chunk *)&header[1];
    let mut chunk: *const caf_chunk =
        byte_add(header, core::mem::size_of::<caf_header>()) as *const caf_chunk;
    let mut desc: *const caf_audio_description = core::ptr::null();
    let mut pakt: *const caf_packet_table = core::ptr::null();
    let mut blocks: *const ima_block = core::ptr::null();

    let chunk_size: ima_s64_t;
    let mut chunk_type: core::ffi::c_uint;

    // if (ima_btoh32(header->type) != 'caff') return -1;
    if ima_btoh32(load::<ima_u32_t>(byte_add(header, 0))) != CAF_HEADER_TYPE {
        return -1;
    }
    // if (ima_btoh16(header->version) != 1) return -2;
    if ima_btoh16(load::<ima_u16_t>(byte_add(header, 4))) != 1 {
        return -2;
    }

    loop {
        chunk_type = ima_btoh32(load::<ima_u32_t>(byte_add(chunk, 0)));
        let size = ima_btoh64(load::<ima_u64_t>(byte_add(chunk, 8))) as ima_s64_t;

        if chunk_type == CAF_CHUNK_DESC {
            desc = byte_add(chunk, 16) as *const caf_audio_description;
        } else if chunk_type == CAF_CHUNK_PAKT {
            pakt = byte_add(chunk, 16) as *const caf_packet_table;
        } else if chunk_type == CAF_CHUNK_DATA {
            // blocks = &((const struct caf_data *)&chunk[1])[1];
            blocks = byte_add(chunk, 16 + core::mem::size_of::<caf_data>()) as *const ima_block;
            chunk_size = size;
            break;
        }

        // chunk = (const struct caf_chunk *)((const ima_u8_t *)&chunk[1] + chunk_size);
        chunk = byte_add(chunk, 16).wrapping_offset(size as isize) as *const caf_chunk;
    }

    // if (ima_btoh32(desc->format_id) != 'ima4') return -3;
    if ima_btoh32(load::<ima_u32_t>(byte_add(desc, 8))) != CAF_FORMAT_IMA4 {
        return -3;
    }

    // info->blocks = blocks;
    (*info).blocks = blocks;
    // info->size = chunk_size;
    (*info).size = chunk_size as ima_u64_t;
    // info->frame_count = ima_btoh64(pakt->frame_count);
    (*info).frame_count = ima_btoh64(load::<ima_u64_t>(byte_add(pakt, 8)));
    // info->channel_count = ima_btoh32(desc->channels_per_frame);
    (*info).channel_count = ima_btoh32(load::<ima_u32_t>(byte_add(desc, 24)));

    // conv64.u = desc->sample_rate;                              /* double -> u64 */
    // conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);      /* byte swap     */
    // info->sample_rate = conv64.f;                              /* u64 bits -> double */
    let raw = load::<ima_f64_t>(byte_add(desc, 0));
    let u = ima_btoh64(double_to_u64(raw));
    (*info).sample_rate = ima_f64_t::from_bits(u);

    0
}
