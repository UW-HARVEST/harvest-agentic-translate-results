//! Translation of `crypto_pwhash/argon2/argon2-fill-block-ref.c` together with
//! the `fBlaMka` / `G` / `BLAKE2_ROUND_NOMSG` macros of
//! `crypto_pwhash/argon2/blamka-round-ref.h`.
//!
//! This is the only fill-block implementation compiled in the reference build
//! (no AVX2/AVX512F/SSSE3/NEON/wasm32 intrinsics headers).

use core::ffi::c_int;

use crate::common::rotr64;
use crate::crypto_pwhash::argon2::argon2::*;
use crate::crypto_pwhash::argon2::argon2_core::*;

// ---------------------------------------------------------------------------
// blamka-round-ref.h
// ---------------------------------------------------------------------------

/// `static inline uint64_t fBlaMka(uint64_t x, uint64_t y)` (designed by the
/// Lyra PHC team)
#[inline(always)]
fn fBlaMka(x: u64, y: u64) -> u64 {
    const m: u64 = 0xFFFF_FFFF;
    let xy: u64 = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(2u64.wrapping_mul(xy))
}

/// ```c
/// #define BLAKE2_ROUND_NOMSG(v0, ..., v15)  \
///     do { G(v0, v4, v8, v12); G(v1, v5, v9, v13); G(v2, v6, v10, v14);
///          G(v3, v7, v11, v15); G(v0, v5, v10, v15); G(v1, v6, v11, v12);
///          G(v2, v7, v8, v13); G(v3, v4, v9, v14); } while (0)
/// ```
///
/// `idx` holds the indices of `v0 .. v15` inside the block word array `v`.
#[inline(always)]
unsafe fn BLAKE2_ROUND_NOMSG(v: *mut u64, idx: [usize; 16]) {
    macro_rules! G {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            let (ia, ib, ic, id) = (idx[$a], idx[$b], idx[$c], idx[$d]);
            let mut a: u64 = unsafe { *v.add(ia) };
            let mut b: u64 = unsafe { *v.add(ib) };
            let mut c: u64 = unsafe { *v.add(ic) };
            let mut d: u64 = unsafe { *v.add(id) };
            a = fBlaMka(a, b);
            d = rotr64(d ^ a, 32);
            c = fBlaMka(c, d);
            b = rotr64(b ^ c, 24);
            a = fBlaMka(a, b);
            d = rotr64(d ^ a, 16);
            c = fBlaMka(c, d);
            b = rotr64(b ^ c, 63);
            unsafe {
                *v.add(ia) = a;
                *v.add(ib) = b;
                *v.add(ic) = c;
                *v.add(id) = d;
            }
        }};
    }

    G!(0, 4, 8, 12);
    G!(1, 5, 9, 13);
    G!(2, 6, 10, 14);
    G!(3, 7, 11, 15);
    G!(0, 5, 10, 15);
    G!(1, 6, 11, 12);
    G!(2, 7, 8, 13);
    G!(3, 4, 9, 14);
}

/// The two `BLAKE2_ROUND_NOMSG()` index patterns of `fill_block()`.
#[inline(always)]
const fn column_idx(i: usize) -> [usize; 16] {
    let b = 16 * i;
    [
        b,
        b + 1,
        b + 2,
        b + 3,
        b + 4,
        b + 5,
        b + 6,
        b + 7,
        b + 8,
        b + 9,
        b + 10,
        b + 11,
        b + 12,
        b + 13,
        b + 14,
        b + 15,
    ]
}

#[inline(always)]
const fn row_idx(i: usize) -> [usize; 16] {
    let b = 2 * i;
    [
        b,
        b + 1,
        b + 16,
        b + 17,
        b + 32,
        b + 33,
        b + 48,
        b + 49,
        b + 64,
        b + 65,
        b + 80,
        b + 81,
        b + 96,
        b + 97,
        b + 112,
        b + 113,
    ]
}

// ---------------------------------------------------------------------------
// argon2-fill-block-ref.c
// ---------------------------------------------------------------------------

/// `static void fill_block(const block *prev_block, const block *ref_block,
///                        block *next_block)`
unsafe fn fill_block(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut blockR: block = block::new();
    let mut block_tmp: block = block::new();
    let mut i: usize;

    unsafe { copy_block(&mut blockR, ref_block) };
    unsafe { xor_block(&mut blockR, prev_block) };
    unsafe { copy_block(&mut block_tmp, &blockR) };
    /* Now blockR = ref_block + prev_block and block_tmp = ref_block + prev_block
       Apply Blake2 on columns of 64-bit words: (0,1,...,15), then
       (16,17,..31)... finally (112,113,...127) */
    i = 0;
    while i < 8 {
        unsafe { BLAKE2_ROUND_NOMSG(blockR.v.as_mut_ptr(), column_idx(i)) };
        i += 1;
    }

    /* Apply Blake2 on rows of 64-bit words: (0,1,16,17,...112,113), then
       (2,3,18,19,...,114,115).. finally (14,15,30,31,...,126,127) */
    i = 0;
    while i < 8 {
        unsafe { BLAKE2_ROUND_NOMSG(blockR.v.as_mut_ptr(), row_idx(i)) };
        i += 1;
    }

    unsafe { copy_block(next_block, &block_tmp) };
    unsafe { xor_block(next_block, &blockR) };
}

/// `static void fill_block_with_xor(const block *prev_block,
///                                 const block *ref_block, block *next_block)`
unsafe fn fill_block_with_xor(
    prev_block: *const block,
    ref_block: *const block,
    next_block: *mut block,
) {
    let mut blockR: block = block::new();
    let mut block_tmp: block = block::new();
    let mut i: usize;

    unsafe { copy_block(&mut blockR, ref_block) };
    unsafe { xor_block(&mut blockR, prev_block) };
    unsafe { copy_block(&mut block_tmp, &blockR) };
    unsafe { xor_block(&mut block_tmp, next_block) }; /* Saving the next block contents for XOR over */
    /* Now blockR = ref_block + prev_block and block_tmp = ref_block + prev_block
     * + next_block */
    /* Apply Blake2 on columns of 64-bit words: (0,1,...,15) , then
       (16,17,..31)... finally (112,113,...127) */
    i = 0;
    while i < 8 {
        unsafe { BLAKE2_ROUND_NOMSG(blockR.v.as_mut_ptr(), column_idx(i)) };
        i += 1;
    }

    /* Apply Blake2 on rows of 64-bit words: (0,1,16,17,...112,113), then
       (2,3,18,19,...,114,115).. finally (14,15,30,31,...,126,127) */
    i = 0;
    while i < 8 {
        unsafe { BLAKE2_ROUND_NOMSG(blockR.v.as_mut_ptr(), row_idx(i)) };
        i += 1;
    }

    unsafe { copy_block(next_block, &block_tmp) };
    unsafe { xor_block(next_block, &blockR) };
}

/// `static void generate_addresses(const argon2_instance_t *instance,
///                                const argon2_position_t *position,
///                                uint64_t *pseudo_rands)`
unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_block: block = block::new();
    let mut input_block: block = block::new();
    let mut address_block: block = block::new();
    let mut tmp_block: block = block::new();
    let mut i: u32;

    unsafe { init_block_value(&mut zero_block, 0) };
    unsafe { init_block_value(&mut input_block, 0) };

    if !instance.is_null() && !position.is_null() {
        input_block.v[0] = unsafe { (*position).pass } as u64;
        input_block.v[1] = unsafe { (*position).lane } as u64;
        input_block.v[2] = unsafe { (*position).slice } as u64;
        input_block.v[3] = unsafe { (*instance).memory_blocks } as u64;
        input_block.v[4] = unsafe { (*instance).passes } as u64;
        input_block.v[5] = unsafe { (*instance).type_ } as u64;

        i = 0;
        while i < unsafe { (*instance).segment_length } {
            if i % ARGON2_ADDRESSES_IN_BLOCK == 0 {
                input_block.v[6] = input_block.v[6].wrapping_add(1);
                unsafe { init_block_value(&mut tmp_block, 0) };
                unsafe { init_block_value(&mut address_block, 0) };
                unsafe { fill_block_with_xor(&zero_block, &input_block, &mut tmp_block) };
                unsafe { fill_block_with_xor(&zero_block, &tmp_block, &mut address_block) };
            }

            unsafe {
                *pseudo_rands.add(i as usize) =
                    address_block.v[(i % ARGON2_ADDRESSES_IN_BLOCK) as usize]
            };
            i = i.wrapping_add(1);
        }
    }
}

/// `void argon2_fill_segment_ref(const argon2_instance_t *instance,
///                              argon2_position_t position)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    let mut ref_block: *mut block;
    let mut curr_block: *mut block;
    /* Pseudo-random values that determine the reference block position */
    let pseudo_rands: *mut u64;
    let mut pseudo_rand: u64;
    let mut ref_index: u64;
    let mut ref_lane: u64;
    let mut prev_offset: u32;
    let mut curr_offset: u32;
    let mut starting_index: u32;
    let mut i: u32;
    let mut data_independent_addressing: c_int = 1;

    if instance.is_null() {
        return;
    }

    if unsafe { (*instance).type_ } == Argon2_id
        && (position.pass != 0 || (position.slice as u32) >= ARGON2_SYNC_POINTS / 2)
    {
        data_independent_addressing = 0;
    }

    pseudo_rands = unsafe { (*instance).pseudo_rands };

    if data_independent_addressing != 0 {
        unsafe { generate_addresses(instance, &position, pseudo_rands) };
    }

    starting_index = 0;

    if 0 == position.pass && 0 == position.slice {
        starting_index = 2; /* we have already generated the first two blocks */
    }

    /* Offset of the current block */
    curr_offset = position
        .lane
        .wrapping_mul(unsafe { (*instance).lane_length })
        .wrapping_add((position.slice as u32).wrapping_mul(unsafe { (*instance).segment_length }))
        .wrapping_add(starting_index);

    if 0 == curr_offset % unsafe { (*instance).lane_length } {
        /* Last block in this lane */
        prev_offset = curr_offset
            .wrapping_add(unsafe { (*instance).lane_length })
            .wrapping_sub(1);
    } else {
        /* Previous block */
        prev_offset = curr_offset.wrapping_sub(1);
    }

    i = starting_index;
    while i < unsafe { (*instance).segment_length } {
        /*1.1 Rotating prev_offset if needed */
        if curr_offset % unsafe { (*instance).lane_length } == 1 {
            prev_offset = curr_offset.wrapping_sub(1);
        }

        /* 1.2 Computing the index of the reference block */
        /* 1.2.1 Taking pseudo-random value from the previous block */
        if data_independent_addressing != 0 {
            pseudo_rand = unsafe { *pseudo_rands.add(i as usize) };
        } else {
            pseudo_rand =
                unsafe { (*(*(*instance).region).memory.add(prev_offset as usize)).v[0] };
        }

        /* 1.2.2 Computing the lane of the reference block */
        ref_lane = (pseudo_rand >> 32) % (unsafe { (*instance).lanes } as u64);

        if position.pass == 0 && position.slice == 0 {
            /* Can not reference other lanes yet */
            ref_lane = position.lane as u64;
        }

        /* 1.2.3 Computing the number of possible reference block within the
         * lane.
         */
        position.index = i;
        ref_index = unsafe {
            index_alpha(
                instance,
                &position,
                (pseudo_rand & 0xFFFFFFFF) as u32,
                (ref_lane == position.lane as u64) as c_int,
            )
        } as u64;

        /* 2 Creating a new block */
        ref_block = unsafe {
            (*(*instance).region).memory.add(
                ((unsafe { (*instance).lane_length } as u64).wrapping_mul(ref_lane))
                    .wrapping_add(ref_index) as usize,
            )
        };
        curr_block = unsafe { (*(*instance).region).memory.add(curr_offset as usize) };
        if position.pass != 0 {
            unsafe {
                fill_block_with_xor(
                    (*(*instance).region).memory.add(prev_offset as usize),
                    ref_block,
                    curr_block,
                )
            };
        } else {
            unsafe {
                fill_block(
                    (*(*instance).region).memory.add(prev_offset as usize),
                    ref_block,
                    curr_block,
                )
            };
        }

        i = i.wrapping_add(1);
        curr_offset = curr_offset.wrapping_add(1);
        prev_offset = prev_offset.wrapping_add(1);
    }
}
