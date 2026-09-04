//! Translated from:
//!   - `c_src/libsodium/crypto_pwhash/argon2/argon2.c`
//!   - `c_src/libsodium/crypto_pwhash/argon2/argon2-core.c`
//!   - `c_src/libsodium/crypto_pwhash/argon2/argon2-fill-block-ref.c`
//!
//! Headers translated inline (types/consts shared via pointers with other
//! modules): `argon2.h`, `argon2-core.h`, `blake2b-long.h`,
//! `blamka-round-ref.h`.
//!
//! The reference build has no `HAVE_PTHREAD` / SIMD support, so only the
//! single-threaded `fill_segment_ref` implementation exists and
//! `argon2_pick_best_implementation()` always selects it.

#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

use crate::common::{rotr64, store32_le};
use crate::csys::{free, malloc, memcpy, strlen, set_errno, ENOMEM};
use crate::types::crypto_generichash_blake2b_state;

// ===========================================================================
// Header: crypto_pwhash/argon2/argon2.h
// ===========================================================================

pub const ARGON2_OK: c_int = 0;
pub const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
pub const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
pub const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
pub const ARGON2_PWD_TOO_SHORT: c_int = -4;
pub const ARGON2_PWD_TOO_LONG: c_int = -5;
pub const ARGON2_SALT_TOO_SHORT: c_int = -6;
pub const ARGON2_SALT_TOO_LONG: c_int = -7;
pub const ARGON2_AD_TOO_SHORT: c_int = -8;
pub const ARGON2_AD_TOO_LONG: c_int = -9;
pub const ARGON2_SECRET_TOO_SHORT: c_int = -10;
pub const ARGON2_SECRET_TOO_LONG: c_int = -11;
pub const ARGON2_TIME_TOO_SMALL: c_int = -12;
pub const ARGON2_TIME_TOO_LARGE: c_int = -13;
pub const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
pub const ARGON2_MEMORY_TOO_MUCH: c_int = -15;
pub const ARGON2_LANES_TOO_FEW: c_int = -16;
pub const ARGON2_LANES_TOO_MANY: c_int = -17;
pub const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
pub const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
pub const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
pub const ARGON2_AD_PTR_MISMATCH: c_int = -21;
pub const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
pub const ARGON2_FREE_MEMORY_CBK_NULL: c_int = -23;
pub const ARGON2_ALLOCATE_MEMORY_CBK_NULL: c_int = -24;
pub const ARGON2_INCORRECT_PARAMETER: c_int = -25;
pub const ARGON2_INCORRECT_TYPE: c_int = -26;
pub const ARGON2_OUT_PTR_MISMATCH: c_int = -27;
pub const ARGON2_THREADS_TOO_FEW: c_int = -28;
pub const ARGON2_THREADS_TOO_MANY: c_int = -29;
pub const ARGON2_MISSING_ARGS: c_int = -30;
pub const ARGON2_ENCODING_FAIL: c_int = -31;
pub const ARGON2_DECODING_FAIL: c_int = -32;
pub const ARGON2_THREAD_FAIL: c_int = -33;
pub const ARGON2_DECODING_LENGTH_FAIL: c_int = -34;
pub const ARGON2_VERIFY_MISMATCH: c_int = -35;

/// `argon2_context` (`Argon2_Context`) — must match `argon2.h` field-for-field
/// (other modules pass pointers to it; layout is `#[repr(C)]`-identical to
/// the copy defined in `argon2_encoding.rs`).
#[repr(C)]
pub struct argon2_context {
    pub out: *mut u8,
    pub outlen: u32,

    pub pwd: *mut u8,
    pub pwdlen: u32,

    pub salt: *mut u8,
    pub saltlen: u32,

    pub secret: *mut u8,
    pub secretlen: u32,

    pub ad: *mut u8,
    pub adlen: u32,

    pub t_cost: u32,
    pub m_cost: u32,
    pub lanes: u32,
    pub threads: u32,

    pub flags: u32,
}

/// `ARGON2_FLAG_CLEAR_PASSWORD` / `ARGON2_FLAG_CLEAR_SECRET` / `ARGON2_DEFAULT_FLAGS`.
pub const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1 << 0;
pub const ARGON2_FLAG_CLEAR_SECRET: u32 = 1 << 1;
pub const ARGON2_DEFAULT_FLAGS: u32 = 0;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum argon2_type {
    Argon2_i = 1,
    Argon2_id = 2,
}

// ===========================================================================
// Header: crypto_pwhash/argon2/argon2-core.h
// ===========================================================================

pub const ARGON2_VERSION_NUMBER: u32 = 0x13;

pub const ARGON2_BLOCK_SIZE: usize = 1024;
pub const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8; // 128
pub const ARGON2_ADDRESSES_IN_BLOCK: u32 = 128;

pub const ARGON2_PREHASH_DIGEST_LENGTH: usize = 64;
pub const ARGON2_PREHASH_SEED_LENGTH: usize = 72;

pub const ARGON2_SYNC_POINTS: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct block {
    pub v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

#[repr(C)]
pub struct block_region {
    pub base: *mut c_void,
    pub memory: *mut block,
    pub size: usize,
}

#[repr(C)]
pub struct argon2_instance_t {
    pub region: *mut block_region,
    pub pseudo_rands: *mut u64,
    pub passes: u32,
    pub current_pass: u32,
    pub memory_blocks: u32,
    pub segment_length: u32,
    pub lane_length: u32,
    pub lanes: u32,
    pub threads: u32,
    pub type_: argon2_type,
    pub print_internals: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct argon2_position_t {
    pub pass: u32,
    pub lane: u32,
    pub slice: u8,
    pub index: u32,
}

#[repr(C)]
pub struct argon2_thread_data {
    pub instance_ptr: *mut argon2_instance_t,
    pub pos: argon2_position_t,
}

type fill_segment_fn = unsafe extern "C" fn(*const argon2_instance_t, argon2_position_t);

// ---------------------------------------------------------------------
// Cross-module calls
// ---------------------------------------------------------------------

extern "C" {
    #[link_name = "_sodium_blake2b_long"]
    fn blake2b_long(pout: *mut c_void, outlen: usize, in_: *const c_void, inlen: usize) -> c_int;

    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_blake2b_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);

    #[link_name = "_sodium_argon2_encode_string"]
    fn argon2_encode_string(
        dst: *mut c_char,
        dst_len: usize,
        ctx: *mut argon2_context,
        type_: argon2_type,
    ) -> c_int;
    #[link_name = "_sodium_argon2_decode_string"]
    fn argon2_decode_string(ctx: *mut argon2_context, str_: *const c_char, type_: argon2_type) -> c_int;
}

// ---------------------------------------------------------------------
// block helpers: init_block_value / copy_block / xor_block
// ---------------------------------------------------------------------

#[inline]
unsafe fn init_block_value(b: *mut block, val: u8) {
    // memset(b->v, in, sizeof(b->v)) — target is little-endian, so filling
    // every byte with `val` is the same as filling every u64 word with the
    // 8-times-repeated byte pattern.
    let word = u64::from_le_bytes([val; 8]);
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        (*b).v[i] = word;
    }
}

#[inline]
unsafe fn copy_block(dst: *mut block, src: *const block) {
    (*dst).v = (*src).v;
}

#[inline]
unsafe fn xor_block(dst: *mut block, src: *const block) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] ^= (*src).v[i];
    }
}

fn zero_block() -> block {
    block { v: [0u64; ARGON2_QWORDS_IN_BLOCK] }
}

// ---------------------------------------------------------------------
// index_alpha (`static uint32_t index_alpha(...)` in argon2-core.h)
// ---------------------------------------------------------------------

unsafe fn index_alpha(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rand: u32,
    same_lane: bool,
) -> u32 {
    let segment_length = (*instance).segment_length;
    let lane_length = (*instance).lane_length;
    let pass = (*position).pass;
    let slice = (*position).slice as u32;
    let index = (*position).index;

    let reference_area_size: u32;
    if pass == 0 {
        if slice == 0 {
            reference_area_size = index.wrapping_sub(1);
        } else if same_lane {
            reference_area_size = slice
                .wrapping_mul(segment_length)
                .wrapping_add(index)
                .wrapping_sub(1);
        } else {
            let adj: u32 = if index == 0 { u32::MAX } else { 0 };
            reference_area_size = slice.wrapping_mul(segment_length).wrapping_add(adj);
        }
    } else if same_lane {
        reference_area_size = lane_length
            .wrapping_sub(segment_length)
            .wrapping_add(index)
            .wrapping_sub(1);
    } else {
        let adj: u32 = if index == 0 { u32::MAX } else { 0 };
        reference_area_size = lane_length.wrapping_sub(segment_length).wrapping_add(adj);
    }

    let mut relative_position: u64 = pseudo_rand as u64;
    relative_position = relative_position.wrapping_mul(relative_position) >> 32;
    let t: u32 = reference_area_size.wrapping_sub(1);
    let product_term: u64 = (reference_area_size as u64).wrapping_mul(relative_position) >> 32;
    relative_position = (t as u64).wrapping_sub(product_term);

    let mut start_position: u32 = 0;
    if pass != 0 {
        start_position = if slice == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            (slice.wrapping_add(1)).wrapping_mul(segment_length)
        };
    }

    let mut absolute_position: u64 = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub(lane_length as u64);
    absolute_position = absolute_position.wrapping_add((lane_length as u64) & (absolute_position >> 32));
    absolute_position as u32
}

// ===========================================================================
// crypto_pwhash/argon2/blamka-round-ref.h
// ===========================================================================

#[inline]
fn fblamka(x: u64, y: u64) -> u64 {
    let m: u64 = 0xFFFFFFFF;
    let xy = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(xy.wrapping_mul(2))
}

#[inline]
unsafe fn g(v: &mut [u64; ARGON2_QWORDS_IN_BLOCK], a: usize, b: usize, c: usize, d: usize) {
    v[a] = fblamka(v[a], v[b]);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = fblamka(v[c], v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = fblamka(v[a], v[b]);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = fblamka(v[c], v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

/// `BLAKE2_ROUND_NOMSG(v0..v15)`.
#[inline]
#[allow(clippy::too_many_arguments)]
unsafe fn blake2_round_nomsg(
    v: &mut [u64; ARGON2_QWORDS_IN_BLOCK],
    v0: usize,
    v1: usize,
    v2: usize,
    v3: usize,
    v4: usize,
    v5: usize,
    v6: usize,
    v7: usize,
    v8: usize,
    v9: usize,
    v10: usize,
    v11: usize,
    v12: usize,
    v13: usize,
    v14: usize,
    v15: usize,
) {
    g(v, v0, v4, v8, v12);
    g(v, v1, v5, v9, v13);
    g(v, v2, v6, v10, v14);
    g(v, v3, v7, v11, v15);
    g(v, v0, v5, v10, v15);
    g(v, v1, v6, v11, v12);
    g(v, v2, v7, v8, v13);
    g(v, v3, v4, v9, v14);
}

// ===========================================================================
// crypto_pwhash/argon2/argon2-fill-block-ref.c
// ===========================================================================

unsafe fn fill_block(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut block_r = zero_block();
    let mut block_tmp = zero_block();

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);

    for i in 0..8usize {
        blake2_round_nomsg(
            &mut block_r.v,
            16 * i,
            16 * i + 1,
            16 * i + 2,
            16 * i + 3,
            16 * i + 4,
            16 * i + 5,
            16 * i + 6,
            16 * i + 7,
            16 * i + 8,
            16 * i + 9,
            16 * i + 10,
            16 * i + 11,
            16 * i + 12,
            16 * i + 13,
            16 * i + 14,
            16 * i + 15,
        );
    }

    for i in 0..8usize {
        blake2_round_nomsg(
            &mut block_r.v,
            2 * i,
            2 * i + 1,
            2 * i + 16,
            2 * i + 17,
            2 * i + 32,
            2 * i + 33,
            2 * i + 48,
            2 * i + 49,
            2 * i + 64,
            2 * i + 65,
            2 * i + 80,
            2 * i + 81,
            2 * i + 96,
            2 * i + 97,
            2 * i + 112,
            2 * i + 113,
        );
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn fill_block_with_xor(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut block_r = zero_block();
    let mut block_tmp = zero_block();

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);
    xor_block(&mut block_tmp, next_block);

    for i in 0..8usize {
        blake2_round_nomsg(
            &mut block_r.v,
            16 * i,
            16 * i + 1,
            16 * i + 2,
            16 * i + 3,
            16 * i + 4,
            16 * i + 5,
            16 * i + 6,
            16 * i + 7,
            16 * i + 8,
            16 * i + 9,
            16 * i + 10,
            16 * i + 11,
            16 * i + 12,
            16 * i + 13,
            16 * i + 14,
            16 * i + 15,
        );
    }

    for i in 0..8usize {
        blake2_round_nomsg(
            &mut block_r.v,
            2 * i,
            2 * i + 1,
            2 * i + 16,
            2 * i + 17,
            2 * i + 32,
            2 * i + 33,
            2 * i + 48,
            2 * i + 49,
            2 * i + 64,
            2 * i + 65,
            2 * i + 80,
            2 * i + 81,
            2 * i + 96,
            2 * i + 97,
            2 * i + 112,
            2 * i + 113,
        );
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_blk = zero_block();
    let mut input_block = zero_block();
    init_block_value(&mut zero_blk, 0);
    init_block_value(&mut input_block, 0);

    if !instance.is_null() && !position.is_null() {
        input_block.v[0] = (*position).pass as u64;
        input_block.v[1] = (*position).lane as u64;
        input_block.v[2] = (*position).slice as u64;
        input_block.v[3] = (*instance).memory_blocks as u64;
        input_block.v[4] = (*instance).passes as u64;
        input_block.v[5] = (*instance).type_ as u64;

        let mut tmp_block = zero_block();
        let mut address_block = zero_block();

        let segment_length = (*instance).segment_length;
        for i in 0..segment_length {
            if i % ARGON2_ADDRESSES_IN_BLOCK == 0 {
                input_block.v[6] = input_block.v[6].wrapping_add(1);
                init_block_value(&mut tmp_block, 0);
                init_block_value(&mut address_block, 0);
                fill_block_with_xor(&zero_blk, &input_block, &mut tmp_block);
                fill_block_with_xor(&zero_blk, &tmp_block, &mut address_block);
            }

            *pseudo_rands.add(i as usize) = address_block.v[(i % ARGON2_ADDRESSES_IN_BLOCK) as usize];
        }
    }
}

/// `argon2_fill_segment_ref` -> `_sodium_argon2_fill_segment_ref`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    if instance.is_null() {
        return;
    }

    let mut data_independent_addressing = true;
    if (*instance).type_ == argon2_type::Argon2_id
        && (position.pass != 0 || position.slice as u32 >= ARGON2_SYNC_POINTS / 2)
    {
        data_independent_addressing = false;
    }

    let pseudo_rands: *mut u64 = (*instance).pseudo_rands;

    if data_independent_addressing {
        generate_addresses(instance, &position, pseudo_rands);
    }

    let mut starting_index: u32 = 0;
    if position.pass == 0 && position.slice == 0 {
        starting_index = 2;
    }

    let lane_length = (*instance).lane_length;
    let segment_length = (*instance).segment_length;

    let mut curr_offset: u32 = position
        .lane
        .wrapping_mul(lane_length)
        .wrapping_add((position.slice as u32).wrapping_mul(segment_length))
        .wrapping_add(starting_index);

    let mut prev_offset: u32 = if curr_offset % lane_length == 0 {
        curr_offset.wrapping_add(lane_length).wrapping_sub(1)
    } else {
        curr_offset.wrapping_sub(1)
    };

    let region = (*instance).region;
    let memory: *mut block = (*region).memory;

    let mut i = starting_index;
    while i < segment_length {
        if curr_offset % lane_length == 1 {
            prev_offset = curr_offset.wrapping_sub(1);
        }

        let pseudo_rand: u64 = if data_independent_addressing {
            *pseudo_rands.add(i as usize)
        } else {
            (*memory.add(prev_offset as usize)).v[0]
        };

        let mut ref_lane: u64 = (pseudo_rand >> 32) % ((*instance).lanes as u64);

        if position.pass == 0 && position.slice == 0 {
            ref_lane = position.lane as u64;
        }

        position.index = i;
        let ref_index: u64 = index_alpha(
            instance,
            &position,
            (pseudo_rand & 0xFFFFFFFF) as u32,
            ref_lane == position.lane as u64,
        ) as u64;

        let ref_block: *mut block =
            memory.add((lane_length as u64).wrapping_mul(ref_lane).wrapping_add(ref_index) as usize);
        let curr_block: *mut block = memory.add(curr_offset as usize);

        if position.pass != 0 {
            fill_block_with_xor(memory.add(prev_offset as usize), ref_block, curr_block);
        } else {
            fill_block(memory.add(prev_offset as usize), ref_block, curr_block);
        }

        i = i.wrapping_add(1);
        curr_offset = curr_offset.wrapping_add(1);
        prev_offset = prev_offset.wrapping_add(1);
    }
}

// ===========================================================================
// crypto_pwhash/argon2/argon2-core.c
// ===========================================================================

static mut FILL_SEGMENT: fill_segment_fn = _sodium_argon2_fill_segment_ref;

unsafe fn load_block(dst: *mut block, input: *const u8) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] = crate::common::load64_le(input.add(i * 8));
    }
}

unsafe fn store_block(output: *mut u8, src: *const block) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        crate::common::store64_le(output.add(i * 8), (*src).v[i]);
    }
}

unsafe fn allocate_memory(region: *mut *mut block_region, m_cost: u32) -> c_int {
    if region.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let memory_size: usize = core::mem::size_of::<block>().wrapping_mul(m_cost as usize);
    if m_cost == 0 || memory_size / (m_cost as usize) != core::mem::size_of::<block>() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    *region = malloc(core::mem::size_of::<block_region>()) as *mut block_region;
    if (*region).is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    (**region).base = core::ptr::null_mut();
    (**region).memory = core::ptr::null_mut();

    let mut memory: *mut block = core::ptr::null_mut();
    let base: *mut c_void;
    if memory_size.wrapping_add(63) < memory_size {
        base = core::ptr::null_mut();
        set_errno(ENOMEM);
    } else {
        base = malloc(memory_size.wrapping_add(63));
        if !base.is_null() {
            let mut aligned = (base as *mut u8).add(63);
            let off = (aligned as usize) & 63;
            aligned = aligned.sub(off);
            memory = aligned as *mut block;
        }
    }

    if base.is_null() {
        free(*region as *mut c_void);
        *region = core::ptr::null_mut();
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    (**region).base = base;
    (**region).memory = memory;
    (**region).size = memory_size;

    ARGON2_OK
}

unsafe fn free_memory(region: *mut block_region) {
    if !region.is_null() && !(*region).base.is_null() {
        free((*region).base);
    }
    free(region as *mut c_void);
}

unsafe fn argon2_free_instance(instance: *mut argon2_instance_t, _flags: u32) {
    free((*instance).pseudo_rands as *mut c_void);
    (*instance).pseudo_rands = core::ptr::null_mut();
    free_memory((*instance).region);
    (*instance).region = core::ptr::null_mut();
}

/// `argon2_finalize` -> `_sodium_argon2_finalize`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_finalize(context: *const argon2_context, instance: *mut argon2_instance_t) {
    if context.is_null() || instance.is_null() {
        return;
    }

    let mut blockhash = zero_block();
    let region = (*instance).region;
    let memory = (*region).memory;
    let lane_length = (*instance).lane_length;

    copy_block(&mut blockhash, memory.add((lane_length - 1) as usize));

    for l in 1..(*instance).lanes {
        let last_block_in_lane = l.wrapping_mul(lane_length).wrapping_add(lane_length.wrapping_sub(1));
        xor_block(&mut blockhash, memory.add(last_block_in_lane as usize));
    }

    let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0u8; ARGON2_BLOCK_SIZE];
    store_block(blockhash_bytes.as_mut_ptr(), &blockhash);
    blake2b_long(
        (*context).out as *mut c_void,
        (*context).outlen as usize,
        blockhash_bytes.as_ptr() as *const c_void,
        ARGON2_BLOCK_SIZE,
    );
    sodium_memzero(blockhash.v.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
    sodium_memzero(blockhash_bytes.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);

    argon2_free_instance(instance, (*context).flags);
}

/// `argon2_fill_memory_blocks` -> `_sodium_argon2_fill_memory_blocks`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_fill_memory_blocks(instance: *mut argon2_instance_t, pass: u32) {
    if instance.is_null() || (*instance).lanes == 0 {
        return;
    }

    let mut position = argon2_position_t { pass, lane: 0, slice: 0, index: 0 };
    for s in 0..ARGON2_SYNC_POINTS {
        position.slice = s as u8;
        for l in 0..(*instance).lanes {
            position.lane = l;
            position.index = 0;
            FILL_SEGMENT(instance as *const argon2_instance_t, position);
        }
    }
}

// Meaningful bounds from argon2.h / argon2-core.h (values that can never be
// exceeded because the field is already a `uint32_t` are kept for fidelity
// even though they are then always-false comparisons, matching the C code).
const ARGON2_MIN_OUTLEN: u32 = 16;
const ARGON2_MAX_OUTLEN: u32 = 0xFFFFFFFF;
const ARGON2_MAX_PWDLEN: u32 = 0xFFFFFFFF;
const ARGON2_MIN_SALT_LENGTH: u32 = 8;
const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MAX_SECRETLEN: u32 = 0xFFFFFFFF;
const ARGON2_MAX_ADLEN: u32 = 0xFFFFFFFF;
const ARGON2_MIN_LANES: u32 = 1;
const ARGON2_MAX_LANES: u32 = 0xFFFFFF;
const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS;
const ARGON2_MAX_MEMORY: u32 = 0xFFFFFFFF;
const ARGON2_MIN_TIME: u32 = 1;
const ARGON2_MAX_TIME: u32 = 0xFFFFFFFF;
const ARGON2_MIN_THREADS: u32 = 1;
const ARGON2_MAX_THREADS: u32 = 0xFFFFFF;

/// `argon2_validate_inputs` -> `_sodium_argon2_validate_inputs`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int {
    if context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }
    let ctx = &*context;

    if ctx.out.is_null() {
        return ARGON2_OUTPUT_PTR_NULL;
    }
    if ARGON2_MIN_OUTLEN > ctx.outlen {
        return ARGON2_OUTPUT_TOO_SHORT;
    }
    if ARGON2_MAX_OUTLEN < ctx.outlen {
        return ARGON2_OUTPUT_TOO_LONG;
    }

    if ctx.pwd.is_null() && ctx.pwdlen != 0 {
        return ARGON2_PWD_PTR_MISMATCH;
    }
    if ARGON2_MAX_PWDLEN < ctx.pwdlen {
        return ARGON2_PWD_TOO_LONG;
    }

    if ctx.salt.is_null() && ctx.saltlen != 0 {
        return ARGON2_SALT_PTR_MISMATCH;
    }
    if ARGON2_MIN_SALT_LENGTH > ctx.saltlen {
        return ARGON2_SALT_TOO_SHORT;
    }
    if ARGON2_MAX_SALT_LENGTH < ctx.saltlen {
        return ARGON2_SALT_TOO_LONG;
    }

    if ctx.secret.is_null() {
        if ctx.secretlen != 0 {
            return ARGON2_SECRET_PTR_MISMATCH;
        }
    } else if ARGON2_MAX_SECRETLEN < ctx.secretlen {
        return ARGON2_SECRET_TOO_LONG;
    }

    if ctx.ad.is_null() {
        if ctx.adlen != 0 {
            return ARGON2_AD_PTR_MISMATCH;
        }
    } else if ARGON2_MAX_ADLEN < ctx.adlen {
        return ARGON2_AD_TOO_LONG;
    }

    if ARGON2_MIN_LANES > ctx.lanes {
        return ARGON2_LANES_TOO_FEW;
    }
    if ARGON2_MAX_LANES < ctx.lanes {
        return ARGON2_LANES_TOO_MANY;
    }

    if ARGON2_MIN_MEMORY > ctx.m_cost {
        return ARGON2_MEMORY_TOO_LITTLE;
    }
    if ARGON2_MAX_MEMORY < ctx.m_cost {
        return ARGON2_MEMORY_TOO_MUCH;
    }
    if ctx.m_cost < 8u32.wrapping_mul(ctx.lanes) {
        return ARGON2_MEMORY_TOO_LITTLE;
    }

    if ARGON2_MIN_TIME > ctx.t_cost {
        return ARGON2_TIME_TOO_SMALL;
    }
    if ARGON2_MAX_TIME < ctx.t_cost {
        return ARGON2_TIME_TOO_LARGE;
    }

    if ARGON2_MIN_THREADS > ctx.threads {
        return ARGON2_THREADS_TOO_FEW;
    }
    if ARGON2_MAX_THREADS < ctx.threads {
        return ARGON2_THREADS_TOO_MANY;
    }

    ARGON2_OK
}

unsafe fn argon2_fill_first_blocks(blockhash: *mut u8, instance: *const argon2_instance_t) {
    let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0u8; ARGON2_BLOCK_SIZE];
    let region = (*instance).region;
    let memory = (*region).memory;
    let lane_length = (*instance).lane_length;

    for l in 0..(*instance).lanes {
        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH), 0);
        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH + 4), l);
        blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(memory.add(l.wrapping_mul(lane_length) as usize), blockhash_bytes.as_ptr());

        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH), 1);
        blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(
            memory.add(l.wrapping_mul(lane_length).wrapping_add(1) as usize),
            blockhash_bytes.as_ptr(),
        );
    }
    sodium_memzero(blockhash_bytes.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
}

unsafe fn argon2_initial_hash(blockhash: *mut u8, context: *mut argon2_context, type_: argon2_type) {
    if context.is_null() || blockhash.is_null() {
        return;
    }

    let mut blake_hash: crypto_generichash_blake2b_state = core::mem::zeroed();
    let mut value: [u8; 4] = [0; 4];

    crypto_generichash_blake2b_init(&mut blake_hash, core::ptr::null(), 0, ARGON2_PREHASH_DIGEST_LENGTH);

    store32_le(value.as_mut_ptr(), (*context).lanes);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).outlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).m_cost);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).t_cost);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), ARGON2_VERSION_NUMBER);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), type_ as u32);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).pwdlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).pwd.is_null() {
        crypto_generichash_blake2b_update(&mut blake_hash, (*context).pwd, (*context).pwdlen as u64);

        if (*context).flags & ARGON2_FLAG_CLEAR_PASSWORD != 0 {
            sodium_memzero((*context).pwd as *mut c_void, (*context).pwdlen as usize);
            (*context).pwdlen = 0;
        }
    }

    store32_le(value.as_mut_ptr(), (*context).saltlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).salt.is_null() {
        crypto_generichash_blake2b_update(&mut blake_hash, (*context).salt, (*context).saltlen as u64);
    }

    store32_le(value.as_mut_ptr(), (*context).secretlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).secret.is_null() {
        crypto_generichash_blake2b_update(&mut blake_hash, (*context).secret, (*context).secretlen as u64);

        if (*context).flags & ARGON2_FLAG_CLEAR_SECRET != 0 {
            sodium_memzero((*context).secret as *mut c_void, (*context).secretlen as usize);
            (*context).secretlen = 0;
        }
    }

    store32_le(value.as_mut_ptr(), (*context).adlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).ad.is_null() {
        crypto_generichash_blake2b_update(&mut blake_hash, (*context).ad, (*context).adlen as u64);
    }

    crypto_generichash_blake2b_final(&mut blake_hash, blockhash, ARGON2_PREHASH_DIGEST_LENGTH);
}

/// `argon2_initialize` -> `_sodium_argon2_initialize`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_initialize(
    instance: *mut argon2_instance_t,
    context: *mut argon2_context,
) -> c_int {
    let mut blockhash: [u8; ARGON2_PREHASH_SEED_LENGTH] = [0u8; ARGON2_PREHASH_SEED_LENGTH];

    if instance.is_null() || context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }

    let pr = malloc(core::mem::size_of::<u64>().wrapping_mul((*instance).segment_length as usize)) as *mut u64;
    (*instance).pseudo_rands = pr;
    if pr.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let result = allocate_memory(&mut (*instance).region, (*instance).memory_blocks);
    if result != ARGON2_OK {
        argon2_free_instance(instance, (*context).flags);
        return result;
    }

    argon2_initial_hash(blockhash.as_mut_ptr(), context, (*instance).type_);

    sodium_memzero(
        blockhash.as_mut_ptr().add(ARGON2_PREHASH_DIGEST_LENGTH) as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH - ARGON2_PREHASH_DIGEST_LENGTH,
    );

    argon2_fill_first_blocks(blockhash.as_mut_ptr(), instance);

    sodium_memzero(blockhash.as_mut_ptr() as *mut c_void, ARGON2_PREHASH_SEED_LENGTH);

    ARGON2_OK
}

unsafe fn argon2_pick_best_implementation() -> c_int {
    FILL_SEGMENT = _sodium_argon2_fill_segment_ref;
    0
}

/// `_crypto_pwhash_argon2_pick_best_implementation`.
#[no_mangle]
pub unsafe extern "C" fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int {
    argon2_pick_best_implementation()
}

// ===========================================================================
// crypto_pwhash/argon2/argon2.c
// ===========================================================================

/// `argon2_ctx` -> `_sodium_argon2_ctx`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_ctx(context: *mut argon2_context, type_: argon2_type) -> c_int {
    let mut result = _sodium_argon2_validate_inputs(context as *const argon2_context);

    if result != ARGON2_OK {
        return result;
    }

    if type_ != argon2_type::Argon2_id && type_ != argon2_type::Argon2_i {
        return ARGON2_INCORRECT_TYPE;
    }

    let mut memory_blocks = (*context).m_cost;
    let min_memory_blocks = 2u32
        .wrapping_mul(ARGON2_SYNC_POINTS)
        .wrapping_mul((*context).lanes);
    if memory_blocks < min_memory_blocks {
        memory_blocks = min_memory_blocks;
    }

    let segment_length = memory_blocks / ((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));
    memory_blocks = segment_length.wrapping_mul((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));

    let mut instance = argon2_instance_t {
        region: core::ptr::null_mut(),
        pseudo_rands: core::ptr::null_mut(),
        passes: (*context).t_cost,
        current_pass: !0u32,
        memory_blocks,
        segment_length,
        lane_length: segment_length.wrapping_mul(ARGON2_SYNC_POINTS),
        lanes: (*context).lanes,
        threads: (*context).threads,
        type_,
        print_internals: 0,
    };

    result = _sodium_argon2_initialize(&mut instance, context);
    if result != ARGON2_OK {
        return result;
    }

    for pass in 0..instance.passes {
        _sodium_argon2_fill_memory_blocks(&mut instance, pass);
    }

    _sodium_argon2_finalize(context as *const argon2_context, &mut instance);

    ARGON2_OK
}

/// `argon2_hash` -> `_sodium_argon2_hash`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_hash(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
    type_: argon2_type,
) -> c_int {
    if !hash.is_null() {
        randombytes_buf(hash, hashlen);
    }

    if pwdlen > u32::MAX as usize {
        return ARGON2_PWD_TOO_LONG;
    }
    if hashlen > u32::MAX as usize {
        return ARGON2_OUTPUT_TOO_LONG;
    }
    if saltlen > u32::MAX as usize {
        return ARGON2_SALT_TOO_LONG;
    }

    let out = malloc(hashlen) as *mut u8;
    if out.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let mut context = argon2_context {
        out,
        outlen: hashlen as u32,
        pwd: pwd as *mut u8,
        pwdlen: pwdlen as u32,
        salt: salt as *mut u8,
        saltlen: saltlen as u32,
        secret: core::ptr::null_mut(),
        secretlen: 0,
        ad: core::ptr::null_mut(),
        adlen: 0,
        t_cost,
        m_cost,
        lanes: parallelism,
        threads: parallelism,
        flags: ARGON2_DEFAULT_FLAGS,
    };

    let result = _sodium_argon2_ctx(&mut context, type_);

    if result != ARGON2_OK {
        sodium_memzero(out as *mut c_void, hashlen);
        free(out as *mut c_void);
        return result;
    }

    if !encoded.is_null() && encodedlen != 0 {
        if argon2_encode_string(encoded, encodedlen, &mut context, type_) != ARGON2_OK {
            sodium_memzero(out as *mut c_void, hashlen);
            sodium_memzero(encoded as *mut c_void, encodedlen);
            free(out as *mut c_void);
            return ARGON2_ENCODING_FAIL;
        }
    }

    if !hash.is_null() {
        memcpy(hash, out as *const c_void, hashlen);
    }

    sodium_memzero(out as *mut c_void, hashlen);
    free(out as *mut c_void);

    ARGON2_OK
}

/// `argon2i_hash_encoded` -> `_sodium_argon2i_hash_encoded`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2i_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        argon2_type::Argon2_i,
    )
}

/// `argon2i_hash_raw` -> `_sodium_argon2i_hash_raw`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2i_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        core::ptr::null_mut(),
        0,
        argon2_type::Argon2_i,
    )
}

/// `argon2id_hash_encoded` -> `_sodium_argon2id_hash_encoded`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2id_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        argon2_type::Argon2_id,
    )
}

/// `argon2id_hash_raw` -> `_sodium_argon2id_hash_raw`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2id_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        core::ptr::null_mut(),
        0,
        argon2_type::Argon2_id,
    )
}

/// `argon2_verify` -> `_sodium_argon2_verify`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: argon2_type,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();

    ctx.pwd = core::ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = core::ptr::null_mut();
    ctx.secretlen = 0;

    let encoded_len = strlen(encoded);
    if encoded_len > u32::MAX as usize {
        return ARGON2_DECODING_LENGTH_FAIL;
    }
    ctx.adlen = encoded_len as u32;
    ctx.saltlen = encoded_len as u32;
    ctx.outlen = encoded_len as u32;

    ctx.ad = malloc(ctx.adlen as usize) as *mut u8;
    ctx.salt = malloc(ctx.saltlen as usize) as *mut u8;
    ctx.out = malloc(ctx.outlen as usize) as *mut u8;

    if ctx.out.is_null() || ctx.salt.is_null() || ctx.ad.is_null() {
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let out = malloc(ctx.outlen as usize) as *mut u8;
    if out.is_null() {
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let decode_result = argon2_decode_string(&mut ctx, encoded, type_);
    if decode_result != ARGON2_OK {
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        free(out as *mut c_void);
        return decode_result;
    }

    let mut ret = _sodium_argon2_hash(
        ctx.t_cost,
        ctx.m_cost,
        ctx.threads,
        pwd,
        pwdlen,
        ctx.salt as *const c_void,
        ctx.saltlen as usize,
        out as *mut c_void,
        ctx.outlen as usize,
        core::ptr::null_mut(),
        0,
        type_,
    );

    free(ctx.ad as *mut c_void);
    free(ctx.salt as *mut c_void);

    if ret == ARGON2_OK && sodium_memcmp(out as *const c_void, ctx.out as *const c_void, ctx.outlen as usize) != 0 {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    free(out as *mut c_void);
    free(ctx.out as *mut c_void);

    ret
}

/// `argon2i_verify` -> `_sodium_argon2i_verify`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2i_verify(encoded: *const c_char, pwd: *const c_void, pwdlen: usize) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, argon2_type::Argon2_i)
}

/// `argon2id_verify` -> `_sodium_argon2id_verify`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2id_verify(encoded: *const c_char, pwd: *const c_void, pwdlen: usize) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, argon2_type::Argon2_id)
}
