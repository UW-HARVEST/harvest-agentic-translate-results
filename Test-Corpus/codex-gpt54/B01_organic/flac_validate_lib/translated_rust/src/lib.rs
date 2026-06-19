#![allow(non_camel_case_types)]

use std::ffi::c_int;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

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

const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul((15u32.wrapping_add(blocksize.wrapping_mul(4))) & 0xFFFFFFF0),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut tflac) -> c_int {
    let t = &mut *t;

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
    if t.channel_mode != TFLAC_CHANNEL_INDEPENDENT {
        if t.channels != 2 || t.bitdepth == 32 {
            t.channel_mode = TFLAC_CHANNEL_INDEPENDENT;
        }
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
    while (t.blocksize % (1u32 << (u32::from(t.partition_order) + 1)) == 0)
        && t.partition_order < t.max_partition_order
    {
        t.partition_order = t.partition_order.wrapping_add(1);
    }
    t.cur_blocksize = t.blocksize;
    0
}
