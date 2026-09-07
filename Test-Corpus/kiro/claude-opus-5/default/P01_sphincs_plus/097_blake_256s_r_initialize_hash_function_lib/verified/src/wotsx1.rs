use core::ffi::{c_uint, c_void};

use crate::context::SpxCtx;
use crate::params::*;

/*
 * This is here to provide an interface to the internal wots_gen_leafx1
 * routine.
 */
#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut u8,
    /* The index of the WOTS we're using to sign */
    pub wots_sign_leaf: u32,
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
    v_info: *mut c_void,
) {
    let info = v_info as *mut leaf_info_x1;
    let leaf_addr: *mut u32 = (*info).leaf_addr.as_mut_ptr();
    let pk_addr: *mut u32 = (*info).pk_addr.as_mut_ptr();
    let mut i: c_uint;
    let mut k: c_uint;
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

    crate::address::SPX_set_keypair_addr(leaf_addr, leaf_idx);
    crate::address::SPX_set_keypair_addr(pk_addr, leaf_idx);

    i = 0;
    buffer = pk_buffer.as_mut_ptr();
    while (i as usize) < SPX_WOTS_LEN {
        /* Set wots_k to the step if we're generating a signature, ~0 if we're not */
        let wots_k: u32 = *(*info).wots_steps.add(i as usize) | wots_k_mask;

        /* Start with the secret seed */
        crate::address::SPX_set_chain_addr(leaf_addr, i as u32);
        crate::address::SPX_set_hash_addr(leaf_addr, 0);
        crate::address::SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        crate::hash::SPX_prf_addr(buffer, ctx, leaf_addr);

        crate::address::SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        /* Iterate down the WOTS chain */
        k = 0;
        loop {
            /* Check if this is the value that needs to be saved as a */
            /* part of the WOTS signature */
            if k == wots_k {
                core::ptr::copy_nonoverlapping(
                    buffer,
                    (*info).wots_sig.add(i as usize * SPX_N),
                    SPX_N,
                );
            }

            /* Check if we hit the top of the chain */
            if k == SPX_WOTS_W - 1 {
                break;
            }

            /* Iterate one step on the chain */
            crate::address::SPX_set_hash_addr(leaf_addr, k as u32);

            crate::hash::SPX_thash(buffer, buffer, 1, ctx, leaf_addr);

            k = k.wrapping_add(1);
        }

        i = i.wrapping_add(1);
        buffer = buffer.add(SPX_N);
    }

    /* Do the final thash to generate the public keys */
    crate::hash::SPX_thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as c_uint, ctx, pk_addr);
}
