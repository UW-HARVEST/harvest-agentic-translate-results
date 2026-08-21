//! Translation of `app/src/fors.c` and the `fors_gen_leaf_info` struct
//! (`app/include/fors.h`) / `fors_gen_leafx1` (`app/include/forsx1.h`).

use crate::address::{
    SPX_copy_keypair_addr, SPX_set_tree_height, SPX_set_tree_index, SPX_set_type,
};
use crate::backend::{SPX_prf_addr, SPX_thash};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_compute_root;
use crate::utilsx1::SPX_fors_treehashx1;

#[repr(C)]
pub struct fors_gen_leaf_info {
    pub leaf_addrx: [u32; 8],
}

impl fors_gen_leaf_info {
    pub fn zeroed() -> Self {
        fors_gen_leaf_info {
            leaf_addrx: [0u32; 8],
        }
    }
}

unsafe fn fors_gen_sk(sk: *mut u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    SPX_prf_addr(sk, ctx, fors_leaf_addr);
}

unsafe fn fors_sk_to_leaf(leaf: *mut u8, sk: *const u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    SPX_thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut fors_gen_leaf_info,
) {
    let fors_leaf_addr = (*info).leaf_addrx.as_mut_ptr();

    // Only set the parts that the caller doesn't set.
    SPX_set_tree_index(fors_leaf_addr, addr_idx);
    SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    SPX_set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    fors_sk_to_leaf(leaf, leaf, ctx, fors_leaf_addr);
}

/// Interprets `m` as SPX_FORS_HEIGHT-bit unsigned integers.
unsafe fn message_to_indices(indices: *mut u32, m: *const u8) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        *indices.add(i) = 0;
        for j in 0..SPX_FORS_HEIGHT {
            *indices.add(i) ^=
                (((*m.add(offset >> 3) >> (offset & 0x7)) & 1u8) as u32) << j;
            offset += 1;
        }
    }
}

/// Signs a message m, deriving the secret key from sk_seed and the FTS address.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    mut sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = fors_gen_leaf_info::zeroed();
    let mut fors_pk_addr = [0u32; 8];

    SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    SPX_copy_keypair_addr(fors_info.leaf_addrx.as_mut_ptr(), fors_addr);

    SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);
    SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        SPX_set_tree_index(fors_tree_addr.as_mut_ptr(), indices[i] + idx_offset);
        SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPRF);

        // Include the secret key part that produces the selected leaf node.
        fors_gen_sk(sig, ctx, fors_tree_addr.as_mut_ptr());
        SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
        sig = sig.add(SPX_N);

        // Compute the authentication path for this leaf node.
        SPX_fors_treehashx1(
            roots.as_mut_ptr().add(i * SPX_N),
            sig,
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            fors_tree_addr.as_mut_ptr(),
            &mut fors_info,
        );

        sig = sig.add(SPX_N * SPX_FORS_HEIGHT);
    }

    // Hash horizontally across all tree roots to derive the public key.
    SPX_thash(pk, roots.as_ptr(), SPX_FORS_TREES as u32, ctx, fors_pk_addr.as_mut_ptr());
}

/// Derives the FORS public key from a signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    mut sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    SPX_copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
    SPX_copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);

    SPX_set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
    SPX_set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

    message_to_indices(indices.as_mut_ptr(), m);

    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        SPX_set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
        SPX_set_tree_index(fors_tree_addr.as_mut_ptr(), indices[i] + idx_offset);

        // Derive the leaf from the included secret key part.
        fors_sk_to_leaf(leaf.as_mut_ptr(), sig, ctx, fors_tree_addr.as_mut_ptr());
        sig = sig.add(SPX_N);

        // Derive the corresponding root node of this tree.
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

    // Hash horizontally across all tree roots to derive the public key.
    SPX_thash(pk, roots.as_ptr(), SPX_FORS_TREES as u32, ctx, fors_pk_addr.as_mut_ptr());
}
