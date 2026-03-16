use crate::params::*;

#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    pub state_seeded_512: [u8; 72],
}
