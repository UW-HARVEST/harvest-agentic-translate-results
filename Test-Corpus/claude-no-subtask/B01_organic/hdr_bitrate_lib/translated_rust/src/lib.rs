use std::ffi::c_uint;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    static HALFRATE: [[[u8; 15]; 3]; 2] = [
        [
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
        ],
        [
            [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
            [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192],
            [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224],
        ],
    ];

    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    let i: usize = if (h1 & 0x8) != 0 { 1 } else { 0 };
    // C: (((h[1]) >> 1) & 3) - 1 — may be -1 (matches original C behavior, including UB)
    let j: isize = (((h1 >> 1) & 3) as isize) - 1;
    let k: isize = (h2 >> 4) as isize;

    // Compute the linear offset into the flat array exactly as C would,
    // preserving the original (potentially out-of-bounds) indexing behavior.
    let base = HALFRATE.as_ptr() as *const u8;
    let offset: isize = (i as isize) * (3 * 15) + j * 15 + k;
    let val = unsafe { *base.offset(offset) };

    2u32 * (val as u32)
}
