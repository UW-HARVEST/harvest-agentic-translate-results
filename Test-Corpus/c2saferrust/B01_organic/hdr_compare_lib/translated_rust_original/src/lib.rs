

pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
fn hdr_valid(h: *const u8) -> i32 {
    let b0 = unsafe { *h.add(0) } as i32;
    let b1 = unsafe { *h.add(1) } as i32;
    let b2 = unsafe { *h.add(2) } as i32;

    (b0 == 0xff
        && ((b1 & 0xf0 == 0xf0) || (b1 & 0xfe == 0xe2))
        && ((b1 >> 1) & 3 != 0)
        && (b2 >> 4 != 15)
        && ((b2 >> 2) & 3 != 3)) as i32
}

#[no_mangle]
pub fn hdr_compare(h1: &[u8], h2: &[u8]) -> i32 {
    (
        hdr_valid(h2.as_ptr()) != 0
            && ((h1[1] as i32 ^ h2[1] as i32) & 0xfe) == 0
            && ((h1[2] as i32 ^ h2[2] as i32) & 0x0c) == 0
            && (((h1[2] as i32 & 0xf0) == 0) as i32 ^ ((h2[2] as i32 & 0xf0) == 0) as i32) == 0
    ) as i32
}

