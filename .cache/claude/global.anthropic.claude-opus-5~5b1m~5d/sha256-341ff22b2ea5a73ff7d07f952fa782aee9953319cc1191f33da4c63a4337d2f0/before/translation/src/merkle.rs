//! Translation of `app/src/merkle.c` (plus `app/include/merkle.h`).

use crate::address::{
    copy_subtree_addr, set_layer_addr, set_type, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::params::{SPX_D, SPX_N, SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN};
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

/*
 * This generates a Merkle signature (WOTS signature followed by the Merkle
 * authentication path).  This is in this file because most of the complexity
 * is involved with the WOTS signature; the Merkle authentication path logic
 * is mostly hidden in treehashx4
 */
pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    /* `unsigned steps[SPX_WOTS_LEN]; chain_lengths(steps, root);`
     * Computed before `sig` is handed to the leaf info, since `root` is both
     * an input (the WOTS message) and an output (the new subtree root). */
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, &root[..SPX_N]);

    /* `unsigned char *auth_path = sig + SPX_WOTS_BYTES;` */
    let (wots_area, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);

    let mut info = LeafInfoX1::new();

    info.wots_sig = Some(wots_area);
    info.wots_steps = steps;

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, &*wots_addr);
    copy_subtree_addr(&mut info.pk_addr, &*wots_addr);

    info.wots_sign_leaf = idx_leaf;

    crate::utilsx1::wots_treehashx1(
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

/* Compute root node of the top-most subtree. */
pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    /* We do not need the auth path in key generation, but it simplifies the
       code to have just one treehash routine that computes both root and path
       in one function. */
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
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
        !0u32, /* ~0 means "don't bother generating an auth path" */
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
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
            &mut *(wots_addr as *mut [u32; 8]),
            &mut *(tree_addr as *mut [u32; 8]),
            idx_leaf,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    unsafe {
        merkle_gen_root(core::slice::from_raw_parts_mut(root, SPX_N), &*ctx);
    }
}
