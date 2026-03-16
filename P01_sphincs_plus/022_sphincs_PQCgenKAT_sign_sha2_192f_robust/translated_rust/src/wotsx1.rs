use crate::params::*;

pub struct LeafInfoX1 {
    pub wots_sig: Option<Vec<u8>>,
    pub wots_sign_leaf: u32,
    pub wots_steps: Vec<u32>,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn new() -> Self {
        LeafInfoX1 {
            wots_sig: None,
            wots_sign_leaf: !0u32,
            wots_steps: vec![0u32; SPX_WOTS_LEN],
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}
