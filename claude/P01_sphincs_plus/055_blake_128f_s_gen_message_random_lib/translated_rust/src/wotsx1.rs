use crate::address::*;
use crate::context::SpxCtx;
use crate::hash;
use crate::params::*;
use crate::thash;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: Vec<u32>,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn new() -> Self {
        LeafInfoX1 {
            wots_sig: Vec::new(),
            wots_sign_leaf: 0,
            wots_steps: Vec::new(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let leaf_addr = &mut info.leaf_addr;
    let pk_addr = &mut info.pk_addr;

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        hash::prf_addr(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], ctx, leaf_addr);

        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N]
                    .copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }

            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            set_hash_addr(leaf_addr, k);

            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            thash::thash(
                &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N],
                &tmp,
                1,
                ctx,
                leaf_addr,
            );
        }
    }

    thash::thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, pk_addr);
}
