// Define the Morton struct
pub struct Morton {
    pub lo: u32,
    pub hi: u32,
}
// Function Declarations
pub fn unmortoner(x: u64) -> u32 {
    let mut x = x & 0x5555_5555_5555_5555;
    x = (x | (x >> 1)) & 0x3333_3333_3333_3333;
    x = (x | (x >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
    x = (x | (x >> 4)) & 0x00ff_00ff_00ff_00ff;
    x = (x | (x >> 8)) & 0x0000_ffff_0000_ffff;
    x = (x | (x >> 16)) & 0x0000_0000_ffff_ffff;
    x as u32
}
pub fn morton(hi: u32, lo: u32) -> u64 {
    fn interleave_zeros(x: u32) -> u64 {
        let mut x = u64::from(x);
        x = (x | (x << 16)) & 0x0000_ffff_0000_ffff;
        x = (x | (x << 8)) & 0x00ff_00ff_00ff_00ff;
        x = (x | (x << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
        x = (x | (x << 2)) & 0x3333_3333_3333_3333;
        x = (x | (x << 1)) & 0x5555_5555_5555_5555;
        x
    }

    interleave_zeros(lo) | (interleave_zeros(hi) << 1)
}
