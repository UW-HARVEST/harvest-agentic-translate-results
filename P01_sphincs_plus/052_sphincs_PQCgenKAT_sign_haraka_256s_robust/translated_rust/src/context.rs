use crate::params::*;

pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub tweaked512_rc64: [[u64; 8]; 10],
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            tweaked512_rc64: [[0u64; 8]; 10],
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}
