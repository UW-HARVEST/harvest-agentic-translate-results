//! Translation of `app/src/wotsx1.c` and `app/include/wotsx1.h`.

use crate::address::*;
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::*;

/// `struct leaf_info_x1` exactly as declared in `app/include/wotsx1.h`.
///
/// Only used to describe the C ABI of [`SPX_wots_gen_leafx1`] and
/// [`crate::utilsx1::SPX_wots_treehashx1`]; the internal code path uses
/// [`LeafInfoX1`] instead.
#[repr(C)]
pub struct LeafInfoX1Raw {
    pub wots_sig: *mut u8,
    /// The index of the WOTS we're using to sign.
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// The same information as `struct leaf_info_x1`, but with the two buffers the
/// C struct points at kept separate so that borrows stay checkable.
pub struct LeafInfoX1 {
    /// The index of the WOTS we're using to sign.
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub const fn new() -> Self {
        LeafInfoX1 {
            wots_sign_leaf: 0,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

impl Default for LeafInfoX1 {
    fn default() -> Self {
        Self::new()
    }
}

/// This generates a WOTS public key.
///
/// It also generates the WOTS signature if `leaf_info` indicates that we're
/// signing with this WOTS key.
pub fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
    wots_sig: &mut [u8],
) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        /* We're traversing the leaf that's signing; generate the WOTS
           signature */
        0
    } else {
        /* Nope, we're just generating pk's; turn off the signature logic */
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        /* Set wots_k to the step if we're generating a signature, ~0 if not */
        let wots_k = info.wots_steps[i] | wots_k_mask;

        /* Start with the secret seed */
        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        let mut buffer = [0u8; SPX_N];
        prf_addr(&mut buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        /* Iterate down the WOTS chain */
        let mut k: u32 = 0;
        loop {
            /* Check if this is the value that needs to be saved as a part of
               the WOTS signature */
            if k == wots_k {
                wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&buffer);
            }

            /* Check if we hit the top of the chain */
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            /* Iterate one step on the chain */
            set_hash_addr(&mut info.leaf_addr, k);

            let tmp = buffer;
            thash(&mut buffer, &tmp, 1, ctx, &info.leaf_addr);

            k += 1;
        }

        pk_buffer[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&buffer);
    }

    /* Do the final thash to generate the public keys */
    thash(
        &mut dest[..SPX_N],
        &pk_buffer,
        SPX_WOTS_LEN,
        ctx,
        &info.pk_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1Raw,
) {
    unsafe {
        let raw = &mut *v_info;
        let mut info = LeafInfoX1 {
            wots_sign_leaf: raw.wots_sign_leaf,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: raw.leaf_addr,
            pk_addr: raw.pk_addr,
        };
        if !raw.wots_steps.is_null() {
            info.wots_steps
                .copy_from_slice(core::slice::from_raw_parts(raw.wots_steps, SPX_WOTS_LEN));
        }
        // `wots_sig` is only written when `leaf_idx == wots_sign_leaf`, which
        // is exactly when the caller has provided a buffer.
        let mut scratch = [0u8; SPX_WOTS_BYTES];
        let sig: &mut [u8] = if raw.wots_sig.is_null() {
            &mut scratch
        } else {
            core::slice::from_raw_parts_mut(raw.wots_sig, SPX_WOTS_BYTES)
        };
        wots_gen_leafx1(
            core::slice::from_raw_parts_mut(dest, SPX_N),
            &*ctx,
            leaf_idx,
            &mut info,
            sig,
        );
        raw.leaf_addr = info.leaf_addr;
        raw.pk_addr = info.pk_addr;
    }
}
