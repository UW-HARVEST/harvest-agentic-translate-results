//! Translation of `app/src/merkle.c` and `app/include/merkle.h`.

use crate::address::{
    addr_mut, addr_ref, copy_subtree_addr, set_layer_addr, set_type, Addr, ZERO_ADDR,
    SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

/// This generates a Merkle signature (WOTS signature followed by the Merkle
/// authentication path).
///
/// This is in this file because most of the complexity is involved with the
/// WOTS signature; the Merkle authentication path logic is mostly hidden in
/// `wots_treehashx1`.
pub unsafe fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &Addr,
    tree_addr: &mut Addr,
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1::zeroed();
    let mut steps = [0u32; SPX_WOTS_LEN];

    /* `auth_path = sig + SPX_WOTS_BYTES`; the WOTS signature is written
       through `info.wots_sig`, which points at the first SPX_WOTS_BYTES. */
    let (wots_sig, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);

    info.wots_sig = wots_sig.as_mut_ptr();
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

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

/// Compute root node of the top-most subtree.
pub unsafe fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    /* We do not need the auth path in key generation, but it simplifies the
       code to have just one treehash routine that computes both root and path
       in one function. */
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = ZERO_ADDR;
    let mut wots_addr = ZERO_ADDR;

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &wots_addr,
        &mut top_tree_addr,
        !0u32, /* ~0 means "don't bother generating an auth path" */
    );
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
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
    let wa = *addr_ref(wots_addr as *const u32);
    merkle_sign(sig_s, root_s, &*ctx, &wa, addr_mut(tree_addr), idx_leaf);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    merkle_gen_root(root_s, &*ctx);
}
