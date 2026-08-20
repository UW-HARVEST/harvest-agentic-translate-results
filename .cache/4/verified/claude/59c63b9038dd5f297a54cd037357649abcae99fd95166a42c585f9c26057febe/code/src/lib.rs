//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `update_frame_header`
//!
//! Behaviour is reproduced verbatim, including the quirks of the original C
//! source (e.g. the `882000` sample-rate case, which in the FLAC spec would be
//! `88200`; the truncating `channel_mode % 4`; and the wrapping
//! `(channels - 1) << 4` for `channels == 0`).

#![allow(non_camel_case_types)]

use core::ffi::c_uint;
use core::mem::offset_of;

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac` from `include/lib.h`.
///
/// The C layout is:
/// ```text
///   offset  0: tflac_u32 samplerate
///   offset  4: tflac_u32 channels
///   offset  8: tflac_u32 bitdepth
///   offset 12: tflac_u8  channel_mode   (+3 bytes padding)
///   offset 16: tflac_u32 frame_header
///   offset 20: tflac_u32 cur_blocksize
///   size 24, align 4
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

// The layout the byte-wise field accessors below rely on, checked at compile
// time against the values the C compiler reports for `struct tflac`:
//   sizeof=24 align=4
//   samplerate=0 channels=4 bitdepth=8 channel_mode=12 frame_header=16
//   cur_blocksize=20
const _: () = {
    assert!(core::mem::size_of::<tflac>() == 24);
    assert!(core::mem::align_of::<tflac>() == 4);
    assert!(offset_of!(tflac, samplerate) == 0);
    assert!(offset_of!(tflac, channels) == 4);
    assert!(offset_of!(tflac, bitdepth) == 8);
    assert!(offset_of!(tflac, channel_mode) == 12);
    assert!(offset_of!(tflac, frame_header) == 16);
    assert!(offset_of!(tflac, cur_blocksize) == 20);
};

/// `enum TFLAC_CHANNEL_MODE` from `src/lib.c` (file-local in C).
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TFLAC_CHANNEL_MODE {
    TFLAC_CHANNEL_INDEPENDENT = 0,
    TFLAC_CHANNEL_LEFT_SIDE = 1,
    TFLAC_CHANNEL_SIDE_RIGHT = 2,
    TFLAC_CHANNEL_MID_SIDE = 3,
    TFLAC_CHANNEL_MODE_COUNT = 4,
}

/// Computes the blocksize nibble contribution to the frame header.
///
/// Mirrors the first `switch` on `t->cur_blocksize`.
fn blocksize_bits(cur_blocksize: tflac_u32) -> tflac_u32 {
    let code: c_uint = match cur_blocksize {
        192 => 0x01,
        576 => 0x02,
        1152 => 0x03,
        2304 => 0x04,
        4608 => 0x05,
        256 => 0x08,
        512 => 0x09,
        1024 => 0x0A,
        2048 => 0x0B,
        4096 => 0x0C,
        8192 => 0x0D,
        16384 => 0x0E,
        32768 => 0x0F,
        // default: the `<= 256` test is dead for 192/256 (handled above) but is
        // still reachable for e.g. 0, 1, 255.
        _ => {
            if cur_blocksize <= 256 {
                0x06
            } else {
                0x07
            }
        }
    };
    (code as tflac_u32) << 12
}

/// Computes the sample-rate nibble contribution to the frame header.
///
/// Mirrors the second `switch` on `t->samplerate`. Note that the original C
/// lists `882000` (not the FLAC-spec `88200`) for code `0x01`; that is
/// reproduced as-is. Also note that the `default` arm may contribute nothing at
/// all, leaving the sample-rate nibble as `0x0` (meaning "get it from
/// STREAMINFO").
fn samplerate_bits(samplerate: tflac_u32) -> tflac_u32 {
    let code: c_uint = match samplerate {
        882000 => 0x01,
        176400 => 0x02,
        192000 => 0x03,
        8000 => 0x04,
        16000 => 0x05,
        22050 => 0x06,
        24000 => 0x07,
        32000 => 0x08,
        44100 => 0x09,
        48000 => 0x0A,
        96000 => 0x0B,
        _ => {
            if samplerate % 1000 == 0 {
                if samplerate / 1000 < 256 {
                    0x0C
                } else {
                    // No bits are OR-ed in by the C code in this case.
                    return 0;
                }
            } else if samplerate < 65536 {
                0x0D
            } else if samplerate % 10 == 0 {
                if samplerate / 10 < 65536 {
                    0x0E
                } else {
                    return 0;
                }
            } else {
                return 0;
            }
        }
    };
    (code as tflac_u32) << 8
}

/// Computes the channel-assignment nibble contribution to the frame header.
///
/// Mirrors the third `switch`. Because the C code first reduces the mode with
/// `% 4`, the `default` arm is unreachable.
fn channel_bits(channel_mode: tflac_u8, channels: tflac_u32) -> tflac_u32 {
    let mode: tflac_u8 = channel_mode % 4;
    match mode {
        // TFLAC_CHANNEL_INDEPENDENT
        //
        // `t->channels - 1` is computed in `tflac_u32` (unsigned) arithmetic, so
        // `channels == 0` wraps to 0xFFFFFFFF and the shift yields 0xFFFFFFF0.
        0 => channels.wrapping_sub(1) << 4,
        // TFLAC_CHANNEL_LEFT_SIDE
        1 => 0x08 << 4,
        // TFLAC_CHANNEL_SIDE_RIGHT
        2 => 0x09 << 4,
        // TFLAC_CHANNEL_MID_SIDE
        3 => 0x0A << 4,
        _ => 0,
    }
}

/// Computes the sample-size contribution to the frame header.
///
/// Mirrors the fourth `switch` on `t->bitdepth`. Unlisted bit depths contribute
/// nothing (code `0x0`, "get it from STREAMINFO").
fn bitdepth_bits(bitdepth: tflac_u32) -> tflac_u32 {
    let code: c_uint = match bitdepth {
        8 => 1,
        12 => 2,
        16 => 4,
        20 => 5,
        24 => 6,
        32 => 7,
        _ => return 0,
    };
    (code as tflac_u32) << 1
}

// ---------------------------------------------------------------------------
// Field accessors.
//
// The C compiler accesses `t->field` with plain loads/stores that carry no
// alignment *requirement* on x86-64: passing a misaligned `tflac*` (undefined
// behaviour on paper, but something an FFI caller can and does construct) makes
// the C read and write the fields anyway. Creating a Rust reference
// (`&mut *t`) instead imposes `align_of::<tflac>() == 4` and traps in any build
// with debug assertions ("misaligned pointer dereference"), which would be a
// visible behavioural difference from the C.
//
// Reading and writing the fields byte-wise through `*mut u8` reproduces the C
// exactly for every pointer an external caller can pass:
//   * aligned pointer   -> identical native-endian value,
//   * misaligned pointer-> the access still happens, as in C,
//   * null pointer      -> the byte access faults, so the process dies with
//                          SIGSEGV just like the C does (no earlier Rust-side
//                          precondition panic, which would turn SIGSEGV into
//                          SIGABRT).
// `wrapping_add` is used for the offset arithmetic so that no `ptr::add`
// precondition check can fire before the faulting access.
// ---------------------------------------------------------------------------

/// Reads the `tflac_u32` field at byte offset `off` from `base`.
///
/// # Safety
/// `base.wrapping_add(off) .. +4` must be readable.
#[inline]
unsafe fn read_u32_at(base: *const u8, off: usize) -> tflac_u32 {
    let p = base.wrapping_add(off);
    unsafe {
        tflac_u32::from_ne_bytes([
            *p,
            *p.wrapping_add(1),
            *p.wrapping_add(2),
            *p.wrapping_add(3),
        ])
    }
}

/// Reads the `tflac_u8` field at byte offset `off` from `base`.
///
/// # Safety
/// `base.wrapping_add(off)` must be readable.
#[inline]
unsafe fn read_u8_at(base: *const u8, off: usize) -> tflac_u8 {
    unsafe { *base.wrapping_add(off) }
}

/// Writes the `tflac_u32` field at byte offset `off` from `base`.
///
/// # Safety
/// `base.wrapping_add(off) .. +4` must be writable.
#[inline]
unsafe fn write_u32_at(base: *mut u8, off: usize, value: tflac_u32) {
    let bytes = value.to_ne_bytes();
    let p = base.wrapping_add(off);
    unsafe {
        *p = bytes[0];
        *p.wrapping_add(1) = bytes[1];
        *p.wrapping_add(2) = bytes[2];
        *p.wrapping_add(3) = bytes[3];
    }
}

/// `void update_frame_header(tflac *t);`
///
/// Rebuilds `t->frame_header` from the sample rate, channel count, bit depth,
/// channel mode and current blocksize stored in `*t`. No other field (and none
/// of the struct padding) is touched, exactly as in the C.
///
/// # Safety
///
/// `t` must point to 24 readable and writable bytes laid out as a
/// `struct tflac`, exactly as the C function requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_frame_header(t: *mut tflac) {
    let base = t as *mut u8;

    // SAFETY: the caller guarantees `*t` is readable/writable; every access is
    // byte-sized, so no alignment requirement is imposed on `t`.
    let (cur_blocksize, samplerate, channel_mode, channels, bitdepth) = unsafe {
        (
            read_u32_at(base, offset_of!(tflac, cur_blocksize)),
            read_u32_at(base, offset_of!(tflac, samplerate)),
            read_u8_at(base, offset_of!(tflac, channel_mode)),
            read_u32_at(base, offset_of!(tflac, channels)),
            read_u32_at(base, offset_of!(tflac, bitdepth)),
        )
    };

    // 0xFFF8U << 16 -- the sync code plus the reserved/blocking-strategy bits.
    // 0xFFF8 has type `unsigned int` in C, so this is 0xFFF80000 in 32 bits.
    // Note that line 12 of the C ASSIGNS this value, so whatever
    // `t->frame_header` held before the call is discarded, not OR-ed into.
    let mut frame_header: tflac_u32 = 0xFFF8u32 << 16;

    frame_header |= blocksize_bits(cur_blocksize);
    frame_header |= samplerate_bits(samplerate);
    frame_header |= channel_bits(channel_mode, channels);
    frame_header |= bitdepth_bits(bitdepth);

    // SAFETY: as above.
    unsafe { write_u32_at(base, offset_of!(tflac, frame_header), frame_header) };
}
