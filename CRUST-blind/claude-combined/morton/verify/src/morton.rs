// Define the Morton struct
pub struct Morton {
    pub lo: u32,
    pub hi: u32,
}
// Function Declarations
pub fn unmortoner(x: u64) -> u32 {
    let mut x = x;
    x &= 0x5555555555555555;
    x = (x | x >> 1) & 0x3333333333333333;
    x = (x | x >> 2) & 0x0F0F0F0F0F0F0F0F;
    x = (x | x >> 4) & 0x00FF00FF00FF00FF;
    x = (x | x >> 8) & 0x0000FFFF0000FFFF;
    x = (x | x >> 16) & 0x00000000FFFFFFFF;
    x as u32
}
pub fn morton(hi: u32, lo: u32) -> u64 {
    let mut xu: u64 = lo as u64;
    let mut yu: u64 = hi as u64;
    xu = (xu | xu << 16) & 0x0000FFFF0000FFFF;
    xu = (xu | xu << 8) & 0x00FF00FF00FF00FF;
    xu = (xu | xu << 4) & 0x0F0F0F0F0F0F0F0F;
    xu = (xu | xu << 2) & 0x3333333333333333;
    xu = (xu | xu << 1) & 0x5555555555555555;
    yu = (yu | yu << 16) & 0x0000FFFF0000FFFF;
    yu = (yu | yu << 8) & 0x00FF00FF00FF00FF;
    yu = (yu | yu << 4) & 0x0F0F0F0F0F0F0F0F;
    yu = (yu | yu << 2) & 0x3333333333333333;
    yu = (yu | yu << 1) & 0x5555555555555555;
    xu | yu << 1
}

pub fn unmorton(z: u64) -> Morton {
    Morton {
        hi: unmortoner(z),
        lo: unmortoner(z >> 1),
    }
}
