//! Translation of `app/src/utilsx1.c` / `app/include/utilsx1.h`.

use crate::address::{set_tree_height, set_tree_index};
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::SPX_N;
use crate::wotsx1::{wots_gen_leafx1, CLeafInfoX1, LeafInfoX1};

/// Generate the entire Merkle tree, computing the authentication path for
/// leaf_idx, and the resulting root node using Merkle's TreeHash algorithm.
/// Expects the layer and tree parts of the tree_addr to be set, as well as the
/// tree type (i.e. SPX_ADDR_TYPE_HASHTREE or SPX_ADDR_TYPE_FORSTREE)
///
/// This expects tree_addr to be initialized to the addr structures for the
/// Merkle tree nodes
///
/// Applies the offset idx_offset to indices before building addresses, so that
/// it is possible to continue counting indices across trees.
///
/// This works by using the standard Merkle tree building algorithm.
///
/// The C file duplicates this body once for `wots_gen_leafx1` and once for
/// `fors_gen_leafx1`; here it is shared via a generic leaf generator.
fn treehashx1<I, F>(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut I,
    mut gen_leafx1: F,
) where
    F: FnMut(&mut [u8], &SpxCtx, u32, &mut I),
{
    /* This is where we keep the intermediate nodes */
    // SPX_VLA(uint8_t, stack, tree_height*SPX_N);
    let mut stack: Vec<u8> = vec![0u8; (tree_height as usize) * SPX_N];

    let max_idx: u32 = 1u32.wrapping_shl(tree_height).wrapping_sub(1);
    let mut idx: u32 = 0;
    loop {
        /* Current logical node is at index[SPX_N].  We do this to minimize the
           number of copies needed during a thash */
        let mut current = [0u8; 2 * SPX_N];
        gen_leafx1(
            &mut current[SPX_N..],
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        /* Now combine the freshly generated right node with previously */
        /* generated left ones */
        let mut internal_idx_offset: u32 = idx_offset;
        let mut internal_idx: u32 = idx;
        let mut internal_leaf: u32 = leaf_idx;
        let mut h: u32 = 0; /* The height we are in the Merkle tree */
        loop {
            /* Check if we hit the top of the tree */
            if h == tree_height {
                /* We hit the root; return it */
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            /*
             * Check if the node we have is a part of the
             * authentication path; if it is, write it out
             */
            if (internal_idx ^ internal_leaf) == 0x01 {
                let dst = h as usize * SPX_N;
                auth_path[dst..dst + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
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
            set_tree_height(tree_addr, h.wrapping_add(1));
            set_tree_index(
                tree_addr,
                (internal_idx / 2).wrapping_add(internal_idx_offset),
            );

            let left = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left..left + SPX_N]);
            // thash(&current[SPX_N], &current[0], 2, ...) -- out and in overlap.
            let tmp = current;
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h = h.wrapping_add(1);
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        /* We've hit a left child; save the current for when we get the */
        /* corresponding right right */
        let dst = h as usize * SPX_N;
        stack[dst..dst + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx = idx.wrapping_add(1);
    }
}

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
    treehashx1(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr,
        info,
        |leaf, c, addr_idx, i| wots_gen_leafx1(leaf, c, addr_idx, i),
    );
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
    treehashx1(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr,
        info,
        |leaf, c, addr_idx, i| fors_gen_leafx1(leaf, c, addr_idx, i),
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

/// ABI mirror of the C `struct fors_gen_leaf_info`.  Note that `utilsx1.h`
/// declares `fors_treehashx1`'s last parameter as `leaf_info_x1 *`, but the
/// implementation hands it straight to `fors_gen_leafx1`, which treats it as a
/// `fors_gen_leaf_info *`.  Both are pointers, so the ABI is unaffected.
#[repr(C)]
pub struct CForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut CLeafInfoX1,
) {
    unsafe {
        let root = core::slice::from_raw_parts_mut(root, SPX_N);
        let auth_path =
            core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N);
        let ctx_ref = &*ctx;
        let tree_addr = &mut *(tree_addr as *mut [u32; 8]);
        crate::wotsx1::with_leaf_info(info, |i| {
            wots_treehashx1(
                root,
                auth_path,
                ctx_ref,
                leaf_idx,
                idx_offset,
                tree_height,
                tree_addr,
                i,
            );
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut CForsGenLeafInfo,
) {
    unsafe {
        let ci = &mut *info;
        let mut rust_info = ForsGenLeafInfo {
            leaf_addrx: ci.leaf_addrx,
        };
        fors_treehashx1(
            core::slice::from_raw_parts_mut(root, SPX_N),
            core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N),
            &*ctx,
            leaf_idx,
            idx_offset,
            tree_height,
            &mut *(tree_addr as *mut [u32; 8]),
            &mut rust_info,
        );
        ci.leaf_addrx = rust_info.leaf_addrx;
    }
}
