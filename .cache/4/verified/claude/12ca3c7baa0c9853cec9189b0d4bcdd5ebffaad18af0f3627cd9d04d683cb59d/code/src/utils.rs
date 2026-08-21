//! Translation of `app/src/utils.c`.
//!
//! Exposes the namespaced symbols `SPX_ull_to_bytes`, `SPX_u32_to_bytes`,
//! `SPX_bytes_to_ull`, `SPX_compute_root` and `SPX_treehash`.

use crate::address::{SPX_set_tree_height, SPX_set_tree_index};
use crate::backend::SPX_thash;
use crate::context::SpxCtx;
use crate::params::SPX_N;
use core::ffi::c_uint;

/// Converts the value of `in` to `outlen` bytes in big-endian byte order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, mut inval: u64) {
    // Iterate over out in decreasing order, for big-endianness.
    let mut i = outlen as i64 - 1;
    while i >= 0 {
        *out.offset(i as isize) = (inval & 0xff) as u8;
        inval >>= 8;
        i -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inval: u32) {
    *out.add(0) = (inval >> 24) as u8;
    *out.add(1) = (inval >> 16) as u8;
    *out.add(2) = (inval >> 8) as u8;
    *out.add(3) = inval as u8;
}

/// Converts the `inlen` bytes in `in` from big-endian byte order to an integer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: c_uint) -> u64 {
    let mut retval: u64 = 0;
    let mut i: c_uint = 0;
    while i < inlen {
        // The C shift count `8 * (inlen - 1 - i)` is >= 64 for `inlen > 8`
        // (formally UB, in practice an x86-64 `shl` that masks the count to its
        // low 6 bits). `wrapping_shl` masks identically, so this matches the
        // compiled C for *every* `inlen`, in every Rust profile.
        retval |= ((*inp.add(i as usize)) as u64).wrapping_shl(8 * (inlen - 1 - i));
        i += 1;
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
/// Expects address to be complete other than the tree_height and tree_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    mut leaf_idx: u32,
    mut idx_offset: u32,
    mut auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let buf = buffer.as_mut_ptr();

    // If leaf_idx is odd (last bit = 1), current path element is a right child
    // and auth_path has to go left. Otherwise it is the other way around.
    if leaf_idx & 1 != 0 {
        core::ptr::copy_nonoverlapping(leaf, buf.add(SPX_N), SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buf, SPX_N);
    } else {
        core::ptr::copy_nonoverlapping(leaf, buf, SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buf.add(SPX_N), SPX_N);
    }
    auth_path = auth_path.add(SPX_N);

    let mut i: u32 = 0;
    // `tree_height - 1` is `uint32_t` arithmetic in the C, so `tree_height == 0`
    // wraps to 0xFFFFFFFF there too; `wrapping_sub` keeps that identical
    // regardless of Rust's overflow-check setting.
    while i < tree_height.wrapping_sub(1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        // Set the address of the node we're creating.
        SPX_set_tree_height(addr, i + 1);
        SPX_set_tree_index(addr, leaf_idx + idx_offset);

        // Pick the right or left neighbor, depending on parity of the node.
        if leaf_idx & 1 != 0 {
            SPX_thash(buf.add(SPX_N), buf, 2, ctx, addr);
            core::ptr::copy_nonoverlapping(auth_path, buf, SPX_N);
        } else {
            SPX_thash(buf, buf, 2, ctx, addr);
            core::ptr::copy_nonoverlapping(auth_path, buf.add(SPX_N), SPX_N);
        }
        auth_path = auth_path.add(SPX_N);
        i += 1;
    }

    // The last iteration is exceptional; we do not copy an auth_path node.
    leaf_idx >>= 1;
    idx_offset >>= 1;
    SPX_set_tree_height(addr, tree_height);
    SPX_set_tree_index(addr, leaf_idx + idx_offset);
    SPX_thash(root, buf, 2, ctx, addr);
}

/// Function pointer type for the `gen_leaf` callback used by `treehash`.
pub type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

/// For a given leaf index, computes the authentication path and the resulting
/// root node using Merkle's TreeHash algorithm.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset: usize = 0;

    let mut idx: u32 = 0;
    while idx < (1u32 << tree_height) {
        // Add the next leaf node to the stack.
        gen_leaf(
            stack.as_mut_ptr().add(offset * SPX_N),
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;

        // If this is a node we need for the auth path..
        if (leaf_idx ^ 0x1) == idx {
            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add((offset - 1) * SPX_N),
                auth_path,
                SPX_N,
            );
        }

        // While the top-most nodes are of equal height..
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            // Compute index of the new node, in the next layer.
            let tree_idx = idx >> (heights[offset - 1] + 1);

            // Set the address of the node we're creating.
            SPX_set_tree_height(tree_addr, heights[offset - 1] + 1);
            SPX_set_tree_index(tree_addr, tree_idx + (idx_offset >> (heights[offset - 1] + 1)));
            // Hash the top-most nodes from the stack together.
            let p = stack.as_mut_ptr().add((offset - 2) * SPX_N);
            SPX_thash(p, p, 2, ctx, tree_addr);
            offset -= 1;
            // Note that the top-most node is now one layer higher.
            heights[offset - 1] += 1;

            // If this is a node we need for the auth path..
            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                core::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset - 1) * SPX_N),
                    auth_path.add(heights[offset - 1] as usize * SPX_N),
                    SPX_N,
                );
            }
        }
        idx += 1;
    }
    core::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
}
