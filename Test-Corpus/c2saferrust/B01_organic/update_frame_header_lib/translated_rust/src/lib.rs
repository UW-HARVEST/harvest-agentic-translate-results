


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
pub fn update_frame_header(t: &mut tflac) {
     (*t).frame_header = (0xfff8_u32) << 16;

let blocksize_code: tflac_u32 = match (*t).cur_blocksize {
    192 => 0x1,
    576 => 0x2,
    1152 => 0x3,
    2304 => 0x4,
    4608 => 0x5,
    256 => 0x8,
    512 => 0x9,
    1024 => 0xa,
    2048 => 0xb,
    4096 => 0xc,
    8192 => 0xd,
    16384 => 0xe,
    32768 => 0xf,
    _ => {
        if (*t).cur_blocksize <= 256 {
            0x6
        } else {
            0x7
        }
    }
};

(*t).frame_header |= blocksize_code << 12;


    let mode: u8 = {
        let t: &mut tflac = t;
         let t = unsafe { &mut *t };

match t.samplerate {
    882000 => t.frame_header |= 0x1 << 8,
    176400 => t.frame_header |= 0x2 << 8,
    192000 => t.frame_header |= 0x3 << 8,
    8000 => t.frame_header |= 0x4 << 8,
    16000 => t.frame_header |= 0x5 << 8,
    22050 => t.frame_header |= 0x6 << 8,
    24000 => t.frame_header |= 0x7 << 8,
    32000 => t.frame_header |= 0x8 << 8,
    44100 => t.frame_header |= 0x9 << 8,
    48000 => t.frame_header |= 0xa << 8,
    96000 => t.frame_header |= 0xb << 8,
    _ => {
        if t.samplerate % 1000 == 0 {
            if t.samplerate / 1000 < 256 {
                t.frame_header |= 0xc << 8;
            }
        } else if t.samplerate < 65536 {
            t.frame_header |= 0xd << 8;
        } else if t.samplerate % 10 == 0 && t.samplerate / 10 < 65536 {
            t.frame_header |= 0xe << 8;
        }
    }
}

let mode: tflac_u8 = (t.channel_mode as i32 % 4) as tflac_u8;

match mode {
    0 => t.frame_header |= (t.channels - 1) << 4,
    1 => t.frame_header |= 0x8 << 4,
    2 => t.frame_header |= 0x9 << 4,
    3 => t.frame_header |= 0xa << 4,
    _ => {}
}

match t.bitdepth {
    8 => t.frame_header |= 1 << 1,
    12 => t.frame_header |= 2 << 1,
    16 => t.frame_header |= 4 << 1,
    20 => t.frame_header |= 5 << 1,
    24 => t.frame_header |= 6 << 1,
    32 => t.frame_header |= 7 << 1,
    _ => {}
}


        mode
    };
    let _ = mode;
}

