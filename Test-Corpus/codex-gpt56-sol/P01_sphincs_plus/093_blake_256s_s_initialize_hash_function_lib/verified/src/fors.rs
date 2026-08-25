//! Translation of `c_src/app/src/fors.c` (plus the declarations of
//! `c_src/app/include/fors.h` and `c_src/app/include/forsx1.h`).

use core::ptr::addr_of_mut;

use crate::address::{
    SPX_copy_keypair_addr, SPX_set_tree_height, SPX_set_tree_index, SPX_set_type,
};
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_TYPE_FORSPK, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE, SPX_FORS_HEIGHT,
    SPX_FORS_TREES, SPX_N,
};
use crate::utils::SPX_compute_root;
use crate::utilsx1::SPX_fors_treehashx1;
use crate::wotsx1::leaf_info_x1;

/// `typedef struct fors_gen_leaf_info { uint32_t leaf_addrx[8]; } fors_gen_leaf_info;`
#[repr(C)]
pub struct fors_gen_leaf_info {
    pub leaf_addrx: [u32; 8],
}

unsafe fn fors_gen_sk(sk: *mut u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    prf_addr(sk, ctx, fors_leaf_addr as *const u32);
}

unsafe fn fors_sk_to_leaf(
    leaf: *mut u8,
    sk: *const u8,
    ctx: *const SpxCtx,
    fors_leaf_addr: *mut u32,
) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut fors_gen_leaf_info,
) {
    let fors_info: *mut fors_gen_leaf_info = info;
    let fors_leaf_addr: *mut u32 = addr_of_mut!((*fors_info).leaf_addrx) as *mut u32;

    /* Only set the parts that the caller doesn't set */
    SPX_set_tree_index(fors_leaf_addr, addr_idx);
    SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    fors_sk_to_leaf(leaf, leaf as *const u8, ctx, fors_leaf_addr);
}

/// Interprets m as SPX_FORS_HEIGHT-bit unsigned integers.
/// Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
/// Assumes indices has space for SPX_FORS_TREES integers.
unsafe fn message_to_indices(indices: *mut u32, m: *const u8) {
    let mut offset: u32 = 0;

    for i in 0..SPX_FORS_TREES {
        *indices.add(i) = 0;
        for j in 0..SPX_FORS_HEIGHT {
            let byte = *m.add((offset >> 3) as usize);
            *indices.add(i) ^= (((byte >> (offset & 0x7)) & 1u8) as u32) << j;
            offset = offset.wrapping_add(1);
        }
    }
}

/// Signs a message m, deriving the secret key from sk_seed and the FORS address.
/// Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut sig: *mut u8 = sig;
    let mut indices: [u32; SPX_FORS_TREES] = [0u32; SPX_FORS_TREES];
    let mut roots: [u8; SPX_FORS_TREES * SPX_N] = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr: [u32; 8] = [0u32; 8];
    let mut fors_info = fors_gen_leaf_info {
        leaf_addrx: [0u32; 8],
    };
    let fors_leaf_addr: *mut u32 = fors_info.leaf_addrx.as_mut_ptr();
    let mut fors_pk_addr: [u32; 8] = [0u32; 8];

    SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    SPX_copy_keypair_addr(fors_leaf_addr, fors_addr);

    SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);
    SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32).wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        SPX_set_tree_index(
            fors_tree_addr.as_mut_ptr(),
            indices[i].wrapping_add(idx_offset),
        );
        SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPRF);

        /* Include the secret key part that produces the selected leaf node. */
        fors_gen_sk(sig, ctx, fors_tree_addr.as_mut_ptr());
        SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
        sig = sig.add(SPX_N);

        /* Compute the authentication path for this leaf node. */
        SPX_fors_treehashx1(
            roots.as_mut_ptr().add(i * SPX_N),
            sig,
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            fors_tree_addr.as_mut_ptr(),
            &mut fors_info as *mut fors_gen_leaf_info as *mut leaf_info_x1,
        );

        sig = sig.add(SPX_N * SPX_FORS_HEIGHT);
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        pk,
        roots.as_ptr(),
        SPX_FORS_TREES as u32,
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
    let mut sig: *const u8 = sig;
    let mut indices: [u32; SPX_FORS_TREES] = [0u32; SPX_FORS_TREES];
    let mut roots: [u8; SPX_FORS_TREES * SPX_N] = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf: [u8; SPX_N] = [0u8; SPX_N];
    let mut fors_tree_addr: [u32; 8] = [0u32; 8];
    let mut fors_pk_addr: [u32; 8] = [0u32; 8];

    SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);

    SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
    SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32).wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        SPX_set_tree_index(
            fors_tree_addr.as_mut_ptr(),
            indices[i].wrapping_add(idx_offset),
        );

        /* Derive the leaf from the included secret key part. */
        fors_sk_to_leaf(leaf.as_mut_ptr(), sig, ctx, fors_tree_addr.as_mut_ptr());
        sig = sig.add(SPX_N);

        /* Derive the corresponding root node of this tree. */
        SPX_compute_root(
            roots.as_mut_ptr().add(i * SPX_N),
            leaf.as_ptr(),
            indices[i],
            idx_offset,
            sig,
            SPX_FORS_HEIGHT as u32,
            ctx,
            fors_tree_addr.as_mut_ptr(),
        );
        sig = sig.add(SPX_N * SPX_FORS_HEIGHT);
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        pk,
        roots.as_ptr(),
        SPX_FORS_TREES as u32,
        ctx,
        fors_pk_addr.as_mut_ptr(),
    );
}
