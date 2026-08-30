//! Translation of `app/src/utilsx1.c` and `app/include/utilsx1.h`.

use crate::address::{set_tree_height, set_tree_index};
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::fors::{ForsGenLeafInfo, fors_gen_leafx1};
use crate::params::*;
use crate::wotsx1::{LeafInfoX1, LeafInfoX1Raw, wots_gen_leafx1};

/// Generate the entire Merkle tree, computing the authentication path for
/// `leaf_idx`, and the resulting root node using Merkle's TreeHash algorithm.
pub fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
    wots_sig: &mut [u8],
) {
    /* This is where we keep the intermediate nodes */
    let mut stack_vla =
        crate::vla::Vla::<{ SPX_MAX_TREE_HEIGHT * SPX_N }>::new(tree_height as usize * SPX_N);
    let stack = stack_vla.as_mut_slice();

    let max_idx: u32 = (1u32 << tree_height) - 1;
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
            wots_sig,
        );

        /* Now combine the freshly generated right node with previously
           generated left ones */
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
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
                let hh = h as usize;
                auth_path[hh * SPX_N..(hh + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
            }

            /* Check if we're at a left child; if so, stop going up the stack.
               Exception: if we've reached the end of the tree, keep on going
               (so we combine the last 4 nodes into the one root node in two
               more iterations) */
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            /* Ok, we're at a right node.  Now combine the left and right
               logical nodes together. */

            /* Set the address of the node we're creating. */
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(
                tree_addr,
                (internal_idx / 2).wrapping_add(internal_idx_offset),
            );

            let left = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left..left + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        /* We've hit a left child; save the current for when we get the
           corresponding right one */
        let slot = h as usize * SPX_N;
        stack[slot..slot + SPX_N].copy_from_slice(&current[SPX_N..]);

        idx += 1;
    }
}

/// The FORS counterpart of [`wots_treehashx1`].
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
    /* This is where we keep the intermediate nodes */
    let mut stack_vla =
        crate::vla::Vla::<{ SPX_MAX_TREE_HEIGHT * SPX_N }>::new(tree_height as usize * SPX_N);
    let stack = stack_vla.as_mut_slice();

    let max_idx: u32 = (1u32 << tree_height) - 1;
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
                let hh = h as usize;
                auth_path[hh * SPX_N..(hh + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
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

            let left = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left..left + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let slot = h as usize * SPX_N;
        stack[slot..slot + SPX_N].copy_from_slice(&current[SPX_N..]);

        idx += 1;
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
    tree_addr: *mut u32,
    v_info: *mut LeafInfoX1Raw,
) {
    unsafe {
        let raw = &mut *v_info;
        let mut info = LeafInfoX1 {
            wots_sign_leaf: raw.wots_sign_leaf,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: raw.leaf_addr,
            pk_addr: raw.pk_addr,
        };
        if !raw.wots_steps.is_null() {
            info.wots_steps
                .copy_from_slice(core::slice::from_raw_parts(raw.wots_steps, SPX_WOTS_LEN));
        }
        let mut scratch = [0u8; SPX_WOTS_BYTES];
        let sig: &mut [u8] = if raw.wots_sig.is_null() {
            &mut scratch
        } else {
            core::slice::from_raw_parts_mut(raw.wots_sig, SPX_WOTS_BYTES)
        };
        wots_treehashx1(
            core::slice::from_raw_parts_mut(root, SPX_N),
            core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N),
            &*ctx,
            leaf_idx,
            idx_offset,
            tree_height,
            &mut *(tree_addr as *mut [u32; 8]),
            &mut info,
            sig,
        );
        raw.leaf_addr = info.leaf_addr;
        raw.pk_addr = info.pk_addr;
    }
}

/// `fors_treehashx1` is declared as taking a `leaf_info_x1 *` in
/// `app/include/utilsx1.h`, but `fors.c` actually hands it a
/// `fors_gen_leaf_info *` (the C compiler warns about it) and the pointer is
/// only ever forwarded to `fors_gen_leafx1`, which reads it as a
/// `fors_gen_leaf_info *`.  The exported signature keeps the header's type and
/// reinterprets it the way the C code does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    v_info: *mut LeafInfoX1Raw,
) {
    unsafe {
        fors_treehashx1(
            core::slice::from_raw_parts_mut(root, SPX_N),
            core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N),
            &*ctx,
            leaf_idx,
            idx_offset,
            tree_height,
            &mut *(tree_addr as *mut [u32; 8]),
            &mut *(v_info as *mut ForsGenLeafInfo),
        );
    }
}
