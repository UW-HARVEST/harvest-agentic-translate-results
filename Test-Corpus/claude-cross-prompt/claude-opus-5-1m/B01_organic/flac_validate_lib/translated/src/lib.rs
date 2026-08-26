// Translation of c_src/src/lib.c and c_src/include/lib.h to Rust.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
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

// enum TFLAC_CHANNEL_MODE values
pub const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;
pub const TFLAC_CHANNEL_LEFT_SIDE: tflac_u8 = 1;
pub const TFLAC_CHANNEL_SIDE_RIGHT: tflac_u8 = 2;
pub const TFLAC_CHANNEL_MID_SIDE: tflac_u8 = 3;
pub const TFLAC_CHANNEL_MODE_COUNT: tflac_u8 = 4;

pub fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(15u32.wrapping_add(blocksize.wrapping_mul(4u32)) & 0xFFFFFFF0u32),
    )
}

pub fn flac_validate(t: &mut tflac) -> i32 {
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
    // C: while ((t->blocksize % (1 << (t->partition_order + 1)) == 0) &&
    //        t->partition_order < t->max_partition_order)
    // The shift in C operates on int (1 << ...). partition_order is u8, +1 widens to int.
    while {
        let shift = (t.partition_order as u32).wrapping_add(1);
        // Replicate C int shift behavior; if shift >= 32, the result in C is undefined,
        // but partition_order <= 15 so shift <= 16 here, safe.
        let divisor: u32 = 1u32 << shift;
        (t.blocksize % divisor) == 0 && t.partition_order < t.max_partition_order
    } {
        t.partition_order = t.partition_order.wrapping_add(1);
    }
    t.cur_blocksize = t.blocksize;
    0
}
