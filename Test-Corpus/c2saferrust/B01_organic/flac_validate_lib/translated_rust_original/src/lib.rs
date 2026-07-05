

pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type tflac_u8 = uint8_t;
pub type tflac_u32 = uint32_t;
#[derive(Copy, Clone)]
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
pub const TFLAC_CHANNEL_INDEPENDENT: TFLAC_CHANNEL_MODE = 0;
pub type TFLAC_CHANNEL_MODE = ::core::ffi::c_uint;
pub const TFLAC_CHANNEL_MODE_COUNT: TFLAC_CHANNEL_MODE = 4;
pub const TFLAC_CHANNEL_MID_SIDE: TFLAC_CHANNEL_MODE = 3;
pub const TFLAC_CHANNEL_SIDE_RIGHT: TFLAC_CHANNEL_MODE = 2;
pub const TFLAC_CHANNEL_LEFT_SIDE: TFLAC_CHANNEL_MODE = 1;
#[no_mangle]
pub fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(
            15u32.wrapping_add(blocksize.wrapping_mul(4u32)) & 0xfffffff0u32,
        ),
    )
}

#[no_mangle]
pub fn flac_validate(t: &mut tflac) -> i32 {
    if t.blocksize < 16 {
        return -1;
    }
    if t.blocksize > 65_535 {
        return -1;
    }
    if t.samplerate == 0 {
        return -1;
    }
    if t.samplerate > 655_350 {
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

    if t.channel_mode as i32 != TFLAC_CHANNEL_INDEPENDENT as i32 {
        if t.channels != 2 || t.bitdepth == 32 {
            t.channel_mode = TFLAC_CHANNEL_INDEPENDENT as tflac_u8;
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
    while t.blocksize % (1u32 << (t.partition_order as u32 + 1)) == 0
        && t.partition_order < t.max_partition_order
    {
        t.partition_order = t.partition_order.wrapping_add(1);
    }

    t.cur_blocksize = t.blocksize;
    0
}

