use core::ffi::c_void;

use crate::context::SpxCtx;
use crate::params::*;

/*
 * Generate the entire Merkle tree, computing the authentication path for
 * leaf_idx, and the resulting root node using Merkle's TreeHash algorithm.
 * Expects the layer and tree parts of the tree_addr to be set, as well as the
 * tree type (i.e. SPX_ADDR_TYPE_HASHTREE or SPX_ADDR_TYPE_FORSTREE)
 *
 * This expects tree_addr to be initialized to the addr structures for the
 * Merkle tree nodes
 *
 * Applies the offset idx_offset to indices before building addresses, so that
 * it is possible to continue counting indices across trees.
 *
 * This works by using the standard Merkle tree building algorithm,
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut c_void,
) {
    /* This is where we keep the intermediate nodes */
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let mut idx: u32;
    let max_idx: u32 = (1i32.wrapping_shl(tree_height)).wrapping_sub(1) as u32;
    idx = 0;
    loop {
        /* Current logical node is at index[SPX_N].  We do this to minimize */
        /* the number of copies needed during a thash */
        let mut current: [u8; 2 * SPX_N] = [0u8; 2 * SPX_N];
        crate::wotsx1::SPX_wots_gen_leafx1(
            current.as_mut_ptr().add(SPX_N),
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        /* Now combine the freshly generated right node with previously */
        /* generated left ones */
        let mut internal_idx_offset: u32 = idx_offset;
        let mut internal_idx: u32 = idx;
        let mut internal_leaf: u32 = leaf_idx;
        let mut h: u32; /* The height we are in the Merkle tree */
        h = 0;
        loop {
            /* Check if we hit the top of the tree */
            if h == tree_height {
                /* We hit the root; return it */
                core::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                return;
            }

            /*
             * Check if the node we have is a part of the
             * authentication path; if it is, write it out
             */
            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    current.as_ptr().add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            /*
             * Check if we're at a left child; if so, stop going up the stack
             * Exception: if we've reached the end of the tree, keep on going
             * (so we combine the last 4 nodes into the one root node in two
             * more iterations)
             */
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            /* Ok, we're at a right node */
            /* Now combine the left and right logical nodes together */

            /* Set the address of the node we're creating. */
            internal_idx_offset >>= 1;
            crate::address::SPX_set_tree_height(tree_addr, h + 1);
            crate::address::SPX_set_tree_index(
                tree_addr,
                internal_idx / 2 + internal_idx_offset,
            );

            let left: *mut u8 = stack.as_mut_ptr().add(h as usize * SPX_N);
            core::ptr::copy_nonoverlapping(left, current.as_mut_ptr().add(0), SPX_N);
            crate::hash::SPX_thash(
                current.as_mut_ptr().add(1 * SPX_N),
                current.as_ptr().add(0 * SPX_N),
                2,
                ctx,
                tree_addr,
            );

            h = h.wrapping_add(1);
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        /* We've hit a left child; save the current for when we get the */
        /* corresponding right right */
        core::ptr::copy_nonoverlapping(
            current.as_ptr().add(SPX_N),
            stack.as_mut_ptr().add(h as usize * SPX_N),
            SPX_N,
        );

        idx = idx.wrapping_add(1);
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
    info: *mut c_void,
) {
    /* This is where we keep the intermediate nodes */
    let mut stack = vec![0u8; tree_height as usize * SPX_N];

    let mut idx: u32;
    let max_idx: u32 = (1i32.wrapping_shl(tree_height)).wrapping_sub(1) as u32;
    idx = 0;
    loop {
        /* Current logical node is at index[SPX_N].  We do this to minimize */
        /* the number of copies needed during a thash */
        let mut current: [u8; 2 * SPX_N] = [0u8; 2 * SPX_N];

        // IAN TODO: WARNING ON INFO PARAMETER
        crate::fors::SPX_fors_gen_leafx1(
            current.as_mut_ptr().add(SPX_N),
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        /* Now combine the freshly generated right node with previously */
        /* generated left ones */
        let mut internal_idx_offset: u32 = idx_offset;
        let mut internal_idx: u32 = idx;
        let mut internal_leaf: u32 = leaf_idx;
        let mut h: u32; /* The height we are in the Merkle tree */
        h = 0;
        loop {
            /* Check if we hit the top of the tree */
            if h == tree_height {
                /* We hit the root; return it */
                core::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                return;
            }

            /*
             * Check if the node we have is a part of the
             * authentication path; if it is, write it out
             */
            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    current.as_ptr().add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            /*
             * Check if we're at a left child; if so, stop going up the stack
             * Exception: if we've reached the end of the tree, keep on going
             * (so we combine the last 4 nodes into the one root node in two
             * more iterations)
             */
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            /* Ok, we're at a right node */
            /* Now combine the left and right logical nodes together */

            /* Set the address of the node we're creating. */
            internal_idx_offset >>= 1;
            crate::address::SPX_set_tree_height(tree_addr, h + 1);
            crate::address::SPX_set_tree_index(
                tree_addr,
                internal_idx / 2 + internal_idx_offset,
            );

            let left: *mut u8 = stack.as_mut_ptr().add(h as usize * SPX_N);
            core::ptr::copy_nonoverlapping(left, current.as_mut_ptr().add(0), SPX_N);
            crate::hash::SPX_thash(
                current.as_mut_ptr().add(1 * SPX_N),
                current.as_ptr().add(0 * SPX_N),
                2,
                ctx,
                tree_addr,
            );

            h = h.wrapping_add(1);
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        /* We've hit a left child; save the current for when we get the */
        /* corresponding right right */
        core::ptr::copy_nonoverlapping(
            current.as_ptr().add(SPX_N),
            stack.as_mut_ptr().add(h as usize * SPX_N),
            SPX_N,
        );

        idx = idx.wrapping_add(1);
    }
}
