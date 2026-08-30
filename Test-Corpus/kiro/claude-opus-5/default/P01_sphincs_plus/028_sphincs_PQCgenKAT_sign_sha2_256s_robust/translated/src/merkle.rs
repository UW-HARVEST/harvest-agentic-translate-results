//! Translation of `app/src/merkle.c` and `app/include/merkle.h`.

use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::{LeafInfoX1, LeafInfoX1Raw};

/// This generates a Merkle signature (WOTS signature followed by the Merkle
/// authentication path).
pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &[u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1::new();

    // `info.wots_sig = sig` and `auth_path = sig + SPX_WOTS_BYTES`: the two
    // regions are disjoint halves of the same buffer.
    chain_lengths(&mut info.wots_steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    let (wots_sig, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);

    wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
        wots_sig,
    );
}

/// Compute root node of the top-most subtree.
pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    /* We do not need the auth path in key generation, but it simplifies the
       code to have just one treehash routine that computes both root and path
       in one function. */
    let mut auth_path = [0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &wots_addr,
        &mut top_tree_addr,
        /* ~0 means "don't bother generating an auth path" */ !0u32,
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
    unsafe {
        merkle_sign(
            core::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N),
            core::slice::from_raw_parts_mut(root, SPX_N),
            &*ctx,
            &*(wots_addr as *const [u32; 8]),
            &mut *(tree_addr as *mut [u32; 8]),
            idx_leaf,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    unsafe {
        merkle_gen_root(core::slice::from_raw_parts_mut(root, SPX_N), &*ctx);
    }
}

// Keep `LeafInfoX1Raw` reachable from this module for readers following the
// C header layout.
#[allow(dead_code)]
type _LeafInfoAbi = LeafInfoX1Raw;
