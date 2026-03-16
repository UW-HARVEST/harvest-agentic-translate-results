use crate::params::*;
use crate::address::*;
use crate::hash::*;
use crate::wots::chain_lengths;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buf = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buf, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0..SPX_WOTS_W as u32 {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&buf[..SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buf);
            thash(buf, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

/// wots_treehashx1
pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

/// merkle_sign
pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES],
        wots_sign_leaf: idx_leaf,
        wots_steps: [0u32; SPX_WOTS_LEN],
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    // wots_sig points into sig
    // We'll copy it back after treehash
    chain_lengths(&mut info.wots_steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(
        root, &mut sig[auth_path_off..],
        ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );

    // Copy wots_sig into sig
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

/// merkle_gen_root
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
