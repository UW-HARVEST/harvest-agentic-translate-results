use core::ffi::{c_uint, c_ulonglong};

use crate::context::SpxCtx;
use crate::params::*;

pub type GenLeafFn =
    Option<unsafe extern "C" fn(*mut u8, *const crate::context::SpxCtx, u32, *const u32)>;

/**
 * Converts the value of 'in' to 'outlen' bytes in big-endian byte order.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, in_: c_ulonglong) {
    let mut in_ = in_;
    let mut i: i32 = outlen as i32 - 1;

    /* Iterate over out in decreasing order, for big-endianness. */
    while i >= 0 {
        *out.add(i as usize) = (in_ & 0xff) as u8;
        in_ = in_ >> 8;
        i -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, in_: u32) {
    *out.add(0) = (in_ >> 24) as u8;
    *out.add(1) = (in_ >> 16) as u8;
    *out.add(2) = (in_ >> 8) as u8;
    *out.add(3) = in_ as u8;
}

/**
 * Converts the inlen bytes in 'in' from big-endian byte order to an integer.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(in_: *const u8, inlen: c_uint) -> c_ulonglong {
    let mut retval: c_ulonglong = 0;
    let mut i: c_uint = 0;

    while i < inlen {
        retval |= (*in_.add(i as usize) as c_ulonglong) << (8 * (inlen - 1 - i));
        i += 1;
    }
    retval
}

/**
 * Computes a root node given a leaf and an auth path.
 * Expects address to be complete other than the tree_height and tree_index.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut leaf_idx = leaf_idx;
    let mut idx_offset = idx_offset;
    let mut auth_path = auth_path;
    let mut buffer = [0u8; 2 * SPX_N];

    /* If leaf_idx is odd (last bit = 1), current path element is a right child
       and auth_path has to go left. Otherwise it is the other way around. */
    if leaf_idx & 1 != 0 {
        core::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr().add(SPX_N), SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr(), SPX_N);
    } else {
        core::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr(), SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr().add(SPX_N), SPX_N);
    }
    auth_path = auth_path.add(SPX_N);

    let mut i: u32 = 0;
    while i < tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        /* Set the address of the node we're creating. */
        crate::address::SPX_set_tree_height(addr, i + 1);
        crate::address::SPX_set_tree_index(addr, leaf_idx + idx_offset);

        /* Pick the right or left neighbor, depending on parity of the node. */
        if leaf_idx & 1 != 0 {
            crate::hash::SPX_thash(
                buffer.as_mut_ptr().add(SPX_N),
                buffer.as_ptr(),
                2,
                ctx,
                addr,
            );
            core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr(), SPX_N);
        } else {
            crate::hash::SPX_thash(buffer.as_mut_ptr(), buffer.as_ptr(), 2, ctx, addr);
            core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr().add(SPX_N), SPX_N);
        }
        auth_path = auth_path.add(SPX_N);
        i += 1;
    }

    /* The last iteration is exceptional; we do not copy an auth_path node. */
    leaf_idx >>= 1;
    idx_offset >>= 1;
    crate::address::SPX_set_tree_height(addr, tree_height);
    crate::address::SPX_set_tree_index(addr, leaf_idx + idx_offset);
    crate::hash::SPX_thash(root, buffer.as_ptr(), 2, ctx, addr);
}

/**
 * For a given leaf index, computes the authentication path and the resulting
 * root node using Merkle's TreeHash algorithm.
 * Expects the layer and tree parts of the tree_addr to be set, as well as the
 * tree type (i.e. SPX_ADDR_TYPE_HASHTREE or SPX_ADDR_TYPE_FORSTREE).
 * Applies the offset idx_offset to indices before building addresses, so that
 * it is possible to continue counting indices across trees.
 */
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
    let mut stack = vec![0u8; ((tree_height + 1) as usize) * SPX_N];
    let mut heights = vec![0u32; (tree_height + 1) as usize];
    let mut offset: c_uint = 0;
    let mut idx: u32;
    let mut tree_idx: u32;

    let gen_leaf = gen_leaf.unwrap();

    idx = 0;
    while idx < (1u32 << tree_height) {
        /* Add the next leaf node to the stack. */
        gen_leaf(
            stack.as_mut_ptr().add(offset as usize * SPX_N),
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[(offset - 1) as usize] = 0;

        /* If this is a node we need for the auth path.. */
        if (leaf_idx ^ 0x1) == idx {
            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add((offset - 1) as usize * SPX_N),
                auth_path,
                SPX_N,
            );
        }

        /* While the top-most nodes are of equal height.. */
        while offset >= 2 && heights[(offset - 1) as usize] == heights[(offset - 2) as usize] {
            /* Compute index of the new node, in the next layer. */
            tree_idx = idx >> (heights[(offset - 1) as usize] + 1);

            /* Set the address of the node we're creating. */
            crate::address::SPX_set_tree_height(tree_addr, heights[(offset - 1) as usize] + 1);
            crate::address::SPX_set_tree_index(
                tree_addr,
                tree_idx + (idx_offset >> (heights[(offset - 1) as usize] + 1)),
            );
            /* Hash the top-most nodes from the stack together. */
            crate::hash::SPX_thash(
                stack.as_mut_ptr().add((offset - 2) as usize * SPX_N),
                stack.as_ptr().add((offset - 2) as usize * SPX_N),
                2,
                ctx,
                tree_addr,
            );
            offset -= 1;
            /* Note that the top-most node is now one layer higher. */
            heights[(offset - 1) as usize] += 1;

            /* If this is a node we need for the auth path.. */
            if ((leaf_idx >> heights[(offset - 1) as usize]) ^ 0x1) == tree_idx {
                core::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset - 1) as usize * SPX_N),
                    auth_path.add(heights[(offset - 1) as usize] as usize * SPX_N),
                    SPX_N,
                );
            }
        }
        idx += 1;
    }
    core::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
}
