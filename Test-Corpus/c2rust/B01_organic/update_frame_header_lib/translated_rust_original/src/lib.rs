pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type tflac_u8 = uint8_t;
pub type tflac_u32 = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub samplerate: tflac_u32,
    pub channels: tflac_u32,
    pub bitdepth: tflac_u32,
    pub channel_mode: tflac_u8,
    pub frame_header: tflac_u32,
    pub cur_blocksize: tflac_u32,
}
pub const TFLAC_CHANNEL_MID_SIDE: TFLAC_CHANNEL_MODE = 3;
pub const TFLAC_CHANNEL_SIDE_RIGHT: TFLAC_CHANNEL_MODE = 2;
pub const TFLAC_CHANNEL_LEFT_SIDE: TFLAC_CHANNEL_MODE = 1;
pub const TFLAC_CHANNEL_INDEPENDENT: TFLAC_CHANNEL_MODE = 0;
pub type TFLAC_CHANNEL_MODE = ::core::ffi::c_uint;
pub const TFLAC_CHANNEL_MODE_COUNT: TFLAC_CHANNEL_MODE = 4;
#[no_mangle]
pub unsafe extern "C" fn update_frame_header(mut t: *mut tflac) {
    (*t).frame_header = ((0xfff8 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int) as tflac_u32;
    match (*t).cur_blocksize {
        192 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x1 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        576 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x2 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        1152 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x3 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        2304 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x4 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        4608 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x5 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        256 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x8 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        512 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x9 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        1024 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xa as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        2048 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xb as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        4096 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xc as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        8192 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xd as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        16384 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xe as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        32768 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xf as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int)
                as tflac_u32;
        }
        _ => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | if (*t).cur_blocksize <= 256 as tflac_u32 {
                    (0x6 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int
                } else {
                    (0x7 as ::core::ffi::c_uint) << 12 as ::core::ffi::c_int
                }) as tflac_u32;
        }
    }
    match (*t).samplerate {
        882000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x1 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        176400 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x2 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        192000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x3 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        8000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x4 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        16000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x5 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        22050 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x6 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        24000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x7 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        32000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x8 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        44100 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0x9 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        48000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xa as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        96000 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (0xb as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                as tflac_u32;
        }
        _ => {
            if (*t).samplerate.wrapping_rem(1000 as tflac_u32) == 0 as tflac_u32 {
                if (*t).samplerate.wrapping_div(1000 as tflac_u32) < 256 as tflac_u32 {
                    (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                        | (0xc as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                        as tflac_u32;
                }
            } else if (*t).samplerate < 65536 as tflac_u32 {
                (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                    | (0xd as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                    as tflac_u32;
            } else if (*t).samplerate.wrapping_rem(10 as tflac_u32) == 0 as tflac_u32 {
                if (*t).samplerate.wrapping_div(10 as tflac_u32) < 65536 as tflac_u32 {
                    (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                        | (0xe as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int)
                        as tflac_u32;
                }
            }
        }
    }
    let mut mode: tflac_u8 =
        ((*t).channel_mode as ::core::ffi::c_int % 4 as ::core::ffi::c_int) as tflac_u8;
    match mode as TFLAC_CHANNEL_MODE as ::core::ffi::c_uint {
        0 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | ((*t).channels.wrapping_sub(1 as tflac_u32) << 4 as ::core::ffi::c_int)
                    as ::core::ffi::c_uint) as tflac_u32;
        }
        1 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | ((0x8 as ::core::ffi::c_int) << 4 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                as tflac_u32;
        }
        2 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | ((0x9 as ::core::ffi::c_int) << 4 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                as tflac_u32;
        }
        3 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | ((0xa as ::core::ffi::c_int) << 4 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                as tflac_u32;
        }
        _ => {}
    }
    match (*t).bitdepth {
        8 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        12 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (2 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        16 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (4 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        20 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (5 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        24 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (6 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        32 => {
            (*t).frame_header = ((*t).frame_header as ::core::ffi::c_uint
                | (7 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int)
                as tflac_u32;
        }
        _ => {}
    };
}
