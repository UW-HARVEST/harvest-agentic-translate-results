// Translation of c_src/app/src/merkle.c

use core::slice;

use crate::address::{
    copy_subtree_addr_inner, set_layer_addr_inner, set_type_inner,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTSPK, SPX_D, SPX_N, SPX_TREE_HEIGHT, SPX_WOTS_BYTES,
    SPX_WOTS_LEN,
};
use crate::utilsx1::wots_treehashx1_inner;
use crate::wots::chain_lengths_inner;
use crate::wotsx1::LeafInfoX1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let sig = unsafe { slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) };
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let ctx = unsafe { &*ctx };
    let wots_addr = unsafe { slice::from_raw_parts_mut(wots_addr, 8) };
    let tree_addr = unsafe { slice::from_raw_parts_mut(tree_addr, 8) };
    merkle_sign_inner(sig, root, ctx, wots_addr, tree_addr, idx_leaf);
}

pub fn merkle_sign_inner(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32],
    tree_addr: &mut [u32],
    idx_leaf: u32,
) {
    let sig_ptr = sig.as_mut_ptr();
    let mut info = LeafInfoX1 {
        wots_sig: sig_ptr,
        wots_sign_leaf: idx_leaf,
        wots_steps: core::ptr::null_mut(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    let mut steps = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_inner(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

    set_type_inner(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_inner(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr_inner(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr_inner(&mut info.pk_addr, wots_addr);

    let (wots_part, auth_path_part) = sig.split_at_mut(SPX_WOTS_BYTES);
    let _ = wots_part;
    wots_treehashx1_inner(
        root,
        auth_path_part,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT,
        tree_addr,
        &mut info,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let ctx = unsafe { &*ctx };
    merkle_gen_root_inner(root, ctx);
}

pub fn merkle_gen_root_inner(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr_inner(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr_inner(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign_inner(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
