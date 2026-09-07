//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `update_frame_header`
//!
//! The C struct `tflac` (from `c_src/include/lib.h`) has, on the reference
//! platform, size 24 with field offsets 0, 4, 8, 12, 16, 20 — reproduced here
//! with `#[repr(C)]`.

#![allow(non_camel_case_types)]

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// ```c
/// struct tflac {
///     tflac_u32 samplerate;
///     tflac_u32 channels;
///     tflac_u32 bitdepth;
///     tflac_u8 channel_mode;
///     tflac_u32 frame_header;
///     tflac_u32 cur_blocksize;
/// };
/// ```
#[repr(C)]
pub struct tflac {
    pub samplerate: tflac_u32,
    pub channels: tflac_u32,
    pub bitdepth: tflac_u32,
    pub channel_mode: tflac_u8,
    pub frame_header: tflac_u32,
    pub cur_blocksize: tflac_u32,
}

/// `enum TFLAC_CHANNEL_MODE` (file-local in `src/lib.c`, no linkage).
#[allow(dead_code)]
mod channel_mode {
    pub const TFLAC_CHANNEL_INDEPENDENT: u8 = 0;
    pub const TFLAC_CHANNEL_LEFT_SIDE: u8 = 1;
    pub const TFLAC_CHANNEL_SIDE_RIGHT: u8 = 2;
    pub const TFLAC_CHANNEL_MID_SIDE: u8 = 3;
    pub const TFLAC_CHANNEL_MODE_COUNT: u8 = 4;
}
use channel_mode::*;

/// `void update_frame_header(tflac *t);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_frame_header(t: *mut tflac) {
    // The C code dereferences `t` unconditionally; do the same.
    let t: &mut tflac = unsafe { &mut *t };

    t.frame_header = 0xFFF8u32 << 16;

    // Block size code.
    match t.cur_blocksize {
        192 => t.frame_header |= 0x01u32 << 12,
        576 => t.frame_header |= 0x02u32 << 12,
        1152 => t.frame_header |= 0x03u32 << 12,
        2304 => t.frame_header |= 0x04u32 << 12,
        4608 => t.frame_header |= 0x05u32 << 12,
        256 => t.frame_header |= 0x08u32 << 12,
        512 => t.frame_header |= 0x09u32 << 12,
        1024 => t.frame_header |= 0x0Au32 << 12,
        2048 => t.frame_header |= 0x0Bu32 << 12,
        4096 => t.frame_header |= 0x0Cu32 << 12,
        8192 => t.frame_header |= 0x0Du32 << 12,
        16384 => t.frame_header |= 0x0Eu32 << 12,
        32768 => t.frame_header |= 0x0Fu32 << 12,
        _ => {
            t.frame_header |= if t.cur_blocksize <= 256 {
                0x06u32 << 12
            } else {
                0x07u32 << 12
            };
        }
    }

    // Sample rate code.
    match t.samplerate {
        882000 => t.frame_header |= 0x01u32 << 8,
        176400 => t.frame_header |= 0x02u32 << 8,
        192000 => t.frame_header |= 0x03u32 << 8,
        8000 => t.frame_header |= 0x04u32 << 8,
        16000 => t.frame_header |= 0x05u32 << 8,
        22050 => t.frame_header |= 0x06u32 << 8,
        24000 => t.frame_header |= 0x07u32 << 8,
        32000 => t.frame_header |= 0x08u32 << 8,
        44100 => t.frame_header |= 0x09u32 << 8,
        48000 => t.frame_header |= 0x0Au32 << 8,
        96000 => t.frame_header |= 0x0Bu32 << 8,
        _ => {
            if t.samplerate % 1000 == 0 {
                if t.samplerate / 1000 < 256 {
                    t.frame_header |= 0x0Cu32 << 8;
                }
            } else if t.samplerate < 65536 {
                t.frame_header |= 0x0Du32 << 8;
            } else if t.samplerate % 10 == 0 {
                if t.samplerate / 10 < 65536 {
                    t.frame_header |= 0x0Eu32 << 8;
                }
            }
        }
    }

    // Channel assignment.
    let mode: tflac_u8 = t.channel_mode % 4;
    match mode {
        TFLAC_CHANNEL_INDEPENDENT => {
            // C: (t->channels - 1) << 4 in uint32_t arithmetic (wraps when
            // channels == 0, and the shift discards the high bits).
            t.frame_header |= t.channels.wrapping_sub(1) << 4;
        }
        TFLAC_CHANNEL_LEFT_SIDE => t.frame_header |= 0x08u32 << 4,
        TFLAC_CHANNEL_SIDE_RIGHT => t.frame_header |= 0x09u32 << 4,
        TFLAC_CHANNEL_MID_SIDE => t.frame_header |= 0x0Au32 << 4,
        _ => {}
    }

    // Sample size code.
    match t.bitdepth {
        8 => t.frame_header |= 1u32 << 1,
        12 => t.frame_header |= 2u32 << 1,
        16 => t.frame_header |= 4u32 << 1,
        20 => t.frame_header |= 5u32 << 1,
        24 => t.frame_header |= 6u32 << 1,
        32 => t.frame_header |= 7u32 << 1,
        _ => {}
    }
}
