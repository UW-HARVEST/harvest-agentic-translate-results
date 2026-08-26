//! Translation of `crypto_core/keccak1600/ref/keccak1600_ref.c`.
//!
//! Portable (`ref`) Keccak-f[1600] permutation.  `HAVE_ARMCRYPTO` /
//! `__ARM_FEATURE_SHA3` are undefined in the reference build, so this is the
//! only backend that is ever selected.

use crate::common::{load64_le, memcpy, memset, rotl64, store64_le};

const KECCAK1600_STATEBYTES: usize = 200;

static keccak_round_constants: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

/* Lane naming used by the C macros: the first letter selects the row
 * (b, g, k, m, s -> y = 0..4), the second the column (a, e, i, o, u ->
 * x = 0..4).  Hence `Xba` == X[0], `Xbe` == X[1], ... `Xsu` == X[24].
 *
 * `c` holds (Ca, Ce, Ci, Co, Cu). */

/// `KECCAK_THETA_RHO_PI_CHI_IOTA_PRE(round_idx, A, E)`
#[inline(always)]
fn keccak_round_pre(round_idx: usize, a: &mut [u64; 25], e: &mut [u64; 25], c: &mut [u64; 5]) {
    let da: u64 = c[4] ^ rotl64(c[1], 1);
    let de: u64 = c[0] ^ rotl64(c[2], 1);
    let di: u64 = c[1] ^ rotl64(c[3], 1);
    let d_o: u64 = c[2] ^ rotl64(c[4], 1);
    let du: u64 = c[3] ^ rotl64(c[0], 1);

    a[0] ^= da;
    let bba = a[0];
    a[6] ^= de;
    let bbe = rotl64(a[6], 44);
    a[12] ^= di;
    let bbi = rotl64(a[12], 43);
    a[18] ^= d_o;
    let bbo = rotl64(a[18], 21);
    a[24] ^= du;
    let bbu = rotl64(a[24], 14);
    e[0] = bba ^ ((!bbe) & bbi);
    e[0] ^= keccak_round_constants[round_idx];
    c[0] = e[0];
    e[1] = bbe ^ ((!bbi) & bbo);
    c[1] = e[1];
    e[2] = bbi ^ ((!bbo) & bbu);
    c[2] = e[2];
    e[3] = bbo ^ ((!bbu) & bba);
    c[3] = e[3];
    e[4] = bbu ^ ((!bba) & bbe);
    c[4] = e[4];

    a[3] ^= d_o;
    let bga = rotl64(a[3], 28);
    a[9] ^= du;
    let bge = rotl64(a[9], 20);
    a[10] ^= da;
    let bgi = rotl64(a[10], 3);
    a[16] ^= de;
    let bgo = rotl64(a[16], 45);
    a[22] ^= di;
    let bgu = rotl64(a[22], 61);
    e[5] = bga ^ ((!bge) & bgi);
    c[0] ^= e[5];
    e[6] = bge ^ ((!bgi) & bgo);
    c[1] ^= e[6];
    e[7] = bgi ^ ((!bgo) & bgu);
    c[2] ^= e[7];
    e[8] = bgo ^ ((!bgu) & bga);
    c[3] ^= e[8];
    e[9] = bgu ^ ((!bga) & bge);
    c[4] ^= e[9];

    a[1] ^= de;
    let bka = rotl64(a[1], 1);
    a[7] ^= di;
    let bke = rotl64(a[7], 6);
    a[13] ^= d_o;
    let bki = rotl64(a[13], 25);
    a[19] ^= du;
    let bko = rotl64(a[19], 8);
    a[20] ^= da;
    let bku = rotl64(a[20], 18);
    e[10] = bka ^ ((!bke) & bki);
    c[0] ^= e[10];
    e[11] = bke ^ ((!bki) & bko);
    c[1] ^= e[11];
    e[12] = bki ^ ((!bko) & bku);
    c[2] ^= e[12];
    e[13] = bko ^ ((!bku) & bka);
    c[3] ^= e[13];
    e[14] = bku ^ ((!bka) & bke);
    c[4] ^= e[14];

    a[4] ^= du;
    let bma = rotl64(a[4], 27);
    a[5] ^= da;
    let bme = rotl64(a[5], 36);
    a[11] ^= de;
    let bmi = rotl64(a[11], 10);
    a[17] ^= di;
    let bmo = rotl64(a[17], 15);
    a[23] ^= d_o;
    let bmu = rotl64(a[23], 56);
    e[15] = bma ^ ((!bme) & bmi);
    c[0] ^= e[15];
    e[16] = bme ^ ((!bmi) & bmo);
    c[1] ^= e[16];
    e[17] = bmi ^ ((!bmo) & bmu);
    c[2] ^= e[17];
    e[18] = bmo ^ ((!bmu) & bma);
    c[3] ^= e[18];
    e[19] = bmu ^ ((!bma) & bme);
    c[4] ^= e[19];

    a[2] ^= di;
    let bsa = rotl64(a[2], 62);
    a[8] ^= d_o;
    let bse = rotl64(a[8], 55);
    a[14] ^= du;
    let bsi = rotl64(a[14], 39);
    a[15] ^= da;
    let bso = rotl64(a[15], 41);
    a[21] ^= de;
    let bsu = rotl64(a[21], 2);
    e[20] = bsa ^ ((!bse) & bsi);
    c[0] ^= e[20];
    e[21] = bse ^ ((!bsi) & bso);
    c[1] ^= e[21];
    e[22] = bsi ^ ((!bso) & bsu);
    c[2] ^= e[22];
    e[23] = bso ^ ((!bsu) & bsa);
    c[3] ^= e[23];
    e[24] = bsu ^ ((!bsa) & bse);
    c[4] ^= e[24];
}

/// `KECCAK_THETA_RHO_PI_CHI_IOTA(round_idx, A, E)` (last round: no new theta
/// parities are accumulated).
#[inline(always)]
fn keccak_round_last(round_idx: usize, a: &mut [u64; 25], e: &mut [u64; 25], c: &[u64; 5]) {
    let da: u64 = c[4] ^ rotl64(c[1], 1);
    let de: u64 = c[0] ^ rotl64(c[2], 1);
    let di: u64 = c[1] ^ rotl64(c[3], 1);
    let d_o: u64 = c[2] ^ rotl64(c[4], 1);
    let du: u64 = c[3] ^ rotl64(c[0], 1);

    a[0] ^= da;
    let bba = a[0];
    a[6] ^= de;
    let bbe = rotl64(a[6], 44);
    a[12] ^= di;
    let bbi = rotl64(a[12], 43);
    a[18] ^= d_o;
    let bbo = rotl64(a[18], 21);
    a[24] ^= du;
    let bbu = rotl64(a[24], 14);
    e[0] = bba ^ ((!bbe) & bbi);
    e[0] ^= keccak_round_constants[round_idx];
    e[1] = bbe ^ ((!bbi) & bbo);
    e[2] = bbi ^ ((!bbo) & bbu);
    e[3] = bbo ^ ((!bbu) & bba);
    e[4] = bbu ^ ((!bba) & bbe);

    a[3] ^= d_o;
    let bga = rotl64(a[3], 28);
    a[9] ^= du;
    let bge = rotl64(a[9], 20);
    a[10] ^= da;
    let bgi = rotl64(a[10], 3);
    a[16] ^= de;
    let bgo = rotl64(a[16], 45);
    a[22] ^= di;
    let bgu = rotl64(a[22], 61);
    e[5] = bga ^ ((!bge) & bgi);
    e[6] = bge ^ ((!bgi) & bgo);
    e[7] = bgi ^ ((!bgo) & bgu);
    e[8] = bgo ^ ((!bgu) & bga);
    e[9] = bgu ^ ((!bga) & bge);

    a[1] ^= de;
    let bka = rotl64(a[1], 1);
    a[7] ^= di;
    let bke = rotl64(a[7], 6);
    a[13] ^= d_o;
    let bki = rotl64(a[13], 25);
    a[19] ^= du;
    let bko = rotl64(a[19], 8);
    a[20] ^= da;
    let bku = rotl64(a[20], 18);
    e[10] = bka ^ ((!bke) & bki);
    e[11] = bke ^ ((!bki) & bko);
    e[12] = bki ^ ((!bko) & bku);
    e[13] = bko ^ ((!bku) & bka);
    e[14] = bku ^ ((!bka) & bke);

    a[4] ^= du;
    let bma = rotl64(a[4], 27);
    a[5] ^= da;
    let bme = rotl64(a[5], 36);
    a[11] ^= de;
    let bmi = rotl64(a[11], 10);
    a[17] ^= di;
    let bmo = rotl64(a[17], 15);
    a[23] ^= d_o;
    let bmu = rotl64(a[23], 56);
    e[15] = bma ^ ((!bme) & bmi);
    e[16] = bme ^ ((!bmi) & bmo);
    e[17] = bmi ^ ((!bmo) & bmu);
    e[18] = bmo ^ ((!bmu) & bma);
    e[19] = bmu ^ ((!bma) & bme);

    a[2] ^= di;
    let bsa = rotl64(a[2], 62);
    a[8] ^= d_o;
    let bse = rotl64(a[8], 55);
    a[14] ^= du;
    let bsi = rotl64(a[14], 39);
    a[15] ^= da;
    let bso = rotl64(a[15], 41);
    a[21] ^= de;
    let bsu = rotl64(a[21], 2);
    e[20] = bsa ^ ((!bse) & bsi);
    e[21] = bse ^ ((!bsi) & bso);
    e[22] = bsi ^ ((!bso) & bsu);
    e[23] = bso ^ ((!bsu) & bsa);
    e[24] = bsu ^ ((!bsa) & bse);
}

/// `KECCAK_PREPARE_THETA`
#[inline(always)]
fn keccak_prepare_theta(a: &[u64; 25], c: &mut [u64; 5]) {
    c[0] = a[0] ^ a[5] ^ a[10] ^ a[15] ^ a[20];
    c[1] = a[1] ^ a[6] ^ a[11] ^ a[16] ^ a[21];
    c[2] = a[2] ^ a[7] ^ a[12] ^ a[17] ^ a[22];
    c[3] = a[3] ^ a[8] ^ a[13] ^ a[18] ^ a[23];
    c[4] = a[4] ^ a[9] ^ a[14] ^ a[19] ^ a[24];
}

fn keccakf_24_rounds(st: &mut [u64; 25]) {
    let mut a: [u64; 25] = *st;
    let mut e: [u64; 25] = [0u64; 25];
    let mut c: [u64; 5] = [0u64; 5];

    keccak_prepare_theta(&a, &mut c);

    keccak_round_pre(0, &mut a, &mut e, &mut c);
    keccak_round_pre(1, &mut e, &mut a, &mut c);
    keccak_round_pre(2, &mut a, &mut e, &mut c);
    keccak_round_pre(3, &mut e, &mut a, &mut c);
    keccak_round_pre(4, &mut a, &mut e, &mut c);
    keccak_round_pre(5, &mut e, &mut a, &mut c);
    keccak_round_pre(6, &mut a, &mut e, &mut c);
    keccak_round_pre(7, &mut e, &mut a, &mut c);
    keccak_round_pre(8, &mut a, &mut e, &mut c);
    keccak_round_pre(9, &mut e, &mut a, &mut c);
    keccak_round_pre(10, &mut a, &mut e, &mut c);
    keccak_round_pre(11, &mut e, &mut a, &mut c);
    keccak_round_pre(12, &mut a, &mut e, &mut c);
    keccak_round_pre(13, &mut e, &mut a, &mut c);
    keccak_round_pre(14, &mut a, &mut e, &mut c);
    keccak_round_pre(15, &mut e, &mut a, &mut c);
    keccak_round_pre(16, &mut a, &mut e, &mut c);
    keccak_round_pre(17, &mut e, &mut a, &mut c);
    keccak_round_pre(18, &mut a, &mut e, &mut c);
    keccak_round_pre(19, &mut e, &mut a, &mut c);
    keccak_round_pre(20, &mut a, &mut e, &mut c);
    keccak_round_pre(21, &mut e, &mut a, &mut c);
    keccak_round_pre(22, &mut a, &mut e, &mut c);
    keccak_round_last(23, &mut e, &mut a, &c);

    *st = a;
}

fn keccakf_12_rounds(st: &mut [u64; 25]) {
    let mut a: [u64; 25] = *st;
    let mut e: [u64; 25] = [0u64; 25];
    let mut c: [u64; 5] = [0u64; 5];

    keccak_prepare_theta(&a, &mut c);

    keccak_round_pre(12, &mut a, &mut e, &mut c);
    keccak_round_pre(13, &mut e, &mut a, &mut c);
    keccak_round_pre(14, &mut a, &mut e, &mut c);
    keccak_round_pre(15, &mut e, &mut a, &mut c);
    keccak_round_pre(16, &mut a, &mut e, &mut c);
    keccak_round_pre(17, &mut e, &mut a, &mut c);
    keccak_round_pre(18, &mut a, &mut e, &mut c);
    keccak_round_pre(19, &mut e, &mut a, &mut c);
    keccak_round_pre(20, &mut a, &mut e, &mut c);
    keccak_round_pre(21, &mut e, &mut a, &mut c);
    keccak_round_pre(22, &mut a, &mut e, &mut c);
    keccak_round_last(23, &mut e, &mut a, &c);

    *st = a;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_24(state: *mut core::ffi::c_void) {
    let mut st: [u64; 25] = [0u64; 25];

    for i in 0usize..25usize {
        st[i] = unsafe { load64_le((state as *const u8).add(i * 8)) };
    }

    keccakf_24_rounds(&mut st);

    for i in 0usize..25usize {
        unsafe { store64_le((state as *mut u8).add(i * 8), st[i]) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_12(state: *mut core::ffi::c_void) {
    let mut st: [u64; 25] = [0u64; 25];

    for i in 0usize..25usize {
        st[i] = unsafe { load64_le((state as *const u8).add(i * 8)) };
    }

    keccakf_12_rounds(&mut st);

    for i in 0usize..25usize {
        unsafe { store64_le((state as *mut u8).add(i * 8), st[i]) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_init(state: *mut core::ffi::c_void) {
    unsafe { memset(state as *mut u8, 0, KECCAK1600_STATEBYTES) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_xor_bytes(
    state: *mut core::ffi::c_void,
    data: *const u8,
    offset: usize,
    length: usize,
) {
    let st: *mut u8 = state as *mut u8;
    let mut t: u64;

    let mut data = data;
    let mut offset = offset;
    let mut length = length;

    while length > 0 && (offset & 7) != 0 {
        unsafe { *st.add(offset) ^= *data };
        data = data.wrapping_add(1);
        offset += 1;
        length -= 1;
    }
    while length >= 8 {
        t = unsafe { load64_le(st.add(offset)) } ^ unsafe { load64_le(data) };
        unsafe { store64_le(st.add(offset), t) };
        data = data.wrapping_add(8);
        offset += 8;
        length -= 8;
    }
    while length > 0 {
        unsafe { *st.add(offset) ^= *data };
        data = data.wrapping_add(1);
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
    let st: *const u8 = state as *const u8;

    unsafe { memcpy(data, st.add(offset), length) };
}
