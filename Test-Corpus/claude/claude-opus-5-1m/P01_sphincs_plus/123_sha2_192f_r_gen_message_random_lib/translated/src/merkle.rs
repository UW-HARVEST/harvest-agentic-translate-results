//! Translation of `app/src/merkle.c`.

use crate::address::{SPX_copy_subtree_addr, SPX_set_layer_addr, SPX_set_type};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::SPX_wots_treehashx1;
use crate::wots::SPX_chain_lengths;
use crate::wotsx1::leaf_info_x1;

/// Generates a Merkle signature (WOTS signature followed by the Merkle
/// authentication path).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let auth_path = sig.add(SPX_WOTS_BYTES);
    let mut info = leaf_info_x1::zeroed();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = sig;
    SPX_chain_lengths(steps.as_mut_ptr(), root);
    info.wots_steps = steps.as_mut_ptr();

    SPX_set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    SPX_set_type(info.pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTSPK);
    SPX_copy_subtree_addr(info.leaf_addr.as_mut_ptr(), wots_addr);
    SPX_copy_subtree_addr(info.pk_addr.as_mut_ptr(), wots_addr);

    info.wots_sign_leaf = idx_leaf;

    SPX_wots_treehashx1(
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

/// Compute root node of the top-most subtree.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    // We do not need the auth path in key generation, but it simplifies the
    // code to have just one treehash routine that computes both root and path.
    let mut auth_path = [0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    SPX_set_layer_addr(top_tree_addr.as_mut_ptr(), (SPX_D - 1) as u32);
    SPX_set_layer_addr(wots_addr.as_mut_ptr(), (SPX_D - 1) as u32);

    SPX_merkle_sign(
        auth_path.as_mut_ptr(),
        root,
        ctx,
        wots_addr.as_mut_ptr(),
        top_tree_addr.as_mut_ptr(),
        !0u32, // ~0 means "don't bother generating an auth path"
    );
}
