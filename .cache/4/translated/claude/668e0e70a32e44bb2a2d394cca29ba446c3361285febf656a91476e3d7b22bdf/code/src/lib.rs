//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `ima_parse`
//!
//! The translation reproduces the original behaviour bit-for-bit, including the
//! quirks/bugs of the C implementation (see `sample_rate` handling in
//! [`ima_parse`]). Nothing is "fixed".

#![allow(non_camel_case_types)]

mod caf;
mod endian;
mod parse;

pub use parse::ima_parse;

// ---------------------------------------------------------------------------
// Public types, mirroring `c_src/include/lib.h`
// ---------------------------------------------------------------------------

/// `typedef unsigned int ima_u32_t;`
pub type ima_u32_t = u32;
/// `typedef unsigned long long ima_u64_t;`
pub type ima_u64_t = u64;
/// `typedef double ima_f64_t;`
pub type ima_f64_t = f64;
/// `typedef unsigned char ima_u8_t;`
pub type ima_u8_t = u8;
/// `typedef unsigned short ima_u16_t;`
pub type ima_u16_t = u16;

/// `typedef signed int ima_s32_t;` (private to `src/lib.c`)
pub type ima_s32_t = i32;
/// `typedef signed long long ima_s64_t;` (private to `src/lib.c`)
pub type ima_s64_t = i64;

/// Number of payload bytes in an IMA block (`(32)` in the C header).
pub const IMA_BLOCK_DATA_LEN: usize = 32;

/// ```c
/// struct ima_block {
///     ima_u16_t preamble;
///     ima_u8_t data[(32)];
/// };
/// ```
///
/// Layout: `preamble` @ 0, `data` @ 2, size 34, align 2.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ima_block {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; IMA_BLOCK_DATA_LEN],
}

/// ```c
/// struct ima_info {
///     const struct ima_block *blocks;
///     ima_u64_t size;
///     ima_f64_t sample_rate;
///     ima_u64_t frame_count;
///     ima_u32_t channel_count;
/// };
/// ```
///
/// Layout: `blocks` @ 0, `size` @ 8, `sample_rate` @ 16, `frame_count` @ 24,
/// `channel_count` @ 32, size 40, align 8.
#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn public_struct_layout_matches_c() {
        assert_eq!(size_of::<ima_block>(), 34);
        assert_eq!(align_of::<ima_block>(), 2);
        assert_eq!(size_of::<ima_info>(), 40);
        assert_eq!(align_of::<ima_info>(), 8);
    }
}
