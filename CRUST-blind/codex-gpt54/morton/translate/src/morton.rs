// Define the Morton struct
pub struct Morton {
    pub lo: u32,
    pub hi: u32,
}
// Function Declarations
pub fn unmortoner(x: u64) -> u32 {
    let mut x = x;
    x &= 0x5555_5555_5555_5555;
    x = (x | (x >> 1)) & 0x3333_3333_3333_3333;
    x = (x | (x >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
    x = (x | (x >> 4)) & 0x00ff_00ff_00ff_00ff;
    x = (x | (x >> 8)) & 0x0000_ffff_0000_ffff;
    x = (x | (x >> 16)) & 0x0000_0000_ffff_ffff;
    x as u32
}
pub fn morton(hi: u32, lo: u32) -> u64 {
    let mut xu = lo as u64;
    let mut yu = hi as u64;

    xu = (xu | (xu << 16)) & 0x0000_ffff_0000_ffff;
    xu = (xu | (xu << 8)) & 0x00ff_00ff_00ff_00ff;
    xu = (xu | (xu << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    xu = (xu | (xu << 2)) & 0x3333_3333_3333_3333;
    xu = (xu | (xu << 1)) & 0x5555_5555_5555_5555;

    yu = (yu | (yu << 16)) & 0x0000_ffff_0000_ffff;
    yu = (yu | (yu << 8)) & 0x00ff_00ff_00ff_00ff;
    yu = (yu | (yu << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    yu = (yu | (yu << 2)) & 0x3333_3333_3333_3333;
    yu = (yu | (yu << 1)) & 0x5555_5555_5555_5555;

    xu | (yu << 1)
}
