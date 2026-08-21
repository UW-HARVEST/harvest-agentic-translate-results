//! Translation of `app/src/wotsx1.c` and the `leaf_info_x1` struct from
//! `app/include/wotsx1.h`.

use crate::address::{SPX_set_chain_addr, SPX_set_hash_addr, SPX_set_keypair_addr, SPX_set_type};
use crate::backend::{SPX_prf_addr, SPX_thash};
use crate::context::SpxCtx;
use crate::params::*;

/// Interface to the internal `wots_gen_leafx1` routine.
#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut u8,
    /// The index of the WOTS we're using to sign.
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl leaf_info_x1 {
    pub fn zeroed() -> Self {
        leaf_info_x1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

/// Generates a WOTS public key, and also the WOTS signature if `leaf_info`
/// indicates that we're signing with this WOTS key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut leaf_info_x1,
) {
    let info = v_info;
    let leaf_addr = (*info).leaf_addr.as_mut_ptr();
    let pk_addr = (*info).pk_addr.as_mut_ptr();

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == (*info).wots_sign_leaf {
        // We're traversing the leaf that's signing; generate the WOTS signature.
        0
    } else {
        // Nope, we're just generating pk's; turn off the signature logic.
        !0u32
    };

    SPX_set_keypair_addr(leaf_addr, leaf_idx);
    SPX_set_keypair_addr(pk_addr, leaf_idx);

    let mut i: u32 = 0;
    while (i as usize) < SPX_WOTS_LEN {
        let buffer = pk_buffer.as_mut_ptr().add(i as usize * SPX_N);
        // Set wots_k to the step if we're generating a signature, ~0 if not.
        let wots_k: u32 = *(*info).wots_steps.add(i as usize) | wots_k_mask;

        // Start with the secret seed.
        SPX_set_chain_addr(leaf_addr, i);
        SPX_set_hash_addr(leaf_addr, 0);
        SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        SPX_prf_addr(buffer, ctx, leaf_addr);

        SPX_set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        // Iterate down the WOTS chain.
        let mut k: u32 = 0;
        loop {
            // Check if this is the value that needs to be saved as a part of
            // the WOTS signature.
            if k == wots_k {
                core::ptr::copy_nonoverlapping(
                    buffer,
                    (*info).wots_sig.add(i as usize * SPX_N),
                    SPX_N,
                );
            }

            // Check if we hit the top of the chain.
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            // Iterate one step on the chain.
            SPX_set_hash_addr(leaf_addr, k);
            SPX_thash(buffer, buffer, 1, ctx, leaf_addr);

            k += 1;
        }
        i += 1;
    }

    // Do the final thash to generate the public keys.
    SPX_thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as u32, ctx, pk_addr);
}
