
pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
#[no_mangle]
pub fn rev16(a: u32) -> u32 {
    ((a & 0xffff) as u16).reverse_bits() as u32
}

