//! Translation of `app/src/wotsx1.c` / `app/include/wotsx1.h`.

use crate::address::{
    set_chain_addr, set_hash_addr, set_keypair_addr, set_type, SPX_ADDR_TYPE_WOTS,
    SPX_ADDR_TYPE_WOTSPRF,
};
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W};

/// Rust counterpart of the C `struct leaf_info_x1` -- laid out and typed
/// exactly like the C struct, so it doubles as the ABI type.
///
/// `wots_sig` and `wots_steps` are kept as RAW POINTERS on purpose.  The C
/// dereferences `info->wots_steps[i]` unconditionally (`wotsx1.c:41`) and
/// `info->wots_sig + i*SPX_N` whenever `k == wots_k` (`wotsx1.c:58`, reached
/// exactly when `leaf_idx == wots_sign_leaf`).  An earlier version of this
/// translation modelled them as `Option<&mut [u8]>` / an inline array and
/// silently skipped the access when the pointer was NULL -- that produced a
/// *different answer* instead of the C's fault, so the raw pointers are the
/// faithful choice.
#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    /// The index of the WOTS we're using to sign
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    /// Equivalent of `struct leaf_info_x1 info = { 0 };`
    pub fn new() -> Self {
        LeafInfoX1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
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
/// It also generates the WOTS signature if leaf_info indicates
/// that we're signing with this WOTS key.
///
/// # Safety
/// `info.wots_steps` must point at `SPX_WOTS_LEN` readable `u32`s, and -- when
/// `leaf_idx == info.wots_sign_leaf` -- `info.wots_sig` must point at
/// `SPX_WOTS_BYTES` writable bytes.  These are exactly the C preconditions.
pub unsafe fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
) {
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
        // `uint32_t wots_k = info->wots_steps[i] | wots_k_mask;` -- an
        // unconditional load, NULL included.
        let wots_k: u32 = unsafe { *info.wots_steps.add(i as usize) } | wots_k_mask;

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
                // `memcpy(info->wots_sig + i * SPX_N, buffer, SPX_N);` -- also
                // unconditional in C.
                let off = i as usize * SPX_N;
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        pk_buffer[buf_off..].as_ptr(),
                        info.wots_sig.add(off),
                        SPX_N,
                    );
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

/// `leaf_info_x1` is `#[repr(C)]` and field-for-field identical to the C
/// struct, so the ABI type is just [`LeafInfoX1`] itself; no marshalling (and
/// hence no NULL-pointer special-casing) happens at the boundary.
pub type CLeafInfoX1 = LeafInfoX1;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut CLeafInfoX1,
) {
    unsafe {
        let dest = core::slice::from_raw_parts_mut(dest, SPX_N);
        wots_gen_leafx1(dest, &*ctx, leaf_idx, &mut *v_info);
    }
}
