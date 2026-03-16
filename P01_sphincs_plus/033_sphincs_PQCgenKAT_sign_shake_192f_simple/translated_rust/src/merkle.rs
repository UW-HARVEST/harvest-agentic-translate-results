use crate::params::*;
use crate::address::*;
use crate::wots::chain_lengths;
use crate::wotsx1::wots_treehashx1;

// merkle.c

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &crate::hash::SpxCtx,
                   wots_addr: &mut Addr, tree_addr: &mut Addr, idx_leaf: u32) {
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    let mut leaf_addr = addr_zero();
    let mut pk_addr = addr_zero();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut leaf_addr, wots_addr);
    copy_subtree_addr(&mut pk_addr, wots_addr);

    // wots_treehashx1 writes both wots_sig (sig[0..SPX_WOTS_BYTES]) and
    // auth_path (sig[SPX_WOTS_BYTES..]). We need to use a temporary buffer
    // for the wots_sig part to avoid double-borrow.
    let mut wots_sig_buf = vec![0u8; SPX_WOTS_BYTES];

    wots_treehashx1(
        root,
        &mut sig[SPX_WOTS_BYTES..],
        ctx,
        idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut wots_sig_buf, idx_leaf, &steps,
        &mut leaf_addr, &mut pk_addr,
    );

    sig[..SPX_WOTS_BYTES].copy_from_slice(&wots_sig_buf);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &crate::hash::SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = addr_zero();
    let mut wots_addr = addr_zero();

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
