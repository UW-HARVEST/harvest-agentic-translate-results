use core::ffi::c_void;

use crate::context::SpxCtx;
use crate::params::*;
use crate::wotsx1::leaf_info_x1;

/*
 * This generates a Merkle signature (WOTS signature followed by the Merkle
 * authentication path).  This is in this file because most of the complexity
 * is involved with the WOTS signature; the Merkle authentication path logic
 * is mostly hidden in treehashx4
 */
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
    let mut info: leaf_info_x1 = leaf_info_x1 {
        wots_sig: core::ptr::null_mut(),
        wots_sign_leaf: 0,
        wots_steps: core::ptr::null_mut(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    let mut steps: [core::ffi::c_uint; SPX_WOTS_LEN] = [0; SPX_WOTS_LEN];

    info.wots_sig = sig;
    crate::wots::SPX_chain_lengths(steps.as_mut_ptr(), root);
    info.wots_steps = steps.as_mut_ptr();

    crate::address::SPX_set_type(&mut *tree_addr.add(0), SPX_ADDR_TYPE_HASHTREE);
    crate::address::SPX_set_type(&mut info.pk_addr[0], SPX_ADDR_TYPE_WOTSPK);
    crate::address::SPX_copy_subtree_addr(&mut info.leaf_addr[0], wots_addr);
    crate::address::SPX_copy_subtree_addr(&mut info.pk_addr[0], wots_addr);

    info.wots_sign_leaf = idx_leaf;

    crate::utilsx1::SPX_wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT,
        tree_addr,
        &mut info as *mut leaf_info_x1 as *mut c_void,
    );
}

/* Compute root node of the top-most subtree. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    /* We do not need the auth path in key generation, but it simplifies the
       code to have just one treehash routine that computes both root and path
       in one function. */
    let mut auth_path: [u8; SPX_TREE_HEIGHT as usize * SPX_N + SPX_WOTS_BYTES] =
        [0u8; SPX_TREE_HEIGHT as usize * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr: [u32; 8] = [0u32; 8];
    let mut wots_addr: [u32; 8] = [0u32; 8];

    crate::address::SPX_set_layer_addr(top_tree_addr.as_mut_ptr(), SPX_D - 1);
    crate::address::SPX_set_layer_addr(wots_addr.as_mut_ptr(), SPX_D - 1);

    SPX_merkle_sign(
        auth_path.as_mut_ptr(),
        root,
        ctx,
        wots_addr.as_mut_ptr(),
        top_tree_addr.as_mut_ptr(),
        !0u32, /* ~0 means "don't bother generating an auth path */
    );
}
