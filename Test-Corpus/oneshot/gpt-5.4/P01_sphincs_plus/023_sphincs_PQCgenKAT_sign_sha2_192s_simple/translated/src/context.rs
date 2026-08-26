use crate::params::SPX_N;

#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self {
            pub_seed: [0; SPX_N],
            sk_seed: [0; SPX_N],
        }
    }
}
