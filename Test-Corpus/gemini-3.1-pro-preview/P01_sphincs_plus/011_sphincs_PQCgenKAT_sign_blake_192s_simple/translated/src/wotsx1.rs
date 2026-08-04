use crate::params::*;
use crate::context::SpxCtx;
use crate::thash::thash;
use crate::hash::prf_addr;
use crate::address::{set_keypair_addr, set_chain_addr, set_hash_addr, set_type};

pub struct LeafInfoX1<'a> {
    pub wots_sig: Option<&'a mut [u8]>,
    pub wots_sign_leaf: u32,
    pub wots_steps: &'a [u32],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };
    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        let mut buffer = vec![0u8; SPX_N];
        prf_addr(&mut buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k = 0;
        loop {
            if k == wots_k {
                if let Some(ref mut sig) = info.wots_sig {
                    sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&buffer);
                }
            }
            if k == (SPX_WOTS_W as u32) - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut temp = vec![0u8; SPX_N];
            thash(&mut temp, &buffer, 1, ctx, &info.leaf_addr);
            buffer.copy_from_slice(&temp);
            k += 1;
        }
        pk_buffer[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&buffer);
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}
