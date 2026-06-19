use crate::params::SPX_N;

#[repr(C)]
#[derive(Clone)]
pub struct spx_ctx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    pub state_seeded_512: [u8; 72],
    pub tweaked512_rc64: [[u64; 8]; 10],
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl Default for spx_ctx {
    fn default() -> Self {
        Self {
            pub_seed: [0; SPX_N],
            sk_seed: [0; SPX_N],
            state_seeded: [0; 40],
            state_seeded_512: [0; 72],
            tweaked512_rc64: [[0; 8]; 10],
            tweaked256_rc32: [[0; 8]; 10],
        }
    }
}
