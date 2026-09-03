use core::ffi::c_void;

use crate::context::SpxCtx;
use crate::params::*;

#[repr(C)]
pub struct fors_gen_leaf_info {
    pub leaf_addrx: [u32; 8],
}

unsafe fn fors_gen_sk(sk: *mut u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    crate::hash::SPX_prf_addr(sk, ctx, fors_leaf_addr as *const u32);
}

unsafe fn fors_sk_to_leaf(
    leaf: *mut u8,
    sk: *const u8,
    ctx: *const SpxCtx,
    fors_leaf_addr: *mut u32,
) {
    crate::hash::SPX_thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut c_void,
) {
    let fors_info = info as *mut fors_gen_leaf_info;
    let fors_leaf_addr = (*fors_info).leaf_addrx.as_mut_ptr();

    /* Only set the parts that the caller doesn't set */
    crate::address::SPX_set_tree_index(fors_leaf_addr, addr_idx);
    crate::address::SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    crate::address::SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    fors_sk_to_leaf(leaf, leaf, ctx, fors_leaf_addr);
}

/// Interprets m as SPX_FORS_HEIGHT-bit unsigned integers.
/// Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
/// Assumes indices has space for SPX_FORS_TREES integers.
unsafe fn message_to_indices(indices: *mut u32, m: *const u8) {
    let mut i: core::ffi::c_uint;
    let mut j: core::ffi::c_uint;
    let mut offset: core::ffi::c_uint = 0;

    i = 0;
    while i < SPX_FORS_TREES {
        *indices.offset(i as isize) = 0;
        j = 0;
        while j < SPX_FORS_HEIGHT {
            *indices.offset(i as isize) ^= ((((*m.offset((offset >> 3) as isize) as core::ffi::c_uint)
                >> (offset & 0x7))
                & 1u32)
                << j) as u32;
            offset = offset.wrapping_add(1);
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
}

/// Signs a message m, deriving the secret key from sk_seed and the FTS address.
/// Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut sig = sig;
    let mut indices: [u32; SPX_FORS_TREES as usize] = [0; SPX_FORS_TREES as usize];
    let mut roots: [u8; SPX_FORS_TREES as usize * SPX_N] =
        [0u8; SPX_FORS_TREES as usize * SPX_N];
    let mut fors_tree_addr: [u32; 8] = [0; 8];
    let mut fors_info = fors_gen_leaf_info { leaf_addrx: [0; 8] };
    let fors_leaf_addr = fors_info.leaf_addrx.as_mut_ptr();
    let mut fors_pk_addr: [u32; 8] = [0; 8];
    let mut idx_offset: u32;
    let mut i: core::ffi::c_uint;

    crate::address::SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    crate::address::SPX_copy_keypair_addr(fors_leaf_addr, fors_addr);

    crate::address::SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);
    crate::address::SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    i = 0;
    while i < SPX_FORS_TREES {
        idx_offset = i.wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        crate::address::SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        crate::address::SPX_set_tree_index(
            fors_tree_addr.as_mut_ptr(),
            indices[i as usize].wrapping_add(idx_offset),
        );
        crate::address::SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPRF);

        /* Include the secret key part that produces the selected leaf node. */
        fors_gen_sk(sig, ctx, fors_tree_addr.as_mut_ptr());
        crate::address::SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
        sig = sig.add(SPX_N);

        /* Compute the authentication path for this leaf node. */
        crate::utilsx1::SPX_fors_treehashx1(
            roots.as_mut_ptr().add(i as usize * SPX_N),
            sig,
            ctx,
            indices[i as usize],
            idx_offset,
            SPX_FORS_HEIGHT,
            fors_tree_addr.as_mut_ptr(),
            &mut fors_info as *mut fors_gen_leaf_info as *mut c_void,
        );

        sig = sig.add(SPX_N * SPX_FORS_HEIGHT as usize);
        i = i.wrapping_add(1);
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    crate::hash::SPX_thash(
        pk,
        roots.as_ptr(),
        SPX_FORS_TREES,
        ctx,
        fors_pk_addr.as_mut_ptr(),
    );
}

/// Derives the FORS public key from a signature.
/// This can be used for verification by comparing to a known public key, or to
/// subsequently verify a signature on the derived public key. The latter is the
/// typical use-case when used as an FTS below an OTS in a hypertree.
/// Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut sig = sig;
    let mut indices: [u32; SPX_FORS_TREES as usize] = [0; SPX_FORS_TREES as usize];
    let mut roots: [u8; SPX_FORS_TREES as usize * SPX_N] =
        [0u8; SPX_FORS_TREES as usize * SPX_N];
    let mut leaf: [u8; SPX_N] = [0u8; SPX_N];
    let mut fors_tree_addr: [u32; 8] = [0; 8];
    let mut fors_pk_addr: [u32; 8] = [0; 8];
    let mut idx_offset: u32;
    let mut i: core::ffi::c_uint;

    crate::address::SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    crate::address::SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);

    crate::address::SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
    crate::address::SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    i = 0;
    while i < SPX_FORS_TREES {
        idx_offset = i.wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        crate::address::SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        crate::address::SPX_set_tree_index(
            fors_tree_addr.as_mut_ptr(),
            indices[i as usize].wrapping_add(idx_offset),
        );

        /* Derive the leaf from the included secret key part. */
        fors_sk_to_leaf(leaf.as_mut_ptr(), sig, ctx, fors_tree_addr.as_mut_ptr());
        sig = sig.add(SPX_N);

        /* Derive the corresponding root node of this tree. */
        crate::utils::SPX_compute_root(
            roots.as_mut_ptr().add(i as usize * SPX_N),
            leaf.as_ptr(),
            indices[i as usize],
            idx_offset,
            sig,
            SPX_FORS_HEIGHT,
            ctx,
            fors_tree_addr.as_mut_ptr(),
        );
        sig = sig.add(SPX_N * SPX_FORS_HEIGHT as usize);
        i = i.wrapping_add(1);
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    crate::hash::SPX_thash(
        pk,
        roots.as_ptr(),
        SPX_FORS_TREES,
        ctx,
        fors_pk_addr.as_mut_ptr(),
    );
}
