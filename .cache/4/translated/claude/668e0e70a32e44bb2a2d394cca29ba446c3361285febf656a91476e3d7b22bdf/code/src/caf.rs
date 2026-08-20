//! CAF (Core Audio Format) container structures, translated from the
//! file-private declarations in `c_src/src/lib.c`.
//!
//! These structures are *not* part of the public ABI, but their exact C layout
//! is load bearing: `ima_parse` walks a raw byte buffer by casting pointers to
//! them, so every field offset and every structure size must match the C
//! compiler's layout for the x86-64 SysV ABI.
//!
//! The parser reads through `read_unaligned`, because the pointers it forms
//! come from arbitrary offsets inside the caller's buffer and are therefore not
//! guaranteed to satisfy the alignment the C `struct` types nominally require
//! (the C code has the same latent misalignment, which x86 tolerates).

// Some layout constants are documentation of the C layout and are only
// referenced from the tests.
#![allow(dead_code)]

use crate::{ima_f64_t, ima_s32_t, ima_s64_t, ima_u16_t, ima_u32_t};

/// ```c
/// struct caf_header {
///     ima_u32_t type;
///     ima_u16_t version;
///     ima_u16_t flags;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct caf_header {
    pub type_: ima_u32_t,
    pub version: ima_u16_t,
    pub flags: ima_u16_t,
}

impl caf_header {
    pub const SIZE: usize = 8;
    pub const OFF_TYPE: usize = 0;
    pub const OFF_VERSION: usize = 4;
}

/// ```c
/// struct caf_chunk {
///     ima_u32_t type;
///     ima_s64_t size;
/// };
/// ```
///
/// `size` is 8-byte aligned, so there are 4 bytes of padding after `type` and
/// the structure is 16 bytes long.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct caf_chunk {
    pub type_: ima_u32_t,
    pub size: ima_s64_t,
}

impl caf_chunk {
    pub const SIZE: usize = 16;
    pub const OFF_TYPE: usize = 0;
    pub const OFF_SIZE: usize = 8;
}

/// ```c
/// struct caf_audio_description {
///     ima_f64_t sample_rate;
///     ima_u32_t format_id;
///     ima_u32_t format_flags;
///     ima_u32_t bytes_per_packet;
///     ima_u32_t frames_per_packet;
///     ima_u32_t channels_per_frame;
///     ima_u32_t bits_per_channel;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct caf_audio_description {
    pub sample_rate: ima_f64_t,
    pub format_id: ima_u32_t,
    pub format_flags: ima_u32_t,
    pub bytes_per_packet: ima_u32_t,
    pub frames_per_packet: ima_u32_t,
    pub channels_per_frame: ima_u32_t,
    pub bits_per_channel: ima_u32_t,
}

impl caf_audio_description {
    pub const SIZE: usize = 32;
    pub const OFF_SAMPLE_RATE: usize = 0;
    pub const OFF_FORMAT_ID: usize = 8;
    pub const OFF_CHANNELS_PER_FRAME: usize = 24;
}

/// ```c
/// struct caf_packet_table {
///     ima_s64_t packet_count;
///     ima_s64_t frame_count;
///     ima_s32_t priming_frames;
///     ima_s32_t remainder_frames;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct caf_packet_table {
    pub packet_count: ima_s64_t,
    pub frame_count: ima_s64_t,
    pub priming_frames: ima_s32_t,
    pub remainder_frames: ima_s32_t,
}

impl caf_packet_table {
    pub const SIZE: usize = 24;
    pub const OFF_PACKET_COUNT: usize = 0;
    pub const OFF_FRAME_COUNT: usize = 8;
}

/// ```c
/// struct caf_data {
///     ima_u32_t edit_count;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct caf_data {
    pub edit_count: ima_u32_t,
}

impl caf_data {
    pub const SIZE: usize = 4;
}

// ---------------------------------------------------------------------------
// Four character codes
//
// The C source spells these out as
//     ((ima_u32_t)(ima_u8_t)('c') | ((ima_u32_t)(ima_u8_t)('a') << 8) | ...)
// i.e. the *first* character listed ends up in the least significant byte.
// The comparisons are made against the big-endian-to-host converted value read
// from the file, so the resulting constant reads naturally when written as a
// hex literal.
// ---------------------------------------------------------------------------

/// Builds the same value as the C `(u32)(u8)a | (u32)(u8)b << 8 | ...` idiom.
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t) | ((b as ima_u32_t) << 8) | ((c as ima_u32_t) << 16) | ((d as ima_u32_t) << 24)
}

/// `'f' | 'f' << 8 | 'a' << 16 | 'c' << 24` == `0x63616666` ("caff").
pub const CAF_TYPE_CAFF: ima_u32_t = fourcc(b'f', b'f', b'a', b'c');
/// `'c' | 's' << 8 | 'e' << 16 | 'd' << 24` == `0x64657363` ("desc").
pub const CAF_CHUNK_DESC: ima_u32_t = fourcc(b'c', b's', b'e', b'd');
/// `'t' | 'k' << 8 | 'a' << 16 | 'p' << 24` == `0x70616b74` ("pakt").
pub const CAF_CHUNK_PAKT: ima_u32_t = fourcc(b't', b'k', b'a', b'p');
/// `'a' | 't' << 8 | 'a' << 16 | 'd' << 24` == `0x64617461` ("data").
pub const CAF_CHUNK_DATA: ima_u32_t = fourcc(b'a', b't', b'a', b'd');
/// `'4' | 'a' << 8 | 'm' << 16 | 'i' << 24` == `0x696d6134` ("ima4").
pub const CAF_FORMAT_IMA4: ima_u32_t = fourcc(b'4', b'a', b'm', b'i');

/// The CAF header version the parser accepts.
pub const CAF_VERSION: ima_u16_t = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn private_struct_sizes_match_c() {
        assert_eq!(size_of::<caf_header>(), caf_header::SIZE);
        assert_eq!(size_of::<caf_chunk>(), caf_chunk::SIZE);
        assert_eq!(
            size_of::<caf_audio_description>(),
            caf_audio_description::SIZE
        );
        assert_eq!(size_of::<caf_packet_table>(), caf_packet_table::SIZE);
        assert_eq!(size_of::<caf_data>(), caf_data::SIZE);
    }

    #[test]
    fn fourcc_values_match_c() {
        assert_eq!(CAF_TYPE_CAFF, 0x6361_6666);
        assert_eq!(CAF_CHUNK_DESC, 0x6465_7363);
        assert_eq!(CAF_CHUNK_PAKT, 0x7061_6b74);
        assert_eq!(CAF_CHUNK_DATA, 0x6461_7461);
        assert_eq!(CAF_FORMAT_IMA4, 0x696d_6134);
    }
}
