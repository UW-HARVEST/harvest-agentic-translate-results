//! Translated from crypto_core/keccak1600/ref/keccak1600_ref.c and keccak1600.c
use crate::primitives::cutil::*;

const KECCAK1600_STATEBYTES: usize = 200;

static RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

#[inline(always)]
fn rl(x: u64, b: u32) -> u64 {
    rotl64(x, b)
}

// Generic keccak-f over the 25-lane state, running `nrounds` rounds ending at round 23,
// i.e. rounds [24 - nrounds .. 24). Matches the unrolled C for both 24 and 12 rounds.
fn keccakf(st: &mut [u64; 25], first_round: usize) {
    let mut a = *st;
    let mut e = [0u64; 25];
    // indices: ba=0,be=1,bi=2,bo=3,bu=4, ga=5..gu=9, ka=10..ku=14, ma=15..mu=19, sa=20..su=24
    let mut round = first_round;
    while round < 24 {
        // one round: theta+rho+pi+chi+iota, reading from `a`, writing to `e`
        if (round - first_round) % 2 == 0 {
            keccak_round(&a, &mut e, round);
        } else {
            keccak_round(&e, &mut a, round);
        }
        round += 1;
    }
    // After the loop, results are in `a` if total rounds is even, else in `e`.
    let total = 24 - first_round;
    if total % 2 == 0 {
        *st = a;
    } else {
        *st = e;
    }
}

#[inline(always)]
fn keccak_round(a: &[u64; 25], e: &mut [u64; 25], round: usize) {
    // Column parity
    let ca = a[0] ^ a[5] ^ a[10] ^ a[15] ^ a[20];
    let ce = a[1] ^ a[6] ^ a[11] ^ a[16] ^ a[21];
    let ci = a[2] ^ a[7] ^ a[12] ^ a[17] ^ a[22];
    let co = a[3] ^ a[8] ^ a[13] ^ a[18] ^ a[23];
    let cu = a[4] ^ a[9] ^ a[14] ^ a[19] ^ a[24];

    let da = cu ^ rl(ce, 1);
    let de = ca ^ rl(ci, 1);
    let di = ce ^ rl(co, 1);
    let dofs = ci ^ rl(cu, 1);
    let du = co ^ rl(ca, 1);

    // group b (E[0..5])
    let bba = a[0] ^ da;
    let bbe = rl(a[6] ^ de, 44);
    let bbi = rl(a[12] ^ di, 43);
    let bbo = rl(a[18] ^ dofs, 21);
    let bbu = rl(a[24] ^ du, 14);
    e[0] = (bba ^ ((!bbe) & bbi)) ^ RC[round];
    e[1] = bbe ^ ((!bbi) & bbo);
    e[2] = bbi ^ ((!bbo) & bbu);
    e[3] = bbo ^ ((!bbu) & bba);
    e[4] = bbu ^ ((!bba) & bbe);

    // group g (E[5..10])
    let bga = rl(a[3] ^ dofs, 28);
    let bge = rl(a[9] ^ du, 20);
    let bgi = rl(a[10] ^ da, 3);
    let bgo = rl(a[16] ^ de, 45);
    let bgu = rl(a[22] ^ di, 61);
    e[5] = bga ^ ((!bge) & bgi);
    e[6] = bge ^ ((!bgi) & bgo);
    e[7] = bgi ^ ((!bgo) & bgu);
    e[8] = bgo ^ ((!bgu) & bga);
    e[9] = bgu ^ ((!bga) & bge);

    // group k (E[10..15])
    let bka = rl(a[1] ^ de, 1);
    let bke = rl(a[7] ^ di, 6);
    let bki = rl(a[13] ^ dofs, 25);
    let bko = rl(a[19] ^ du, 8);
    let bku = rl(a[20] ^ da, 18);
    e[10] = bka ^ ((!bke) & bki);
    e[11] = bke ^ ((!bki) & bko);
    e[12] = bki ^ ((!bko) & bku);
    e[13] = bko ^ ((!bku) & bka);
    e[14] = bku ^ ((!bka) & bke);

    // group m (E[15..20])
    let bma = rl(a[4] ^ du, 27);
    let bme = rl(a[5] ^ da, 36);
    let bmi = rl(a[11] ^ de, 10);
    let bmo = rl(a[17] ^ di, 15);
    let bmu = rl(a[23] ^ dofs, 56);
    e[15] = bma ^ ((!bme) & bmi);
    e[16] = bme ^ ((!bmi) & bmo);
    e[17] = bmi ^ ((!bmo) & bmu);
    e[18] = bmo ^ ((!bmu) & bma);
    e[19] = bmu ^ ((!bma) & bme);

    // group s (E[20..25])
    let bsa = rl(a[2] ^ di, 62);
    let bse = rl(a[8] ^ dofs, 55);
    let bsi = rl(a[14] ^ du, 39);
    let bso = rl(a[15] ^ da, 41);
    let bsu = rl(a[21] ^ de, 2);
    e[20] = bsa ^ ((!bse) & bsi);
    e[21] = bse ^ ((!bsi) & bso);
    e[22] = bsi ^ ((!bso) & bsu);
    e[23] = bso ^ ((!bsu) & bsa);
    e[24] = bsu ^ ((!bsa) & bse);
}

unsafe fn load_state(state: *const u8, st: &mut [u64; 25]) {
    for i in 0..25 {
        st[i] = load64_le(state.add(i * 8));
    }
}
unsafe fn store_state(state: *mut u8, st: &[u64; 25]) {
    for i in 0..25 {
        store64_le(state.add(i * 8), st[i]);
    }
}

// ---- _sodium_keccak1600_ref_* exported symbols ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_24(state: *mut core::ffi::c_void) {
    let mut st = [0u64; 25];
    load_state(state as *const u8, &mut st);
    keccakf(&mut st, 0);
    store_state(state as *mut u8, &st);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_12(state: *mut core::ffi::c_void) {
    let mut st = [0u64; 25];
    load_state(state as *const u8, &mut st);
    keccakf(&mut st, 12);
    store_state(state as *mut u8, &st);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_init(state: *mut core::ffi::c_void) {
    core::ptr::write_bytes(state as *mut u8, 0, KECCAK1600_STATEBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_xor_bytes(
    state: *mut core::ffi::c_void,
    mut data: *const u8,
    mut offset: usize,
    mut length: usize,
) {
    let st = state as *mut u8;
    while length > 0 && (offset & 7) != 0 {
        *st.add(offset) ^= *data;
        data = data.add(1);
        offset += 1;
        length -= 1;
    }
    while length >= 8 {
        let t = load64_le(st.add(offset)) ^ load64_le(data);
        store64_le(st.add(offset), t);
        data = data.add(8);
        offset += 8;
        length -= 8;
    }
    while length > 0 {
        *st.add(offset) ^= *data;
        data = data.add(1);
        offset += 1;
        length -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_extract_bytes(
    state: *const core::ffi::c_void,
    data: *mut u8,
    offset: usize,
    length: usize,
) {
    let st = state as *const u8;
    core::ptr::copy_nonoverlapping(st.add(offset), data, length);
}

// ---- crypto_core_keccak1600 wrappers ----
// Only statebytes and permute_24 are exported (in P1). The rest are private helpers
// used by the SHA-3 module.

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_keccak1600_statebytes() -> usize {
    // sizeof(crypto_core_keccak1600_state) == 224 (opaque[224])
    224
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_24(state: *mut core::ffi::c_void) {
    _sodium_keccak1600_ref_permute_24(state);
}

// state->opaque is at offset 0, so the state pointer equals &state->opaque.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_init(state: *mut core::ffi::c_void) {
    _sodium_keccak1600_ref_init(state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_xor_bytes(
    state: *mut core::ffi::c_void,
    bytes: *const u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_xor_bytes(state, bytes, offset, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_extract_bytes(
    state: *const core::ffi::c_void,
    bytes: *mut u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_extract_bytes(state, bytes, offset, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_12(state: *mut core::ffi::c_void) {
    _sodium_keccak1600_ref_permute_12(state);
}

// private helpers (state pointer == &state.opaque, offset 0)
pub(crate) unsafe fn core_init(state: *mut u8) {
    _sodium_keccak1600_ref_init(state as *mut core::ffi::c_void);
}
pub(crate) unsafe fn core_xor_bytes(state: *mut u8, data: *const u8, offset: usize, length: usize) {
    _sodium_keccak1600_ref_xor_bytes(state as *mut core::ffi::c_void, data, offset, length);
}
pub(crate) unsafe fn core_extract_bytes(state: *const u8, data: *mut u8, offset: usize, length: usize) {
    _sodium_keccak1600_ref_extract_bytes(state as *const core::ffi::c_void, data, offset, length);
}
pub(crate) unsafe fn core_permute_24(state: *mut u8) {
    _sodium_keccak1600_ref_permute_24(state as *mut core::ffi::c_void);
}
