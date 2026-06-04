use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type};
use crate::context::SpxCtx;
use crate::hash::SPX_prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::wots::LeafInfoX1;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    unsafe {
        let info = &mut *v_info;
        let leaf_addr = info.leaf_addr.as_mut_ptr();
        let pk_addr = info.pk_addr.as_mut_ptr();

        let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

        let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
            0
        } else {
            !0u32
        };

        set_keypair_addr(leaf_addr, leaf_idx);
        set_keypair_addr(pk_addr, leaf_idx);

        for i in 0..SPX_WOTS_LEN {
            let buffer_off = i * SPX_N;
            let wots_k = (*info.wots_steps.add(i)) | wots_k_mask;

            set_chain_addr(leaf_addr, i as u32);
            set_hash_addr(leaf_addr, 0);
            set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

            SPX_prf_addr(pk_buffer.as_mut_ptr().add(buffer_off), ctx, leaf_addr);

            set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

            let mut k: u32 = 0;
            loop {
                if k == wots_k {
                    std::ptr::copy_nonoverlapping(
                        pk_buffer.as_ptr().add(buffer_off),
                        info.wots_sig.add(i * SPX_N),
                        SPX_N,
                    );
                }

                if k == (SPX_WOTS_W as u32) - 1 {
                    break;
                }

                set_hash_addr(leaf_addr, k);
                thash(
                    pk_buffer.as_mut_ptr().add(buffer_off),
                    pk_buffer.as_ptr().add(buffer_off),
                    1,
                    ctx,
                    leaf_addr,
                );
                k += 1;
            }
        }

        thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as u32, ctx, info.pk_addr.as_mut_ptr());
    }
}
