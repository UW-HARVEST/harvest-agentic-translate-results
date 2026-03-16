use crate::context::*;
use crate::hash::*;
use crate::params::*;
use crate::wots::*;

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut Addr, tree_addr: &mut Addr, idx_leaf: u32,
) {
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);

    // wots_treehashx1 writes wots_sig to sig[..SPX_WOTS_BYTES]
    // and auth_path to sig[SPX_WOTS_BYTES..]
    wots_treehashx1(
        root, sig, ctx,
        idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, wots_addr,
        &steps, idx_leaf,
    );
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut buf = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr: Addr = [0; SPX_ADDR_BYTES];
    let mut wots_addr: Addr = [0; SPX_ADDR_BYTES];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    set_type(&mut top_tree_addr, SPX_ADDR_TYPE_HASHTREE);

    wots_treehashx1(
        root, &mut buf, ctx,
        !0u32, 0, SPX_TREE_HEIGHT as u32,
        &mut top_tree_addr, &mut wots_addr,
        &steps, !0u32,
    );
}

/// wots_treehashx1 from utilsx1.c
/// sig_and_auth is the combined buffer: [wots_sig (SPX_WOTS_BYTES) | auth_path]
fn wots_treehashx1(
    root: &mut [u8], sig_and_auth: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut Addr, wots_addr: &mut Addr,
    wots_steps: &[u32; SPX_WOTS_LEN], wots_sign_leaf: u32,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];

        wots_gen_leaf(
            &mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset,
            wots_addr, sig_and_auth, wots_steps, wots_sign_leaf,
        );

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
                let ap_off = SPX_WOTS_BYTES + h as usize * SPX_N;
                sig_and_auth[ap_off..ap_off + SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

/// wots_gen_leafx1 from wotsx1.c
fn wots_gen_leaf(
    dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32,
    leaf_addr_template: &mut Addr,
    wots_sig: &mut [u8], wots_steps: &[u32; SPX_WOTS_LEN], wots_sign_leaf: u32,
) {
    let mut leaf_addr = *leaf_addr_template;
    let mut pk_addr = *leaf_addr_template;

    let wots_k_mask: u32 = if leaf_idx == wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut leaf_addr, leaf_idx);
    set_keypair_addr(&mut pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut leaf_addr, i as u32);
        set_hash_addr(&mut leaf_addr, 0);
        set_type(&mut leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], ctx, &leaf_addr);

        set_type(&mut leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0..SPX_WOTS_W as u32 {
            if k == wots_k {
                wots_sig[i * SPX_N..(i + 1) * SPX_N]
                    .copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }
            set_hash_addr(&mut leaf_addr, k);
            let tmp = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            thash(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], &tmp, 1, ctx, &leaf_addr);
        }
    }

    set_type(&mut pk_addr, SPX_ADDR_TYPE_WOTSPK);
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &pk_addr);
}
