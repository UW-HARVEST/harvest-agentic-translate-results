use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::forsx1::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::SPX_N;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

pub fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    let n = SPX_N;
    let mut stack = vec![0u8; tree_height as usize * n];
    let max_idx = (1u32 << tree_height) - 1;
    let mut current = vec![0u8; 2 * n];

    let mut idx: u32 = 0;
    loop {
        wots_gen_leafx1(&mut current[n..2 * n], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..n].copy_from_slice(&current[n..2 * n]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * n..(h as usize + 1) * n]
                    .copy_from_slice(&current[n..2 * n]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..n].copy_from_slice(&stack[h as usize * n..(h as usize + 1) * n]);
            let in_copy = current.clone();
            thash(&mut current[n..2 * n], &in_copy, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[h as usize * n..(h as usize + 1) * n].copy_from_slice(&current[n..2 * n]);
        idx += 1;
    }
}

pub fn fors_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    let n = SPX_N;
    let mut stack = vec![0u8; tree_height as usize * n];
    let max_idx = (1u32 << tree_height) - 1;
    let mut current = vec![0u8; 2 * n];

    let mut idx: u32 = 0;
    loop {
        fors_gen_leafx1(&mut current[n..2 * n], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..n].copy_from_slice(&current[n..2 * n]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * n..(h as usize + 1) * n]
                    .copy_from_slice(&current[n..2 * n]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..n].copy_from_slice(&stack[h as usize * n..(h as usize + 1) * n]);
            let in_copy = current.clone();
            thash(&mut current[n..2 * n], &in_copy, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[h as usize * n..(h as usize + 1) * n].copy_from_slice(&current[n..2 * n]);
        idx += 1;
    }
}
