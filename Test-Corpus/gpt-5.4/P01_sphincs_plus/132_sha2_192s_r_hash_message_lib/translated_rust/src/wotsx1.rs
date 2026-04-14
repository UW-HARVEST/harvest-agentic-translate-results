use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;

pub struct LeafInfoX1<'a> {
    pub wots_sig: Option<&'a mut [u8]>,
    pub wots_sign_leaf: u32,
    pub wots_steps: &'a [u32],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1<'_>) {
    let leaf_addr = &mut info.leaf_addr;
    let pk_addr = &mut info.pk_addr;
    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];
    let wots_k_mask = if leaf_idx == info.wots_sign_leaf { 0 } else { u32::MAX };
    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);
    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];
        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(buffer, ctx, leaf_addr);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);
        let mut k = 0u32;
        loop {
            if k == wots_k {
                if let Some(sig) = info.wots_sig.as_deref_mut() {
                    sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buffer);
                }
            }
            if k == (SPX_WOTS_W - 1) as u32 {
                break;
            }
            set_hash_addr(leaf_addr, k);
            let out = thash(buffer, 1, ctx, leaf_addr);
            buffer.copy_from_slice(&out);
            k += 1;
        }
    }
    let out = thash(&pk_buffer, SPX_WOTS_LEN, ctx, pk_addr);
    dest[..SPX_N].copy_from_slice(&out);
}
