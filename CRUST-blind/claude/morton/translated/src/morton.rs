// Define the Morton struct
pub struct Morton {
    pub lo: u32,
    pub hi: u32,
}
// Function Declarations
pub fn unmortoner(x: u64) -> u32 {
    let mut x = x & 0x5555555555555555u64;
    x = (x | (x >> 1)) & 0x3333333333333333u64;
    x = (x | (x >> 2)) & 0x0F0F0F0F0F0F0F0Fu64;
    x = (x | (x >> 4)) & 0x00FF00FF00FF00FFu64;
    x = (x | (x >> 8)) & 0x0000FFFF0000FFFFu64;
    x = (x | (x >> 16)) & 0x00000000FFFFFFFFu64;
    x as u32
}
pub fn morton(hi: u32, lo: u32) -> u64 {
    let mut xu = lo as u64;
    let mut yu = hi as u64;
    xu = (xu | (xu << 16)) & 0x0000FFFF0000FFFFu64;
    xu = (xu | (xu << 8)) & 0x00FF00FF00FF00FFu64;
    xu = (xu | (xu << 4)) & 0x0F0F0F0F0F0F0F0Fu64;
    xu = (xu | (xu << 2)) & 0x3333333333333333u64;
    xu = (xu | (xu << 1)) & 0x5555555555555555u64;
    yu = (yu | (yu << 16)) & 0x0000FFFF0000FFFFu64;
    yu = (yu | (yu << 8)) & 0x00FF00FF00FF00FFu64;
    yu = (yu | (yu << 4)) & 0x0F0F0F0F0F0F0F0Fu64;
    yu = (yu | (yu << 2)) & 0x3333333333333333u64;
    yu = (yu | (yu << 1)) & 0x5555555555555555u64;
    xu | (yu << 1)
}
