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

impl ima_info {
    /// Field offsets, confirmed against the C codegen (`objdump -d`): the stores
    /// are `mov %rdx,(%rax)`, `mov %rdx,0x8(%rax)`, `movsd %xmm0,0x10(%rax)`,
    /// `mov %rax,0x18(%rdx)` and `mov %eax,0x20(%rdx)`.
    pub const OFF_BLOCKS: usize = 0;
    pub const OFF_SIZE: usize = 8;
    pub const OFF_SAMPLE_RATE: usize = 16;
    pub const OFF_FRAME_COUNT: usize = 24;
    pub const OFF_CHANNEL_COUNT: usize = 32;
    /// Total size, including the 4 tail padding bytes that no store touches.
    pub const SIZE: usize = 40;
}

// The offsets `ima_parse` stores through must agree with the `repr(C)` layout.
// Checked at compile time so the two can never drift apart.
const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(ima_info, blocks) == ima_info::OFF_BLOCKS);
    assert!(offset_of!(ima_info, size) == ima_info::OFF_SIZE);
    assert!(offset_of!(ima_info, sample_rate) == ima_info::OFF_SAMPLE_RATE);
    assert!(offset_of!(ima_info, frame_count) == ima_info::OFF_FRAME_COUNT);
    assert!(offset_of!(ima_info, channel_count) == ima_info::OFF_CHANNEL_COUNT);
    assert!(core::mem::size_of::<ima_info>() == ima_info::SIZE);
};

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
