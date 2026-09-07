//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `tflac_size_memory`
//!   * `flac_validate`
//!
//! Behaviour is reproduced exactly, including the original order of validation
//! checks and C wrapping-arithmetic semantics.

#![allow(non_camel_case_types)]

use core::ffi::c_int;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

/// Mirrors `struct tflac` from `include/lib.h`.
///
/// Layout (x86-64 / LP64): four `u32` at offsets 0/4/8/12, five `u8` at
/// offsets 16..=20, three bytes of tail padding, then `cur_blocksize` at
/// offset 24; total size 28, alignment 4.
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

// Lock the ABI layout down at compile time (verified against the C build:
// sizeof(tflac) == 28, offsetof(tflac, cur_blocksize) == 24).
const _: () = {
    assert!(core::mem::size_of::<tflac>() == 28);
    assert!(core::mem::align_of::<tflac>() == 4);
    assert!(core::mem::offset_of!(tflac, blocksize) == 0);
    assert!(core::mem::offset_of!(tflac, samplerate) == 4);
    assert!(core::mem::offset_of!(tflac, channels) == 8);
    assert!(core::mem::offset_of!(tflac, bitdepth) == 12);
    assert!(core::mem::offset_of!(tflac, channel_mode) == 16);
    assert!(core::mem::offset_of!(tflac, max_rice_value) == 17);
    assert!(core::mem::offset_of!(tflac, min_partition_order) == 18);
    assert!(core::mem::offset_of!(tflac, max_partition_order) == 19);
    assert!(core::mem::offset_of!(tflac, partition_order) == 20);
    assert!(core::mem::offset_of!(tflac, cur_blocksize) == 24);
};

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
/// All arithmetic is unsigned 32-bit and therefore wraps, matching C.
#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(15u32.wrapping_add(blocksize.wrapping_mul(4)) & 0xFFFF_FFF0u32),
    )
}

/// `int flac_validate(tflac *t)`
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
    // `1 << (partition_order + 1)`: partition_order <= max_partition_order <= 15
    // here, so the shift amount is at most 16 and the divisor never overflows.
    while (t.blocksize % (1u32 << (t.partition_order as u32 + 1)) == 0)
        && t.partition_order < t.max_partition_order
    {
        t.partition_order += 1;
    }
    t.cur_blocksize = t.blocksize;
    0
}
