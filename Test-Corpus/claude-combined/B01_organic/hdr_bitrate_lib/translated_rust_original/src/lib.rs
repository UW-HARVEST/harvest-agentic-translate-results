use std::ffi::c_uint;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    // Reproduce C indexing exactly, including potentially out-of-bounds access
    // when `(((h[1]) >> 1) & 3) - 1 == -1`. We compute a flat byte offset and
    // dereference relative to the base of HALFRATE so the layout matches the C
    // memory layout (row-major [2][3][15]).
    let a: isize = if (h1 & 0x8) != 0 { 1 } else { 0 };
    let b: isize = (((h1 >> 1) & 3) as isize) - 1;
    let c: isize = (h2 >> 4) as isize;

    let base = HALFRATE.as_ptr() as *const u8;
    let offset = a * 45 + b * 15 + c;
    let val = unsafe { *base.offset(offset) };

    2u32 * (val as c_uint)
}
