use crate::params::*;

#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

#[repr(C)]
#[derive(Clone)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

#[repr(C)]
#[derive(Clone)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}
