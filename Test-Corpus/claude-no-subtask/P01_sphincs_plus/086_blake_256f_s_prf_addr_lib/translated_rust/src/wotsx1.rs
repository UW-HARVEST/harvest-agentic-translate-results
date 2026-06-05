// WOTSx1 implementation

use crate::address::{self, *};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;

#[derive(Clone)]
pub struct LeafInfoX1 {
    pub wots_sig_off: usize, // offset into shared signature buffer; signed buffer pointer can't be stored as &mut here
    pub wots_sign_leaf: u32,
    pub wots_steps: Vec<u32>,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn new() -> Self {
        Self {
            wots_sig_off: 0,
            wots_sign_leaf: 0,
            wots_steps: vec![0u32; SPX_WOTS_LEN],
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

pub fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
    wots_sig: Option<&mut [u8]>,
) {
    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut sig_view = wots_sig;

    for i in 0..SPX_WOTS_LEN {
        let buffer_off = i * SPX_N;
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        address::set_type(&mut info.leaf_addr, address::SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(
            &mut pk_buffer[buffer_off..buffer_off + SPX_N],
            ctx,
            &info.leaf_addr,
        );

        address::set_type(&mut info.leaf_addr, address::SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k {
                if let Some(ref mut sig) = sig_view {
                    let sig_off = i * SPX_N;
                    sig[sig_off..sig_off + SPX_N]
                        .copy_from_slice(&pk_buffer[buffer_off..buffer_off + SPX_N]);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let in_data = pk_buffer[buffer_off..buffer_off + SPX_N].to_vec();
            thash(
                &mut pk_buffer[buffer_off..buffer_off + SPX_N],
                &in_data,
                1,
                ctx,
                &mut info.leaf_addr,
            );
            k += 1;
        }
    }

    let mut pk_addr = info.pk_addr;
    thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut pk_addr);
}
