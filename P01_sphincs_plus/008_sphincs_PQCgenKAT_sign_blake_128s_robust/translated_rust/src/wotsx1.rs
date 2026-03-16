use crate::params::*;
use crate::address;
use crate::hash_blake::{self, SpxCtx};
use crate::thash;
use crate::merkle::LeafInfoX1;

pub fn wots_gen_leafx1(
    dest: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, info: &mut LeafInfoX1,
) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    address::set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    address::set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buf = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        address::set_chain_addr(&mut info.leaf_addr, i as u32);
        address::set_hash_addr(&mut info.leaf_addr, 0);
        address::set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        hash_blake::prf_addr(buf, ctx, &info.leaf_addr);

        address::set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buf);
            }
            if k == (SPX_WOTS_W as u32) - 1 {
                break;
            }
            address::set_hash_addr(&mut info.leaf_addr, k);
            let tmp = buf.to_vec();
            thash::thash(buf, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    thash::thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}
