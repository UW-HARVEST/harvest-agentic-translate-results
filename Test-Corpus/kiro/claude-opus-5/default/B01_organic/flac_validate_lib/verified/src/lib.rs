//! Rust translation of `c_src/src/lib.c` / `c_src/include/lib.h`.
//!
//! Behaviour is intentionally identical to the C original, including its
//! quirks (e.g. the `partition_order` loop that checks the modulo before the
//! upper bound, and the wrapping arithmetic in `tflac_size_memory`).

#![allow(non_camel_case_types)]

use std::ffi::c_int;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

/// Mirrors `struct tflac` from `include/lib.h`.
#[repr(C)]
pub struct tflac {
    pub blocksize: tflac_u32,
    pub samplerate: tflac_u32,
    pub channels: tflac_u32,
    pub bitdepth: tflac_u32,
    pub channel_mode: tflac_u8,
    pub max_rice_value: tflac_u8,
    pub min_partition_order: tflac_u8,
    pub max_partition_order: tflac_u8,
    pub partition_order: tflac_u8,
    pub cur_blocksize: tflac_u32,
}

// enum TFLAC_CHANNEL_MODE
const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;
#[allow(dead_code)]
const TFLAC_CHANNEL_LEFT_SIDE: tflac_u8 = 1;
#[allow(dead_code)]
const TFLAC_CHANNEL_SIDE_RIGHT: tflac_u8 = 2;
#[allow(dead_code)]
const TFLAC_CHANNEL_MID_SIDE: tflac_u8 = 3;
#[allow(dead_code)]
const TFLAC_CHANNEL_MODE_COUNT: tflac_u8 = 4;

/// `tflac_u32 tflac_size_memory(tflac_u32 blocksize)`
///
/// The C version relies on unsigned wraparound, so every operation here is
/// explicitly wrapping.
#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(15u32.wrapping_add(blocksize.wrapping_mul(4)) & 0xFFFF_FFF0u32),
    )
}

/// `int flac_validate(tflac *t)`
///
/// Validation checks are performed in exactly the same order as the C code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut tflac) -> c_int {
    let t = unsafe { &mut *t };

    if t.blocksize < 16 {
        return -1;
    }
    if t.blocksize > 65535 {
        return -1;
    }
    if t.samplerate == 0 {
        return -1;
    }
    if t.samplerate > 655350 {
        return -1;
    }
    if t.channels == 0 {
        return -1;
    }
    if t.channels > 8 {
        return -1;
    }
    if t.bitdepth == 0 {
        return -1;
    }
    if t.bitdepth > 32 {
        return -1;
    }
    if t.channel_mode != TFLAC_CHANNEL_INDEPENDENT
        && (t.channels != 2 || t.bitdepth == 32)
    {
        t.channel_mode = TFLAC_CHANNEL_INDEPENDENT;
    }
    if t.max_rice_value == 0 {
        if t.bitdepth <= 16 {
            t.max_rice_value = 14;
        } else {
            t.max_rice_value = 30;
        }
    } else if t.max_rice_value > 30 {
        return -1;
    }
    if t.max_partition_order > 15 {
        return -1;
    }
    if t.min_partition_order > t.max_partition_order {
        return -1;
    }
    t.partition_order = t.min_partition_order;
    // `1 << (partition_order + 1)` is an `int` shift in C; partition_order is
    // bounded by max_partition_order <= 15 here, so the shift stays in range.
    while (t.blocksize % (1u32 << (t.partition_order as u32 + 1))) == 0
        && t.partition_order < t.max_partition_order
    {
        t.partition_order += 1;
    }
    t.cur_blocksize = t.blocksize;
    0
}
