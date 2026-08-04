// Merkle tree implementation

use crate::address::{
    copy_subtree_addr, set_layer_addr, set_type, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &[u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1::new();
    let mut steps = vec![0u32; SPX_WOTS_LEN];

    chain_lengths(&mut steps, root);
    info.wots_steps = steps;

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

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &wots_addr,
        &mut top_tree_addr,
        !0u32,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let s = unsafe { std::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) };
    let r = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let c = unsafe { &*ctx };
    let w = unsafe { &*(wots_addr as *const [u32; 8]) };
    let t = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    merkle_sign(s, r, c, w, t, idx_leaf);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let r = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let c = unsafe { &*ctx };
    merkle_gen_root(r, c);
}
