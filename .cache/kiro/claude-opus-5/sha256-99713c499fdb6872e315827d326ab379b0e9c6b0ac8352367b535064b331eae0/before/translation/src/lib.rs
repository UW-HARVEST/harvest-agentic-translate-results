//! Rust translation of c_src/src/lib.c (tflac frame header construction).
//!
//! Behaviour is a byte-for-byte match of the original C, including the
//! wrapping arithmetic when `channels == 0`.

use std::ffi::c_void;

pub type TflacU8 = u8;
pub type TflacU32 = u32;

/// Mirrors `struct tflac` from include/lib.h.
#[repr(C)]
pub struct tflac {
    pub samplerate: TflacU32,
    pub channels: TflacU32,
    pub bitdepth: TflacU32,
    pub channel_mode: TflacU8,
    pub frame_header: TflacU32,
    pub cur_blocksize: TflacU32,
}

/* enum TFLAC_CHANNEL_MODE */
const TFLAC_CHANNEL_INDEPENDENT: u8 = 0;
const TFLAC_CHANNEL_LEFT_SIDE: u8 = 1;
const TFLAC_CHANNEL_SIDE_RIGHT: u8 = 2;
const TFLAC_CHANNEL_MID_SIDE: u8 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_frame_header(t: *mut tflac) {
    if t.is_null() {
        // The C code dereferences unconditionally; a null pointer there is
        // undefined behaviour rather than a defined no-op. Guard anyway so the
        // Rust side cannot form an invalid reference.
        let _ = t as *const c_void;
        return;
    }
    let t: &mut tflac = unsafe { &mut *t };

    t.frame_header = 0xFFF8u32 << 16;

    // Block size bits.
    t.frame_header |= match t.cur_blocksize {
        192 => 0x01u32 << 12,
        576 => 0x02u32 << 12,
        1152 => 0x03u32 << 12,
        2304 => 0x04u32 << 12,
        4608 => 0x05u32 << 12,
        256 => 0x08u32 << 12,
        512 => 0x09u32 << 12,
        1024 => 0x0Au32 << 12,
        2048 => 0x0Bu32 << 12,
        4096 => 0x0Cu32 << 12,
        8192 => 0x0Du32 << 12,
        16384 => 0x0Eu32 << 12,
        32768 => 0x0Fu32 << 12,
        other => {
            if other <= 256 {
                0x06u32 << 12
            } else {
                0x07u32 << 12
            }
        }
    };

    // Sample rate bits.
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
        rate => {
            if rate % 1000 == 0 {
                if rate / 1000 < 256 {
                    t.frame_header |= 0x0Cu32 << 8;
                }
            } else if rate < 65536 {
                t.frame_header |= 0x0Du32 << 8;
            } else if rate % 10 == 0 {
                if rate / 10 < 65536 {
                    t.frame_header |= 0x0Eu32 << 8;
                }
            }
        }
    }

    // Channel assignment bits.
    let mode: TflacU8 = t.channel_mode % 4;
    match mode {
        TFLAC_CHANNEL_INDEPENDENT => {
            // Wrapping matches C's unsigned arithmetic when channels == 0.
            t.frame_header |= t.channels.wrapping_sub(1) << 4;
        }
        TFLAC_CHANNEL_LEFT_SIDE => t.frame_header |= 0x08u32 << 4,
        TFLAC_CHANNEL_SIDE_RIGHT => t.frame_header |= 0x09u32 << 4,
        TFLAC_CHANNEL_MID_SIDE => t.frame_header |= 0x0Au32 << 4,
        _ => {}
    }

    // Bit depth bits.
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
