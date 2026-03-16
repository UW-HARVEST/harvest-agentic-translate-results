use crate::params::*;
use crate::address::*;
use crate::context::SpxCtx;
use crate::wots::{chain_lengths, LeafInfoX1};
use crate::utilsx1::wots_treehashx1;

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: sig[..SPX_WOTS_BYTES].to_vec(), // will be written back
        wots_sign_leaf: idx_leaf,
        wots_steps: vec![0u32; SPX_WOTS_LEN],
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    chain_lengths(&mut info.wots_steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    // We need a mutable buffer for auth_path that starts at sig[SPX_WOTS_BYTES..]
    let mut auth_path_buf = vec![0u8; SPX_TREE_HEIGHT * SPX_N];

    wots_treehashx1(
        root, &mut auth_path_buf,
        ctx, idx_leaf, 0,
        SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );

    // Write back wots_sig and auth_path
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
    sig[auth_path_off..auth_path_off + SPX_TREE_HEIGHT * SPX_N].copy_from_slice(&auth_path_buf);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path, root, ctx,
        &mut wots_addr, &mut top_tree_addr,
        !0u32,
    );
}
