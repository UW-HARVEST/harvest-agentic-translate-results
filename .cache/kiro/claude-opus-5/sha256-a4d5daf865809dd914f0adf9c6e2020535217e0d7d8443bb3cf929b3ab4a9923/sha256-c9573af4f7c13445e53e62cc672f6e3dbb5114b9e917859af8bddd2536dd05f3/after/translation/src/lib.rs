//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (matches `nm -D` on the C shared object):
//!   * `update_frame_header`
//!
//! The behaviour of the original C is reproduced exactly, including its quirks
//! (e.g. the `882000` sample-rate case, which in real FLAC would be `88200`,
//! and the wrapping `channels - 1` when `channels == 0`).

#![allow(non_camel_case_types)]

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

/// Mirrors `struct tflac` from `include/lib.h`.
///
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

/// `enum TFLAC_CHANNEL_MODE` from `src/lib.c`.
const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;
const TFLAC_CHANNEL_LEFT_SIDE: tflac_u8 = 1;
const TFLAC_CHANNEL_SIDE_RIGHT: tflac_u8 = 2;
const TFLAC_CHANNEL_MID_SIDE: tflac_u8 = 3;

/// Computes the FLAC frame header bits from the encoder state.
///
/// Direct translation of `void update_frame_header(tflac *t)`.
///
/// # Safety
///
/// `t` must be a valid, aligned, non-null pointer to a `tflac` struct, exactly
/// as required by the C original (which also dereferences it unconditionally).
///
/// Field access goes through raw pointers (`addr_of!` / `addr_of_mut!`) rather
/// than through a `&mut tflac` reference. This matters for FFI fidelity: forming
/// a reference from the incoming pointer would trip rustc's debug-only
/// null/alignment dereference check, which panics and — across an `extern "C"`
/// boundary — aborts with `SIGABRT`, whereas the C simply faults with
/// `SIGSEGV`. Staying on raw pointers reproduces the C's fault behaviour, and
/// also avoids asserting the aliasing and validity guarantees a reference would
/// imply for a pointer that came from outside.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_frame_header(t: *mut tflac) {
    use core::ptr::{addr_of, addr_of_mut};

    let cur_blocksize: tflac_u32 = addr_of!((*t).cur_blocksize).read();
    let samplerate: tflac_u32 = addr_of!((*t).samplerate).read();
    let channels: tflac_u32 = addr_of!((*t).channels).read();
    let bitdepth: tflac_u32 = addr_of!((*t).bitdepth).read();
    let channel_mode: tflac_u8 = addr_of!((*t).channel_mode).read();

    // Sync code + reserved + blocking strategy bits.
    let mut frame_header: tflac_u32 = 0xFFF8u32 << 16;

    // ---- block size ----
    frame_header |= match cur_blocksize {
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
        _ => {
            if cur_blocksize <= 256 {
                0x06u32 << 12
            } else {
                0x07u32 << 12
            }
        }
    };

    // ---- sample rate ----
    match samplerate {
        // NOTE: `882000` (not `88200`) is what the C source says; preserved.
        882000 => frame_header |= 0x01u32 << 8,
        176400 => frame_header |= 0x02u32 << 8,
        192000 => frame_header |= 0x03u32 << 8,
        8000 => frame_header |= 0x04u32 << 8,
        16000 => frame_header |= 0x05u32 << 8,
        22050 => frame_header |= 0x06u32 << 8,
        24000 => frame_header |= 0x07u32 << 8,
        32000 => frame_header |= 0x08u32 << 8,
        44100 => frame_header |= 0x09u32 << 8,
        48000 => frame_header |= 0x0Au32 << 8,
        96000 => frame_header |= 0x0Bu32 << 8,
        _ => {
            if samplerate % 1000 == 0 {
                if samplerate / 1000 < 256 {
                    frame_header |= 0x0Cu32 << 8;
                }
            } else if samplerate < 65536 {
                frame_header |= 0x0Du32 << 8;
            } else if samplerate % 10 == 0 {
                if samplerate / 10 < 65536 {
                    frame_header |= 0x0Eu32 << 8;
                }
            }
        }
    }

    // ---- channel assignment ----
    let mode: tflac_u8 = channel_mode % 4;
    match mode {
        TFLAC_CHANNEL_INDEPENDENT => {
            // `wrapping_sub` reproduces the C unsigned underflow when
            // `channels == 0`; `wrapping_shl` reproduces the bits shifted off
            // the top of the 32-bit word for large `channels`.
            frame_header |= channels.wrapping_sub(1).wrapping_shl(4);
        }
        TFLAC_CHANNEL_LEFT_SIDE => frame_header |= 0x08u32 << 4,
        TFLAC_CHANNEL_SIDE_RIGHT => frame_header |= 0x09u32 << 4,
        TFLAC_CHANNEL_MID_SIDE => frame_header |= 0x0Au32 << 4,
        _ => {}
    }

    // ---- bit depth ----
    match bitdepth {
        8 => frame_header |= 1u32 << 1,
        12 => frame_header |= 2u32 << 1,
        16 => frame_header |= 4u32 << 1,
        20 => frame_header |= 5u32 << 1,
        24 => frame_header |= 6u32 << 1,
        32 => frame_header |= 7u32 << 1,
        _ => {}
    }

    // The C assigns `frame_header` (it never reads the incoming value), and it
    // writes no other field.
    addr_of_mut!((*t).frame_header).write(frame_header);
}
