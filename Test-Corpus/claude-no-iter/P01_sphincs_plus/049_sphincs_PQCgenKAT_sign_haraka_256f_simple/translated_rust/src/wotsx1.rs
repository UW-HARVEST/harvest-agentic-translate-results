// Translation of c_src/app/include/wotsx1.h and c_src/app/src/wotsx1.c

use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::{
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF, SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W,
};
use crate::thash::thash;

pub struct LeafInfoX1<'a> {
    /// `wots_sig` may be empty when called from the benchmark code; in our
    /// implementation it is always either an empty slice (sentinel) or a slice
    /// of the signature buffer. Length is `SPX_WOTS_LEN * SPX_N`.
    pub wots_sig: &'a mut [u8],
    pub wots_sign_leaf: u32,
    pub wots_steps: &'a [u32],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask = if leaf_idx == info.wots_sign_leaf {
        0u32
    } else {
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        let buf_off = i * SPX_N;
        prf_addr(
            &mut pk_buffer[buf_off..buf_off + SPX_N],
            ctx,
            &info.leaf_addr,
        );

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k = 0u32;
        loop {
            if k == wots_k && !info.wots_sig.is_empty() {
                let sig_off = i * SPX_N;
                info.wots_sig[sig_off..sig_off + SPX_N]
                    .copy_from_slice(&pk_buffer[buf_off..buf_off + SPX_N]);
            }

            if k == (SPX_WOTS_W as u32 - 1) {
                break;
            }

            set_hash_addr(&mut info.leaf_addr, k);

            let in_block = pk_buffer[buf_off..buf_off + SPX_N].to_vec();
            thash(
                &mut pk_buffer[buf_off..buf_off + SPX_N],
                &in_block,
                1,
                ctx,
                &mut info.leaf_addr,
            );
            k += 1;
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}
