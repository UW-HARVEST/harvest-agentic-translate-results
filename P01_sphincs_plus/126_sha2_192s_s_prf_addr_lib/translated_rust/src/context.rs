use crate::params::*;

#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    pub state_seeded_512: [u8; 72],
}

impl Default for SpxCtx {
    fn default() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            state_seeded: [0u8; 40],
            state_seeded_512: [0u8; 72],
        }
    }
}

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

#[repr(C)]
#[derive(Clone, Default)]
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

impl Default for LeafInfoX1 {
    fn default() -> Self {
        LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: std::ptr::null_mut(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}
