//! Translation of `app/src/utilsx1.c`.

use crate::address::{SPX_set_tree_height, SPX_set_tree_index};
use crate::backend::SPX_thash;
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leaf_info, SPX_fors_gen_leafx1};
use crate::params::SPX_N;
use crate::wotsx1::{leaf_info_x1, SPX_wots_gen_leafx1};

/// Generate the entire Merkle tree, computing the authentication path for
/// `leaf_idx`, and the resulting root node using Merkle's TreeHash algorithm.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut leaf_info_x1,
) {
    // This is where we keep the intermediate nodes.
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let max_idx: u32 = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        // Current logical node is at index[SPX_N].
        let mut current = [0u8; 2 * SPX_N];
        let cur = current.as_mut_ptr();
        SPX_wots_gen_leafx1(cur.add(SPX_N), ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            // Check if we hit the top of the tree.
            if h == tree_height {
                core::ptr::copy_nonoverlapping(cur.add(SPX_N), root, SPX_N);
                return;
            }

            // Check if the node we have is a part of the authentication path.
            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    cur.add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            // Check if we're at a left child; if so, stop going up the stack.
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            // Combine the left and right logical nodes together.
            internal_idx_offset >>= 1;
            SPX_set_tree_height(tree_addr, h + 1);
            SPX_set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add(h as usize * SPX_N),
                cur,
                SPX_N,
            );
            SPX_thash(cur.add(SPX_N), cur, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // We've hit a left child; save the current for the right sibling.
        core::ptr::copy_nonoverlapping(cur.add(SPX_N), stack.as_mut_ptr().add(h as usize * SPX_N), SPX_N);
        idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut fors_gen_leaf_info,
) {
    // This is where we keep the intermediate nodes.
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let max_idx: u32 = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        let cur = current.as_mut_ptr();
        SPX_fors_gen_leafx1(cur.add(SPX_N), ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                core::ptr::copy_nonoverlapping(cur.add(SPX_N), root, SPX_N);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    cur.add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            SPX_set_tree_height(tree_addr, h + 1);
            SPX_set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add(h as usize * SPX_N),
                cur,
                SPX_N,
            );
            SPX_thash(cur.add(SPX_N), cur, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        core::ptr::copy_nonoverlapping(cur.add(SPX_N), stack.as_mut_ptr().add(h as usize * SPX_N), SPX_N);
        idx += 1;
    }
}
