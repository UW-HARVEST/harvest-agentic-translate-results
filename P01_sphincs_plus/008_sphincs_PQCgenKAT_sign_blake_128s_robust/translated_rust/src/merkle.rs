use crate::params::*;
use crate::address;
use crate::hash_blake::SpxCtx;
use crate::wots;
use crate::utilsx1;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: Vec<u32>,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES],
        wots_sign_leaf: idx_leaf,
        wots_steps: vec![0u32; SPX_WOTS_LEN],
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    wots::chain_lengths(&mut info.wots_steps, root);

    address::set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    address::set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    address::copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    address::copy_subtree_addr(&mut info.pk_addr, wots_addr);

    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N];
    let mut root_buf = [0u8; SPX_N];

    utilsx1::wots_treehashx1(
        &mut root_buf, &mut auth_path,
        ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );

    // Copy wots_sig into sig
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
    // Copy auth_path after wots sig
    sig[SPX_WOTS_BYTES..SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N]
        .copy_from_slice(&auth_path);

    root[..SPX_N].copy_from_slice(&root_buf);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    address::set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    address::set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
