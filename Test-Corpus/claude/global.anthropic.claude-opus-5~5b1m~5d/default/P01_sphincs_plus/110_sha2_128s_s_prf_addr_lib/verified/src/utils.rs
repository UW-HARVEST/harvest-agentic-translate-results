//! Translation of `app/src/utils.c` / `app/include/utils.h`.

use crate::address::{set_tree_height, set_tree_index};
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::params::SPX_N;

/// Converts the value of `input` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: u32, input: u64) {
    let mut input = input;
    /* Iterate over out in decreasing order, for big-endianness. */
    let mut i = outlen as i32 - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], input: u32) {
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

/// Converts the `inlen` bytes in `input` from big-endian byte order to an integer.
pub fn bytes_to_ull(input: &[u8], inlen: u32) -> u64 {
    let mut retval: u64 = 0;
    let mut i: u32 = 0;
    while i < inlen {
        retval |= (input[i as usize] as u64) << (8 * (inlen - 1 - i));
        i += 1;
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
/// Expects address to be complete other than the tree_height and tree_index.
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
    // Emulates the advancing `auth_path` pointer of the C code.
    let mut ap: usize = 0;

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
    while i < tree_height.wrapping_sub(1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        /* Set the address of the node we're creating. */
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        /* Pick the right or left neighbor, depending on parity of the node. */
        if leaf_idx & 1 != 0 {
            // thash(buffer + SPX_N, buffer, 2, ...) -- out overlaps in.
            let tmp = buffer;
            thash(&mut buffer[SPX_N..2 * SPX_N], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        } else {
            // thash(buffer, buffer, 2, ...) -- out overlaps in.
            let tmp = buffer;
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        }
        ap += SPX_N;
        i += 1;
    }

    /* The last iteration is exceptional; we do not copy an auth_path node. */
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    thash(&mut root[..SPX_N], &buffer, 2, ctx, addr);
}

/// Safe function-pointer type mirroring the C `gen_leaf` callback.
pub type GenLeafFn = fn(&mut [u8], &SpxCtx, u32, &[u32; 8]);

/// Core of `treehash`, generic over the leaf generator so that both the safe
/// wrapper and the C ABI wrapper can share the implementation.
fn treehash_impl<F>(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    mut gen_leaf: F,
    tree_addr: &mut [u32; 8],
) where
    F: FnMut(&mut [u8], &SpxCtx, u32, &[u32; 8]),
{
    // SPX_VLA(uint8_t, stack, (tree_height+1)*SPX_N);
    let mut stack: Vec<u8> = vec![0u8; (tree_height as usize + 1) * SPX_N];
    // SPX_VLA(unsigned int, heights, tree_height+1);
    let mut heights: Vec<u32> = vec![0u32; tree_height as usize + 1];
    let mut offset: u32 = 0;
    let mut tree_idx: u32;

    let count: u32 = 1u32.wrapping_shl(tree_height);
    let mut idx: u32 = 0;
    while idx < count {
        /* Add the next leaf node to the stack. */
        let base = offset as usize * SPX_N;
        gen_leaf(
            &mut stack[base..],
            ctx,
            idx.wrapping_add(idx_offset),
            tree_addr,
        );
        offset += 1;
        heights[offset as usize - 1] = 0;

        /* If this is a node we need for the auth path.. */
        if (leaf_idx ^ 0x1) == idx {
            let src = (offset as usize - 1) * SPX_N;
            auth_path[..SPX_N].copy_from_slice(&stack[src..src + SPX_N]);
        }

        /* While the top-most nodes are of equal height.. */
        while offset >= 2 && heights[offset as usize - 1] == heights[offset as usize - 2] {
            /* Compute index of the new node, in the next layer. */
            tree_idx = idx >> (heights[offset as usize - 1] + 1);

            /* Set the address of the node we're creating. */
            set_tree_height(tree_addr, heights[offset as usize - 1] + 1);
            set_tree_index(
                tree_addr,
                tree_idx.wrapping_add(idx_offset >> (heights[offset as usize - 1] + 1)),
            );
            /* Hash the top-most nodes from the stack together. */
            let base = (offset as usize - 2) * SPX_N;
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&stack[base..base + 2 * SPX_N]);
            thash(&mut stack[base..base + SPX_N], &tmp, 2, ctx, tree_addr);
            offset -= 1;
            /* Note that the top-most node is now one layer higher. */
            heights[offset as usize - 1] += 1;

            /* If this is a node we need for the auth path.. */
            if ((leaf_idx >> heights[offset as usize - 1]) ^ 0x1) == tree_idx {
                let dst = heights[offset as usize - 1] as usize * SPX_N;
                let src = (offset as usize - 1) * SPX_N;
                auth_path[dst..dst + SPX_N].copy_from_slice(&stack[src..src + SPX_N]);
            }
        }
        idx += 1;
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

/// For a given leaf index, computes the authentication path and the resulting
/// root node using Merkle's TreeHash algorithm.
pub fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: &mut [u32; 8],
) {
    treehash_impl(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        |leaf, c, addr_idx, ta| gen_leaf(leaf, c, addr_idx, ta),
        tree_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: core::ffi::c_uint, input: u64) {
    unsafe {
        let mut input = input;
        let mut i = outlen as i32 - 1;
        while i >= 0 {
            *out.add(i as usize) = (input & 0xff) as u8;
            input >>= 8;
            i -= 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    unsafe {
        u32_to_bytes(core::slice::from_raw_parts_mut(out, 4), input);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: core::ffi::c_uint) -> u64 {
    unsafe { bytes_to_ull(core::slice::from_raw_parts(input, inlen as usize), inlen) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        // `compute_root` reads `auth_path[0 .. SPX_N]` BEFORE the loop and one
        // more node per loop iteration, so it always touches at least SPX_N
        // bytes -- including for `tree_height == 0`, where the C loop bound
        // `tree_height - 1` underflows to 0xFFFFFFFF and the function runs away.
        // Sizing the slice as `tree_height * SPX_N` would make Rust panic on the
        // very first read instead, before mutating `addr` at all.
        let ap_len = (tree_height as usize).max(1).saturating_mul(SPX_N);
        compute_root(
            core::slice::from_raw_parts_mut(root, SPX_N),
            core::slice::from_raw_parts(leaf, SPX_N),
            leaf_idx,
            idx_offset,
            core::slice::from_raw_parts(auth_path, ap_len),
            tree_height,
            &*ctx,
            &mut *(addr as *mut [u32; 8]),
        );
    }
}

type CGenLeaf = extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: Option<CGenLeaf>,
    tree_addr: *mut u32,
) {
    unsafe {
        // The C calls through the function pointer unconditionally, so a NULL
        // `gen_leaf` faults there; `Option<fn>` uses the null-pointer
        // optimisation, so transmuting back and calling reproduces that instead
        // of turning it into a Rust panic with a different exit path.
        let f: CGenLeaf = core::mem::transmute::<Option<CGenLeaf>, CGenLeaf>(gen_leaf);
        let ctx_ptr = ctx;
        treehash_impl(
            core::slice::from_raw_parts_mut(root, SPX_N),
            // `treehash` performs no bounds checks at all: it writes
            // `auth_path[heights[offset-1] * SPX_N ..][..SPX_N]`, and
            // `heights[offset-1]` reaches `tree_height` on the final merge, so
            // the extent the C can touch is `(tree_height + 1) * SPX_N` — the
            // last node only for a `leaf_idx` outside `0 .. 2^tree_height`
            // (which the C happily accepts, overrunning the caller's buffer).
            // The Rust wrapper must not impose a bound the C does not have, or
            // it would panic where the C writes.
            core::slice::from_raw_parts_mut(auth_path, (tree_height as usize + 1) * SPX_N),
            &*ctx,
            leaf_idx,
            idx_offset,
            tree_height,
            move |leaf: &mut [u8], _c: &SpxCtx, addr_idx: u32, ta: &[u32; 8]| {
                f(leaf.as_mut_ptr(), ctx_ptr, addr_idx, ta.as_ptr())
            },
            &mut *(tree_addr as *mut [u32; 8]),
        );
    }
}
