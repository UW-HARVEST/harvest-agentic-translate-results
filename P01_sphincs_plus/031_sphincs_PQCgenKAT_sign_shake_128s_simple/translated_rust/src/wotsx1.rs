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

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buf);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, buf, 1, ctx, &info.leaf_addr);
            buf.copy_from_slice(&tmp);
        }
    }

    thash(&mut dest[..SPX_N], &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

/// Exact translation of the C wots_treehashx1 / fors_treehashx1 pattern.
/// `gen_leaf` is a closure that generates a leaf into `dest[..SPX_N]`.
fn treehashx1_generic<F>(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8],
    mut gen_leaf: F,
) where F: FnMut(&mut [u8], &SpxCtx, u32) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset);

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
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            // Right node: combine with left from stack
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let s_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[s_off..s_off + SPX_N]);
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &current, 2, ctx, tree_addr);
            current[SPX_N..2 * SPX_N].copy_from_slice(&tmp);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child to stack
        let s_off = h as usize * SPX_N;
        stack[s_off..s_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    // We need to pass info mutably into the closure, but also use tree_addr mutably.
    // Use raw pointer trick since the C code does the same thing.
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_generic(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        |dest, ctx, addr_idx| {
            let info = unsafe { &mut *info_ptr };
            wots_gen_leafx1(dest, ctx, addr_idx, info);
        });
}
