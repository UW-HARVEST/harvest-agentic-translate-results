//! Translation of `app/src/merkle.c`.

use crate::address::{copy_subtree_addr, set_layer_addr, set_type};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

/// Generates a Merkle signature (WOTS signature followed by the Merkle
/// authentication path).
pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1::new();
    let mut steps = [0u32; SPX_WOTS_LEN];

    let (wots_part, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);
    info.wots_sig = wots_part.as_mut_ptr();
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_ptr();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
    );
}

/// Computes the root node of the top-most subtree.
pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = [0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &mut wots_addr,
        &mut top_tree_addr,
        !0u32,
    );
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let sig_s = core::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let wa = &mut *(wots_addr as *mut [u32; 8]);
    let ta = &mut *(tree_addr as *mut [u32; 8]);
    merkle_sign(sig_s, root_s, &*ctx, wa, ta, idx_leaf);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    merkle_gen_root(root_s, &*ctx);
}
