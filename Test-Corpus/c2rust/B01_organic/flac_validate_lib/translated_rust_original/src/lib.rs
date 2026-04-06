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
pub unsafe extern "C" fn tflac_size_memory(mut blocksize: tflac_u32) -> tflac_u32 {
    return (15 as ::core::ffi::c_uint as tflac_u32).wrapping_add((5 as tflac_u32).wrapping_mul(
        (15 as tflac_u32).wrapping_add(blocksize.wrapping_mul(4 as tflac_u32))
            & 0xfffffff0 as tflac_u32,
    ));
}
#[no_mangle]
pub unsafe extern "C" fn flac_validate(mut t: *mut tflac) -> ::core::ffi::c_int {
    if (*t).blocksize < 16 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).blocksize > 65535 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).samplerate == 0 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).samplerate > 655350 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).channels == 0 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).channels > 8 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).bitdepth == 0 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).bitdepth > 32 as tflac_u32 {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).channel_mode as ::core::ffi::c_int != TFLAC_CHANNEL_INDEPENDENT as ::core::ffi::c_int {
        if (*t).channels != 2 as tflac_u32 || (*t).bitdepth == 32 as tflac_u32 {
            (*t).channel_mode = TFLAC_CHANNEL_INDEPENDENT as ::core::ffi::c_int as tflac_u8;
        }
    }
    if (*t).max_rice_value as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        if (*t).bitdepth <= 16 as tflac_u32 {
            (*t).max_rice_value = 14 as tflac_u8;
        } else {
            (*t).max_rice_value = 30 as tflac_u8;
        }
    } else if (*t).max_rice_value as ::core::ffi::c_int > 30 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).max_partition_order as ::core::ffi::c_int > 15 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    if (*t).min_partition_order as ::core::ffi::c_int
        > (*t).max_partition_order as ::core::ffi::c_int
    {
        return -(1 as ::core::ffi::c_int);
    }
    (*t).partition_order = (*t).min_partition_order;
    while (*t).blocksize.wrapping_rem(
        ((1 as ::core::ffi::c_int)
            << (*t).partition_order as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
            as tflac_u32,
    ) == 0 as tflac_u32
        && ((*t).partition_order as ::core::ffi::c_int)
            < (*t).max_partition_order as ::core::ffi::c_int
    {
        (*t).partition_order = (*t).partition_order.wrapping_add(1);
    }
    (*t).cur_blocksize = (*t).blocksize;
    return 0 as ::core::ffi::c_int;
}
