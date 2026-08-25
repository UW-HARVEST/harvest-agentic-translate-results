//! Translation of `app/src/wotsx1.c` (+ `app/include/wotsx1.h`).

use crate::address::{SPX_set_chain_addr, SPX_set_hash_addr, SPX_set_keypair_addr, SPX_set_type};
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF, SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W,
};

/*
 * This is here to provide an interface to the internal wots_gen_leafx1
 * routine.  While this routine is not referenced in the package outside of
 * wots.c, it is called from the stand-alone benchmark code to characterize
 * the performance
 */
#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32, /* The index of the WOTS we're using to sign */
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/*
 * This generates a WOTS public key
 * It also generates the WOTS signature if leaf_info indicates
 * that we're signing with this WOTS key
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut leaf_info_x1,
) {
    unsafe {
        let info: *mut leaf_info_x1 = v_info;
        let leaf_addr: *mut u32 = (*info).leaf_addr.as_mut_ptr();
        let pk_addr: *mut u32 = (*info).pk_addr.as_mut_ptr();
        let mut i: u32;
        let mut k: u32;
        let mut pk_buffer: [u8; SPX_WOTS_BYTES] = [0u8; SPX_WOTS_BYTES];
        let mut buffer: *mut u8;
        let wots_k_mask: u32;

        if leaf_idx == (*info).wots_sign_leaf {
            /* We're traversing the leaf that's signing; generate the WOTS */
            /* signature */
            wots_k_mask = 0;
        } else {
            /* Nope, we're just generating pk's; turn off the signature logic */
            wots_k_mask = !0u32;
        }

        SPX_set_keypair_addr(leaf_addr, leaf_idx);
        SPX_set_keypair_addr(pk_addr, leaf_idx);

        i = 0;
        buffer = pk_buffer.as_mut_ptr();
        while (i as usize) < SPX_WOTS_LEN {
            /* Set wots_k to the step if we're generating a signature, ~0 if
             * we're not */
            let wots_k: u32 = *(*info).wots_steps.add(i as usize) | wots_k_mask;

            /* Start with the secret seed */
            SPX_set_chain_addr(leaf_addr, i);
            SPX_set_hash_addr(leaf_addr, 0);
            SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

            prf_addr(buffer, ctx, leaf_addr as *const u32);

            SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

            /* Iterate down the WOTS chain */
            k = 0;
            loop {
                /* Check if this is the value that needs to be saved as a */
                /* part of the WOTS signature */
                if k == wots_k {
                    core::ptr::copy_nonoverlapping(
                        buffer as *const u8,
                        (*info).wots_sig.add(i as usize * SPX_N),
                        SPX_N,
                    );
                }

                /* Check if we hit the top of the chain */
                if k == (SPX_WOTS_W as u32).wrapping_sub(1) {
                    break;
                }

                /* Iterate one step on the chain */
                SPX_set_hash_addr(leaf_addr, k);

                thash(buffer, buffer as *const u8, 1, ctx, leaf_addr);

                k = k.wrapping_add(1);
            }

            i = i.wrapping_add(1);
            buffer = buffer.add(SPX_N);
        }

        /* Do the final thash to generate the public keys */
        thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as u32, ctx, pk_addr);
    }
}
