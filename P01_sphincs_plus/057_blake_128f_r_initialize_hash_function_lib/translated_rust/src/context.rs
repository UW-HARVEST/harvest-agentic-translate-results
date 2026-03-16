use crate::params::*;

#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

pub fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

pub fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}
