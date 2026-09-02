//! Translation of `app/src/utilsx1.c` and `app/include/utilsx1.h`.

use crate::address::{addr_mut, set_tree_height, set_tree_index, Addr};
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::*;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

/// Generate the entire Merkle tree, computing the authentication path for
/// `leaf_idx`, and the resulting root node using Merkle's TreeHash algorithm.
///
/// Expects the layer and tree parts of the `tree_addr` to be set, as well as
/// the tree type (i.e. `SPX_ADDR_TYPE_HASHTREE` or `SPX_ADDR_TYPE_FORSTREE`).
///
/// Applies the offset `idx_offset` to indices before building addresses, so
/// that it is possible to continue counting indices across trees.
pub unsafe fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut Addr,
    info: &mut LeafInfoX1,
) {
    /* This is where we keep the intermediate nodes */
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let max_idx: u32 = (1u32 << tree_height).wrapping_sub(1);
    let mut idx: u32 = 0;
    loop {
        /* Current logical node is at index[SPX_N].  We do this to minimize the
           number of copies needed during a thash */
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(
            &mut current[SPX_N..],
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        /* Now combine the freshly generated right node with previously
           generated left ones */
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0; /* The height we are in the Merkle tree */
        loop {
            /* Check if we hit the top of the tree */
            if h == tree_height {
                /* We hit the root; return it */
                root[..SPX_N].copy_from_slice(&current[SPX_N..]);
                return;
            }

            /* Check if the node we have is a part of the authentication path;
               if it is, write it out */
            if (internal_idx ^ internal_leaf) == 0x01 {
                let d = h as usize * SPX_N;
                auth_path[d..d + SPX_N].copy_from_slice(&current[SPX_N..]);
            }

            /* Check if we're at a left child; if so, stop going up the stack.
               Exception: if we've reached the end of the tree, keep on going
               (so we combine the last 4 nodes into the one root node in two
               more iterations) */
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            /* Ok, we're at a right node.
               Now combine the left and right logical nodes together. */

            /* Set the address of the node we're creating. */
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(
                tree_addr,
                (internal_idx / 2).wrapping_add(internal_idx_offset),
            );

            let base = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[base..base + SPX_N]);
            let src = current;
            thash(&mut current[SPX_N..], &src, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        /* We've hit a left child; save the current for when we get the
           corresponding right */
        let base = h as usize * SPX_N;
        stack[base..base + SPX_N].copy_from_slice(&current[SPX_N..]);

        idx = idx.wrapping_add(1);
    }
}

pub unsafe fn fors_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut Addr,
    info: &mut ForsGenLeafInfo,
) {
    /* This is where we keep the intermediate nodes */
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let max_idx: u32 = (1u32 << tree_height).wrapping_sub(1);
    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(
            &mut current[SPX_N..],
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let d = h as usize * SPX_N;
                auth_path[d..d + SPX_N].copy_from_slice(&current[SPX_N..]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(
                tree_addr,
                (internal_idx / 2).wrapping_add(internal_idx_offset),
            );

            let base = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[base..base + SPX_N]);
            let src = current;
            thash(&mut current[SPX_N..], &src, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let base = h as usize * SPX_N;
        stack[base..base + SPX_N].copy_from_slice(&current[SPX_N..]);

        idx = idx.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addrx4: *mut u32,
    info: *mut LeafInfoX1,
) {
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let ap_s = core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    wots_treehashx1(
        root_s,
        ap_s,
        &*ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        addr_mut(tree_addrx4),
        &mut *info,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addrx4: *mut u32,
    info: *mut LeafInfoX1,
) {
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let ap_s = core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    fors_treehashx1(
        root_s,
        ap_s,
        &*ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        addr_mut(tree_addrx4),
        &mut *(info as *mut ForsGenLeafInfo),
    );
}
