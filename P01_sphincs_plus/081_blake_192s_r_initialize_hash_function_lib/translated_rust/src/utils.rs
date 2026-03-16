use crate::params::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 8];
    ull_to_bytes(&mut buf, 8, tree);
    bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8].copy_from_slice(&buf);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, keypair);
    bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&buf);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, tree_index);
    bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4].copy_from_slice(&buf);
}

pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &crate::context::SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap_offset = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
    }
    ap_offset += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        if leaf_idx & 1 != 0 {
            let tmp: Vec<u8> = buffer.to_vec();
            crate::thash::thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
        } else {
            let tmp: Vec<u8> = buffer.to_vec();
            crate::thash::thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
        }
        ap_offset += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    crate::thash::thash(root, &buffer, 2, ctx, addr);
}

pub fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &crate::context::SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(&mut [u8], &crate::context::SpxCtx, u32, &[u32; 8]),
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
            idx.wrapping_add(idx_offset),
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
                tree_idx.wrapping_add(idx_offset >> (heights[offset - 1] + 1)),
            );
            let start = (offset - 2) * SPX_N;
            let tmp: Vec<u8> = stack[start..start + 2 * SPX_N].to_vec();
            crate::thash::thash(&mut stack[start..start + SPX_N], &tmp, 2, ctx, tree_addr);
            offset -= 1;
            heights[offset - 1] += 1;

            let h = heights[offset - 1];
            if ((leaf_idx >> h) ^ 0x1) == tree_idx {
                let ap_start = (h as usize) * SPX_N;
                auth_path[ap_start..ap_start + SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}
