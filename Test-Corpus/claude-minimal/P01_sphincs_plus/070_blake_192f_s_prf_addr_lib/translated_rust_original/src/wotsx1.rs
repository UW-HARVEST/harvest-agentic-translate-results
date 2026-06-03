use crate::address;
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::{
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF, SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W,
};
use crate::thash::thash;

pub struct LeafInfoX1<'a> {
    pub wots_sig: &'a mut [u8],
    pub wots_sign_leaf: u32,
    pub wots_steps: &'a [u32],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// Generate a WOTS public key.
pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    address::set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    address::set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        address::set_chain_addr(&mut info.leaf_addr, i as u32);
        address::set_hash_addr(&mut info.leaf_addr, 0);
        address::set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], ctx, &info.leaf_addr);

        address::set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k && !info.wots_sig.is_empty() {
                let off = i * SPX_N;
                info.wots_sig[off..off + SPX_N]
                    .copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }

            if k as usize == SPX_WOTS_W - 1 {
                break;
            }

            address::set_hash_addr(&mut info.leaf_addr, k);
            let in_clone = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            thash(
                &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N],
                &in_clone,
                1,
                ctx,
                &mut info.leaf_addr,
            );
            k += 1;
        }
    }

    thash(
        &mut dest[..SPX_N],
        &pk_buffer,
        SPX_WOTS_LEN as u32,
        ctx,
        &mut info.pk_addr,
    );
}
