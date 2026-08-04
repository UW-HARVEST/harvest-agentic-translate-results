use crate::params::*;
use crate::context::SpxCtx;
use crate::address::{set_tree_height, set_tree_index};
use crate::thash::thash;

/// Converts the value of `in_val` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut in_val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (in_val & 0xff) as u8;
        in_val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], in_val: u32) {
    out[0] = (in_val >> 24) as u8;
    out[1] = (in_val >> 16) as u8;
    out[2] = (in_val >> 8) as u8;
    out[3] = in_val as u8;
}

/// Converts the first `inlen` bytes from big-endian byte order to an integer.
pub fn bytes_to_ull(in_bytes: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (in_bytes[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = vec![0u8; 2 * SPX_N];
    let mut auth_off: usize = 0;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer.clone();
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let tmp = buffer.clone();
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

/// Merkle's TreeHash algorithm with a gen_leaf function pointer.
pub fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(&mut [u8], &SpxCtx, u32, &[u32; 8]),
    tree_addr: &mut [u32; 8],
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(
            &mut stack[offset * SPX_N..(offset + 1) * SPX_N],
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);

            set_tree_height(tree_addr, heights[offset - 1] + 1);
            set_tree_index(
                tree_addr,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );

            let base = (offset - 2) * SPX_N;
            let src = stack[base..base + 2 * SPX_N].to_vec();
            thash(&mut stack[base..base + SPX_N], &src, 2, ctx, tree_addr);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

// --- extern "C" wrappers ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, in_val: u64) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes(out, outlen as usize, in_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, in_val: u32) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes(out, in_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(in_bytes: *const u8, inlen: u32) -> u64 {
    let in_bytes = unsafe { std::slice::from_raw_parts(in_bytes, inlen as usize) };
    bytes_to_ull(in_bytes, inlen as usize)
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
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let leaf = unsafe { std::slice::from_raw_parts(leaf, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    // Wrap the C-style function pointer into our Rust signature
    let ctx_ref = unsafe { &*ctx };
    let addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let th = tree_height as usize;
    let root_s = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_s = unsafe { std::slice::from_raw_parts_mut(auth_path, th * SPX_N) };

    // We need to use the raw function pointer approach since the C callback has different types
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(
            stack[offset * SPX_N..].as_mut_ptr(),
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_s[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);

            set_tree_height(addr, heights[offset - 1] + 1);
            set_tree_index(
                addr,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );

            let base = (offset - 2) * SPX_N;
            let src = stack[base..base + 2 * SPX_N].to_vec();
            thash(&mut stack[base..base + SPX_N], &src, 2, ctx_ref, addr);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_s[h * SPX_N..(h + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root_s[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}
