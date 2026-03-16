use crate::context::{SpxCtx, LeafInfoX1};
use crate::params::*;
use crate::utils::*;

pub fn wots_gen_leafx1_internal(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr_internal(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr_internal(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr_internal(&mut info.leaf_addr, i as u32);
        set_hash_addr_internal(&mut info.leaf_addr, 0);
        set_type_internal(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        crate::hash::prf_addr_internal(buffer, ctx, &info.leaf_addr);

        set_type_internal(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            buffer.as_ptr(),
                            info.wots_sig.add(i * SPX_N),
                            SPX_N,
                        );
                    }
                }
            }
            if k == (SPX_WOTS_W as u32) - 1 {
                break;
            }
            set_hash_addr_internal(&mut info.leaf_addr, k);
            let tmp = buffer.to_vec();
            crate::thash::thash_internal(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    crate::thash::thash_internal(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}
