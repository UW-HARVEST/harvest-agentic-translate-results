//! Translation of `app/src/utils.c` and `app/include/utils.h`.
//!
//! In the C build this file is compiled into the core object library and, for
//! the SHA-2 and BLAKE backends, a second time into the backend library
//! (`../../app/src/utils.c` in their `CMakeLists.txt`).

use core::ffi::{c_uint, c_ulonglong};

use crate::address::{set_tree_height, set_tree_index};
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::params::*;

/// Converts the value of `inn` to `out.len()` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], inn: u64) {
    let mut inn = inn;
    /* Iterate over out in decreasing order, for big-endianness. */
    for i in (0..out.len()).rev() {
        out[i] = (inn & 0xff) as u8;
        inn >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8; 4], inn: u32) {
    out[0] = (inn >> 24) as u8;
    out[1] = (inn >> 16) as u8;
    out[2] = (inn >> 8) as u8;
    out[3] = inn as u8;
}

/// Converts the bytes in `inp` from big-endian byte order to an integer.
pub fn bytes_to_ull(inp: &[u8]) -> u64 {
    let inlen = inp.len();
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
///
/// Expects `addr` to be complete other than the tree_height and tree_index.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut leaf_idx = leaf_idx;
    let mut idx_offset = idx_offset;
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap = 0usize;

    /* If leaf_idx is odd (last bit = 1), current path element is a right child
       and auth_path has to go left. Otherwise it is the other way around. */
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    }
    ap += SPX_N;

    for i in 0..tree_height.wrapping_sub(1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        /* Set the address of the node we're creating. */
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        /* Pick the right or left neighbor, depending on parity of the node. */
        let tmp = buffer;
        if leaf_idx & 1 != 0 {
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        } else {
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        }
        ap += SPX_N;
    }

    /* The last iteration is exceptional; we do not copy an auth_path node. */
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    let tmp = buffer;
    thash(&mut root[..SPX_N], &tmp, 2, ctx, addr);
}

/// The C type of the `gen_leaf` callback taken by [`SPX_treehash`].
pub type GenLeafFn =
    unsafe extern "C" fn(leaf: *mut u8, ctx: *const SpxCtx, addr_idx: u32, tree_addr: *const u32);

/// For a given leaf index, computes the authentication path and the resulting
/// root node using Merkle's TreeHash algorithm.
///
/// Not called anywhere inside the library (the `x1` variants in `utilsx1.c`
/// replaced it) but exported, so it is translated in full.
pub unsafe fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: &mut [u32; 8],
) {
    let th = tree_height as usize;
    let mut stack_vla =
        crate::vla::Vla::<{ (SPX_MAX_TREE_HEIGHT + 1) * SPX_N }>::new((th + 1) * SPX_N);
    let stack = stack_vla.as_mut_slice();
    let mut heights_vla = crate::vla::VlaU32::<{ SPX_MAX_TREE_HEIGHT + 1 }>::new(th + 1);
    let heights = heights_vla.as_mut_slice();
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        /* Add the next leaf node to the stack. */
        unsafe {
            gen_leaf(
                stack[offset * SPX_N..].as_mut_ptr(),
                ctx,
                idx.wrapping_add(idx_offset),
                tree_addr.as_ptr(),
            );
        }
        offset += 1;
        heights[offset - 1] = 0;

        /* If this is a node we need for the auth path.. */
        if (leaf_idx ^ 0x1) == idx {
            let src: [u8; SPX_N] = stack[(offset - 1) * SPX_N..offset * SPX_N]
                .try_into()
                .unwrap();
            auth_path[..SPX_N].copy_from_slice(&src);
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
            let tmp: [u8; 2 * SPX_N] = stack[base..base + 2 * SPX_N].try_into().unwrap();
            let ctx_ref = unsafe { &*ctx };
            thash(
                &mut stack[base..base + SPX_N],
                &tmp,
                2,
                ctx_ref,
                tree_addr,
            );
            offset -= 1;
            /* Note that the top-most node is now one layer higher. */
            heights[offset - 1] += 1;

            /* If this is a node we need for the auth path.. */
            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                let src: [u8; SPX_N] = stack[(offset - 1) * SPX_N..offset * SPX_N]
                    .try_into()
                    .unwrap();
                auth_path[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&src);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, inn: c_ulonglong) {
    ull_to_bytes(
        unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) },
        inn,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inn: u32) {
    u32_to_bytes(unsafe { &mut *(out as *mut [u8; 4]) }, inn);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: c_uint) -> c_ulonglong {
    bytes_to_ull(unsafe { core::slice::from_raw_parts(inp, inlen as usize) })
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
    let auth_len = tree_height as usize * SPX_N;
    compute_root(
        unsafe { core::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { core::slice::from_raw_parts(leaf, SPX_N) },
        leaf_idx,
        idx_offset,
        unsafe { core::slice::from_raw_parts(auth_path, auth_len) },
        tree_height,
        unsafe { &*ctx },
        unsafe { &mut *(addr as *mut [u32; 8]) },
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
    unsafe {
        treehash(
            core::slice::from_raw_parts_mut(root, SPX_N),
            core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N),
            ctx,
            leaf_idx,
            idx_offset,
            tree_height,
            gen_leaf,
            &mut *(tree_addr as *mut [u32; 8]),
        );
    }
}
