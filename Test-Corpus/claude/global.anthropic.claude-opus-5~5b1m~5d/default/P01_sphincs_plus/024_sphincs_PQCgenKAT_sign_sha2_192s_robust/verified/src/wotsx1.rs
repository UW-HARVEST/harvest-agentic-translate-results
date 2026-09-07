//! Translation of `app/src/wotsx1.c` / `app/include/wotsx1.h`.

use crate::address::{
    set_chain_addr, set_hash_addr, set_keypair_addr, set_type, SPX_ADDR_TYPE_WOTS,
    SPX_ADDR_TYPE_WOTSPRF,
};
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W};

/// Rust counterpart of the C `struct leaf_info_x1`.
///
/// `wots_sig` points at the start of the WOTS signature area (`NULL` in C is
/// represented by `None`); `wots_steps` is an inline array instead of a
/// pointer since it always has `SPX_WOTS_LEN` entries.
pub struct LeafInfoX1<'a> {
    pub wots_sig: Option<&'a mut [u8]>,
    /// The index of the WOTS we're using to sign
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl<'a> LeafInfoX1<'a> {
    /// Equivalent of `struct leaf_info_x1 info = { 0 };`
    pub fn new() -> Self {
        LeafInfoX1 {
            wots_sig: None,
            wots_sign_leaf: 0,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

impl<'a> Default for LeafInfoX1<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// This generates a WOTS public key.
/// It also generates the WOTS signature if leaf_info indicates
/// that we're signing with this WOTS key.
pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];
    let wots_k_mask: u32;

    if leaf_idx == info.wots_sign_leaf {
        /* We're traversing the leaf that's signing; generate the WOTS */
        /* signature */
        wots_k_mask = 0;
    } else {
        /* Nope, we're just generating pk's; turn off the signature logic */
        wots_k_mask = !0u32;
    }

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut i: u32 = 0;
    while (i as usize) < SPX_WOTS_LEN {
        // `buffer` in C: pk_buffer + i * SPX_N
        let buf_off = i as usize * SPX_N;

        /* Set wots_k to the step if we're generating a signature, ~0 if we're
           not */
        let wots_k: u32 = info.wots_steps[i as usize] | wots_k_mask;

        /* Start with the secret seed */
        set_chain_addr(&mut info.leaf_addr, i);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[buf_off..], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        /* Iterate down the WOTS chain */
        let mut k: u32 = 0;
        loop {
            /* Check if this is the value that needs to be saved as a */
            /* part of the WOTS signature */
            if k == wots_k {
                let off = i as usize * SPX_N;
                if let Some(sig) = info.wots_sig.as_deref_mut() {
                    sig[off..off + SPX_N].copy_from_slice(&pk_buffer[buf_off..buf_off + SPX_N]);
                }
            }

            /* Check if we hit the top of the chain */
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            /* Iterate one step on the chain */
            set_hash_addr(&mut info.leaf_addr, k);

            // thash(buffer, buffer, 1, ...) -- out and in overlap in C.
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[buf_off..buf_off + SPX_N]);
            thash(
                &mut pk_buffer[buf_off..buf_off + SPX_N],
                &tmp,
                1,
                ctx,
                &mut info.leaf_addr,
            );

            k = k.wrapping_add(1);
        }

        i += 1;
    }

    /* Do the final thash to generate the public keys */
    thash(
        &mut dest[..SPX_N],
        &pk_buffer,
        SPX_WOTS_LEN as u32,
        ctx,
        &mut info.pk_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

/// ABI mirror of the C `struct leaf_info_x1`.
#[repr(C)]
pub struct CLeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// Converts a C `leaf_info_x1 *` into a Rust [`LeafInfoX1`], runs `f`, and
/// copies the (possibly modified) address fields back out.
pub(crate) unsafe fn with_leaf_info<R>(
    c_info: *mut CLeafInfoX1,
    f: impl FnOnce(&mut LeafInfoX1) -> R,
) -> R {
    unsafe {
        let ci = &mut *c_info;

        let mut wots_steps = [0u32; SPX_WOTS_LEN];
        if !ci.wots_steps.is_null() {
            core::ptr::copy_nonoverlapping(ci.wots_steps, wots_steps.as_mut_ptr(), SPX_WOTS_LEN);
        }

        let wots_sig: Option<&mut [u8]> = if ci.wots_sig.is_null() {
            None
        } else {
            Some(core::slice::from_raw_parts_mut(ci.wots_sig, SPX_WOTS_BYTES))
        };

        let mut info = LeafInfoX1 {
            wots_sig,
            wots_sign_leaf: ci.wots_sign_leaf,
            wots_steps,
            leaf_addr: ci.leaf_addr,
            pk_addr: ci.pk_addr,
        };

        let r = f(&mut info);

        ci.wots_sign_leaf = info.wots_sign_leaf;
        ci.leaf_addr = info.leaf_addr;
        ci.pk_addr = info.pk_addr;
        if !ci.wots_steps.is_null() {
            core::ptr::copy_nonoverlapping(info.wots_steps.as_ptr(), ci.wots_steps, SPX_WOTS_LEN);
        }

        r
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut CLeafInfoX1,
) {
    unsafe {
        let dest = core::slice::from_raw_parts_mut(dest, SPX_N);
        let ctx = &*ctx;
        with_leaf_info(v_info, |info| {
            wots_gen_leafx1(dest, ctx, leaf_idx, info);
        });
    }
}
