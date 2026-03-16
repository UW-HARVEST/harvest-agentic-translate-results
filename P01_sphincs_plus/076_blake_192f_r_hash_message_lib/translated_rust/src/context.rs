use crate::params::SPX_N;

#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}
