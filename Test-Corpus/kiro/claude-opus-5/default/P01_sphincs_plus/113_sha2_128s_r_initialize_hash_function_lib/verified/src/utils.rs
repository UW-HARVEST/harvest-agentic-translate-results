//! Translation of `app/src/utils.c` and `app/include/utils.h`.

use crate::address::{addr_mut, set_tree_height, set_tree_index, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use core::ffi::{c_uint, c_ulonglong};

/// Converts the value of `in` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut inv: u64) {
    /* Iterate over out in decreasing order, for big-endianness. */
    let mut i = outlen;
    while i > 0 {
        i -= 1;
        out[i] = (inv & 0xff) as u8;
        inv >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], inv: u32) {
    out[0] = (inv >> 24) as u8;
    out[1] = (inv >> 16) as u8;
    out[2] = (inv >> 8) as u8;
    out[3] = inv as u8;
}

/// Converts the `inlen` bytes in `in` from big-endian byte order to an integer.
pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
///
/// Expects address to be complete other than the tree_height and tree_index.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut Addr,
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap = 0usize;

    /* If leaf_idx is odd (last bit = 1), current path element is a right child
       and auth_path has to go left. Otherwise it is the other way around. */
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    }
    ap += SPX_N;

    let mut i: u32 = 0;
    while i < tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        /* Set the address of the node we're creating. */
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        /* Pick the right or left neighbor, depending on parity of the node. */
        let src = buffer;
        if leaf_idx & 1 != 0 {
            thash(&mut buffer[SPX_N..2 * SPX_N], &src, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        } else {
            thash(&mut buffer[..SPX_N], &src, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        }
        ap += SPX_N;
        i += 1;
    }

    /* The last iteration is exceptional; we do not copy an auth_path node. */
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(&mut root[..SPX_N], &buffer, 2, ctx, addr);
}

/// The tail of `hash_message()`, which is written out identically in every
/// `hash_<backend>.c`: split the `SPX_DGST_BYTES` digest into the FORS message,
/// the tree index and the leaf index.
pub fn split_digest(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    buf: &[u8; SPX_DGST_BYTES],
) {
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= u64::MAX >> ((64 - SPX_TREE_BITS) % 64);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= u32::MAX >> ((32 - SPX_LEAF_BITS) % 32);
}

/// `void (*gen_leaf)(unsigned char *, const spx_ctx *, uint32_t, const uint32_t[8])`
pub type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

/// For a given leaf index, computes the authentication path and the resulting
/// root node using Merkle's TreeHash algorithm.
///
/// Expects the layer and tree parts of the tree_addr to be set, as well as the
/// tree type (i.e. `SPX_ADDR_TYPE_HASHTREE` or `SPX_ADDR_TYPE_FORSTREE`).
/// Applies the offset `idx_offset` to indices before building addresses, so
/// that it is possible to continue counting indices across trees.
pub unsafe fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: &mut Addr,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        /* Add the next leaf node to the stack. */
        gen_leaf(
            stack.as_mut_ptr().add(offset * SPX_N),
            ctx as *const SpxCtx,
            idx.wrapping_add(idx_offset),
            tree_addr.as_ptr() as *const u32,
        );
        offset += 1;
        heights[offset - 1] = 0;

        /* If this is a node we need for the auth path.. */
        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        /* While the top-most nodes are of equal height.. */
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            /* Compute index of the new node, in the next layer. */
            let tree_idx = idx >> (heights[offset - 1] + 1);

            /* Set the address of the node we're creating. */
            set_tree_height(tree_addr, heights[offset - 1] + 1);
            set_tree_index(
                tree_addr,
                tree_idx.wrapping_add(idx_offset >> (heights[offset - 1] + 1)),
            );
            /* Hash the top-most nodes from the stack together. */
            let base = (offset - 2) * SPX_N;
            let mut src = [0u8; 2 * SPX_N];
            src.copy_from_slice(&stack[base..base + 2 * SPX_N]);
            thash(&mut stack[base..base + SPX_N], &src, 2, ctx, tree_addr);
            offset -= 1;
            /* Note that the top-most node is now one layer higher. */
            heights[offset - 1] += 1;

            /* If this is a node we need for the auth path.. */
            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let dst = heights[offset - 1] as usize * SPX_N;
                let s = (offset - 1) * SPX_N;
                auth_path[dst..dst + SPX_N].copy_from_slice(&stack[s..s + SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, inv: c_ulonglong) {
    let s = core::slice::from_raw_parts_mut(out, outlen as usize);
    ull_to_bytes(s, outlen as usize, inv as u64);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inv: u32) {
    let s = core::slice::from_raw_parts_mut(out, 4);
    u32_to_bytes(s, inv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: c_uint) -> c_ulonglong {
    let s = core::slice::from_raw_parts(inp, inlen as usize);
    bytes_to_ull(s, inlen as usize) as c_ulonglong
}

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
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let leaf_s = core::slice::from_raw_parts(leaf, SPX_N);
    let ap_s = core::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N);
    compute_root(
        root_s,
        leaf_s,
        leaf_idx,
        idx_offset,
        ap_s,
        tree_height,
        &*ctx,
        addr_mut(addr),
    );
}

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
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let ap_s = core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    treehash(
        root_s,
        ap_s,
        &*ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        gen_leaf,
        addr_mut(tree_addr),
    );
}
