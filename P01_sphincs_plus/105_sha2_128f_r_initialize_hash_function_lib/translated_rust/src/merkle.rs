use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::wots::chain_lengths_rs;
use crate::wotsx1::LeafInfoX1;
use crate::utilsx1::wots_treehashx1_rs;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8, root: *mut u8, ctx: *const SpxCtx,
    wots_addr: *mut u32, tree_addr: *mut u32, idx_leaf: u32,
) {
    let ctx = unsafe { &*ctx };
    let wots_addr = unsafe { &mut *(wots_addr as *mut [u32; 8]) };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    merkle_sign_rs(sig, root, ctx, wots_addr, tree_addr, idx_leaf);
}

pub fn merkle_sign_rs(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32,
) {
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut steps, root);

    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    set_type_rs(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_rs(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr_rs(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr_rs(&mut info.pk_addr, wots_addr);

    wots_treehashx1_rs(
        root, &mut sig[SPX_WOTS_BYTES..], ctx,
        idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let ctx = unsafe { &*ctx };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    merkle_gen_root_rs(root, ctx);
}

pub fn merkle_gen_root_rs(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr_rs(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr_rs(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign_rs(
        &mut auth_path, root, ctx,
        &mut wots_addr, &mut top_tree_addr, !0u32,
    );
}
