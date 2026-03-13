use std::os::raw::c_uint;

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

/// # Safety
/// `h` must point to at least 3 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };
    let i0 = if h1 & 0x8 != 0 { 1usize } else { 0 };
    let i1 = (((h1 >> 1) & 3) as usize).wrapping_sub(1);
    let i2 = (h2 >> 4) as usize;
    let row = &*HALFRATE.as_ptr().add(i0);
    let col = &*row.as_ptr().add(i1);
    2 * (*col.as_ptr().add(i2)) as c_uint
}
