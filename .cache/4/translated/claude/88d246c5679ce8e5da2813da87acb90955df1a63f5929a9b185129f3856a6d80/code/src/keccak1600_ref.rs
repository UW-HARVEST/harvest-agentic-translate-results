//! Translation of `crypto_core/keccak1600/ref/keccak1600_ref.c`.
//!
//! The reference build defines neither `HAVE_ARMSHA3` nor `__ARM_FEATURE_SHA3`,
//! so this portable implementation is the one that gets selected by
//! `crypto_core/keccak1600/keccak1600.c`.
//!
//! `private/quirks.h` renames every non-`static` function of this file with a
//! `_sodium_` prefix, hence the `no_mangle` names below.

use crate::common::*;
use core::ffi::c_void;

const KECCAK1600_STATEBYTES: usize = 200;

static KECCAK_ROUND_CONSTANTS: [u64; 24] = [
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

/* Lane indices matching the `xy` suffixes used by the C macros
 * (`ba` == state[0], `be` == state[1], ... `su` == state[24]). */
const BA: usize = 0;
const BE: usize = 1;
const BI: usize = 2;
const BO: usize = 3;
const BU: usize = 4;
const GA: usize = 5;
const GE: usize = 6;
const GI: usize = 7;
const GO: usize = 8;
const GU: usize = 9;
const KA: usize = 10;
const KE: usize = 11;
const KI: usize = 12;
const KO: usize = 13;
const KU: usize = 14;
const MA: usize = 15;
const ME: usize = 16;
const MI: usize = 17;
const MO: usize = 18;
const MU: usize = 19;
const SA: usize = 20;
const SE: usize = 21;
const SI: usize = 22;
const SO: usize = 23;
const SU: usize = 24;

/* KECCAK_PREPARE_THETA */
#[inline(always)]
fn keccak_prepare_theta(a: &[u64; 25], c: &mut [u64; 5]) {
    c[0] = a[BA] ^ a[GA] ^ a[KA] ^ a[MA] ^ a[SA];
    c[1] = a[BE] ^ a[GE] ^ a[KE] ^ a[ME] ^ a[SE];
    c[2] = a[BI] ^ a[GI] ^ a[KI] ^ a[MI] ^ a[SI];
    c[3] = a[BO] ^ a[GO] ^ a[KO] ^ a[MO] ^ a[SO];
    c[4] = a[BU] ^ a[GU] ^ a[KU] ^ a[MU] ^ a[SU];
}

/* KECCAK_THETA_RHO_PI_CHI_IOTA_PRE(round_idx, A, E) */
#[inline(always)]
fn keccak_theta_rho_pi_chi_iota_pre(
    round_idx: usize,
    a: &mut [u64; 25],
    e: &mut [u64; 25],
    c: &mut [u64; 5],
) {
    let (ca_in, ce_in, ci_in, co_in, cu_in) = (c[0], c[1], c[2], c[3], c[4]);

    let da = cu_in ^ rotl64(ce_in, 1);
    let de = ca_in ^ rotl64(ci_in, 1);
    let di = ce_in ^ rotl64(co_in, 1);
    let do_ = ci_in ^ rotl64(cu_in, 1);
    let du = co_in ^ rotl64(ca_in, 1);

    a[BA] ^= da;
    let bba = a[BA];
    a[GE] ^= de;
    let bbe = rotl64(a[GE], 44);
    a[KI] ^= di;
    let bbi = rotl64(a[KI], 43);
    a[MO] ^= do_;
    let bbo = rotl64(a[MO], 21);
    a[SU] ^= du;
    let bbu = rotl64(a[SU], 14);
    e[BA] = bba ^ ((!bbe) & bbi);
    e[BA] ^= KECCAK_ROUND_CONSTANTS[round_idx];
    let mut ca = e[BA];
    e[BE] = bbe ^ ((!bbi) & bbo);
    let mut ce = e[BE];
    e[BI] = bbi ^ ((!bbo) & bbu);
    let mut ci = e[BI];
    e[BO] = bbo ^ ((!bbu) & bba);
    let mut co = e[BO];
    e[BU] = bbu ^ ((!bba) & bbe);
    let mut cu = e[BU];

    a[BO] ^= do_;
    let bga = rotl64(a[BO], 28);
    a[GU] ^= du;
    let bge = rotl64(a[GU], 20);
    a[KA] ^= da;
    let bgi = rotl64(a[KA], 3);
    a[ME] ^= de;
    let bgo = rotl64(a[ME], 45);
    a[SI] ^= di;
    let bgu = rotl64(a[SI], 61);
    e[GA] = bga ^ ((!bge) & bgi);
    ca ^= e[GA];
    e[GE] = bge ^ ((!bgi) & bgo);
    ce ^= e[GE];
    e[GI] = bgi ^ ((!bgo) & bgu);
    ci ^= e[GI];
    e[GO] = bgo ^ ((!bgu) & bga);
    co ^= e[GO];
    e[GU] = bgu ^ ((!bga) & bge);
    cu ^= e[GU];

    a[BE] ^= de;
    let bka = rotl64(a[BE], 1);
    a[GI] ^= di;
    let bke = rotl64(a[GI], 6);
    a[KO] ^= do_;
    let bki = rotl64(a[KO], 25);
    a[MU] ^= du;
    let bko = rotl64(a[MU], 8);
    a[SA] ^= da;
    let bku = rotl64(a[SA], 18);
    e[KA] = bka ^ ((!bke) & bki);
    ca ^= e[KA];
    e[KE] = bke ^ ((!bki) & bko);
    ce ^= e[KE];
    e[KI] = bki ^ ((!bko) & bku);
    ci ^= e[KI];
    e[KO] = bko ^ ((!bku) & bka);
    co ^= e[KO];
    e[KU] = bku ^ ((!bka) & bke);
    cu ^= e[KU];

    a[BU] ^= du;
    let bma = rotl64(a[BU], 27);
    a[GA] ^= da;
    let bme = rotl64(a[GA], 36);
    a[KE] ^= de;
    let bmi = rotl64(a[KE], 10);
    a[MI] ^= di;
    let bmo = rotl64(a[MI], 15);
    a[SO] ^= do_;
    let bmu = rotl64(a[SO], 56);
    e[MA] = bma ^ ((!bme) & bmi);
    ca ^= e[MA];
    e[ME] = bme ^ ((!bmi) & bmo);
    ce ^= e[ME];
    e[MI] = bmi ^ ((!bmo) & bmu);
    ci ^= e[MI];
    e[MO] = bmo ^ ((!bmu) & bma);
    co ^= e[MO];
    e[MU] = bmu ^ ((!bma) & bme);
    cu ^= e[MU];

    a[BI] ^= di;
    let bsa = rotl64(a[BI], 62);
    a[GO] ^= do_;
    let bse = rotl64(a[GO], 55);
    a[KU] ^= du;
    let bsi = rotl64(a[KU], 39);
    a[MA] ^= da;
    let bso = rotl64(a[MA], 41);
    a[SE] ^= de;
    let bsu = rotl64(a[SE], 2);
    e[SA] = bsa ^ ((!bse) & bsi);
    ca ^= e[SA];
    e[SE] = bse ^ ((!bsi) & bso);
    ce ^= e[SE];
    e[SI] = bsi ^ ((!bso) & bsu);
    ci ^= e[SI];
    e[SO] = bso ^ ((!bsu) & bsa);
    co ^= e[SO];
    e[SU] = bsu ^ ((!bsa) & bse);
    cu ^= e[SU];

    c[0] = ca;
    c[1] = ce;
    c[2] = ci;
    c[3] = co;
    c[4] = cu;
}

/* KECCAK_THETA_RHO_PI_CHI_IOTA(round_idx, A, E) -- identical to the `_PRE`
 * variant except that the column parities are not recomputed (this is only
 * used for the very last round). */
#[inline(always)]
fn keccak_theta_rho_pi_chi_iota(
    round_idx: usize,
    a: &mut [u64; 25],
    e: &mut [u64; 25],
    c: &[u64; 5],
) {
    let (ca_in, ce_in, ci_in, co_in, cu_in) = (c[0], c[1], c[2], c[3], c[4]);

    let da = cu_in ^ rotl64(ce_in, 1);
    let de = ca_in ^ rotl64(ci_in, 1);
    let di = ce_in ^ rotl64(co_in, 1);
    let do_ = ci_in ^ rotl64(cu_in, 1);
    let du = co_in ^ rotl64(ca_in, 1);

    a[BA] ^= da;
    let bba = a[BA];
    a[GE] ^= de;
    let bbe = rotl64(a[GE], 44);
    a[KI] ^= di;
    let bbi = rotl64(a[KI], 43);
    a[MO] ^= do_;
    let bbo = rotl64(a[MO], 21);
    a[SU] ^= du;
    let bbu = rotl64(a[SU], 14);
    e[BA] = bba ^ ((!bbe) & bbi);
    e[BA] ^= KECCAK_ROUND_CONSTANTS[round_idx];
    e[BE] = bbe ^ ((!bbi) & bbo);
    e[BI] = bbi ^ ((!bbo) & bbu);
    e[BO] = bbo ^ ((!bbu) & bba);
    e[BU] = bbu ^ ((!bba) & bbe);

    a[BO] ^= do_;
    let bga = rotl64(a[BO], 28);
    a[GU] ^= du;
    let bge = rotl64(a[GU], 20);
    a[KA] ^= da;
    let bgi = rotl64(a[KA], 3);
    a[ME] ^= de;
    let bgo = rotl64(a[ME], 45);
    a[SI] ^= di;
    let bgu = rotl64(a[SI], 61);
    e[GA] = bga ^ ((!bge) & bgi);
    e[GE] = bge ^ ((!bgi) & bgo);
    e[GI] = bgi ^ ((!bgo) & bgu);
    e[GO] = bgo ^ ((!bgu) & bga);
    e[GU] = bgu ^ ((!bga) & bge);

    a[BE] ^= de;
    let bka = rotl64(a[BE], 1);
    a[GI] ^= di;
    let bke = rotl64(a[GI], 6);
    a[KO] ^= do_;
    let bki = rotl64(a[KO], 25);
    a[MU] ^= du;
    let bko = rotl64(a[MU], 8);
    a[SA] ^= da;
    let bku = rotl64(a[SA], 18);
    e[KA] = bka ^ ((!bke) & bki);
    e[KE] = bke ^ ((!bki) & bko);
    e[KI] = bki ^ ((!bko) & bku);
    e[KO] = bko ^ ((!bku) & bka);
    e[KU] = bku ^ ((!bka) & bke);

    a[BU] ^= du;
    let bma = rotl64(a[BU], 27);
    a[GA] ^= da;
    let bme = rotl64(a[GA], 36);
    a[KE] ^= de;
    let bmi = rotl64(a[KE], 10);
    a[MI] ^= di;
    let bmo = rotl64(a[MI], 15);
    a[SO] ^= do_;
    let bmu = rotl64(a[SO], 56);
    e[MA] = bma ^ ((!bme) & bmi);
    e[ME] = bme ^ ((!bmi) & bmo);
    e[MI] = bmi ^ ((!bmo) & bmu);
    e[MO] = bmo ^ ((!bmu) & bma);
    e[MU] = bmu ^ ((!bma) & bme);

    a[BI] ^= di;
    let bsa = rotl64(a[BI], 62);
    a[GO] ^= do_;
    let bse = rotl64(a[GO], 55);
    a[KU] ^= du;
    let bsi = rotl64(a[KU], 39);
    a[MA] ^= da;
    let bso = rotl64(a[MA], 41);
    a[SE] ^= de;
    let bsu = rotl64(a[SE], 2);
    e[SA] = bsa ^ ((!bse) & bsi);
    e[SE] = bse ^ ((!bsi) & bso);
    e[SI] = bsi ^ ((!bso) & bsu);
    e[SO] = bso ^ ((!bsu) & bsa);
    e[SU] = bsu ^ ((!bsa) & bse);
}

fn keccakf_24_rounds(st: &mut [u64; 25]) {
    /* KECCAK_DECLARE_STATE / KECCAK_COPY_FROM_STATE(A, state) */
    let mut a: [u64; 25] = *st;
    let mut e: [u64; 25] = [0u64; 25];
    let mut c: [u64; 5] = [0u64; 5];

    keccak_prepare_theta(&a, &mut c);

    keccak_theta_rho_pi_chi_iota_pre(0, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(1, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(2, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(3, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(4, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(5, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(6, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(7, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(8, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(9, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(10, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(11, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(12, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(13, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(14, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(15, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(16, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(17, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(18, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(19, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(20, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(21, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(22, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota(23, &mut e, &mut a, &c);

    /* KECCAK_COPY_TO_STATE(state, A) */
    *st = a;
}

fn keccakf_12_rounds(st: &mut [u64; 25]) {
    let mut a: [u64; 25] = *st;
    let mut e: [u64; 25] = [0u64; 25];
    let mut c: [u64; 5] = [0u64; 5];

    keccak_prepare_theta(&a, &mut c);

    keccak_theta_rho_pi_chi_iota_pre(12, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(13, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(14, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(15, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(16, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(17, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(18, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(19, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(20, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(21, &mut e, &mut a, &mut c);
    keccak_theta_rho_pi_chi_iota_pre(22, &mut a, &mut e, &mut c);
    keccak_theta_rho_pi_chi_iota(23, &mut e, &mut a, &c);

    *st = a;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_24(state: *mut c_void) {
    let mut st: [u64; 25] = [0u64; 25];

    let mut i: usize = 0;
    while i < 25 {
        st[i] = load64_le((state as *const u8).add(i * 8));
        i += 1;
    }

    keccakf_24_rounds(&mut st);

    let mut i: usize = 0;
    while i < 25 {
        store64_le((state as *mut u8).add(i * 8), st[i]);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_12(state: *mut c_void) {
    let mut st: [u64; 25] = [0u64; 25];

    let mut i: usize = 0;
    while i < 25 {
        st[i] = load64_le((state as *const u8).add(i * 8));
        i += 1;
    }

    keccakf_12_rounds(&mut st);

    let mut i: usize = 0;
    while i < 25 {
        store64_le((state as *mut u8).add(i * 8), st[i]);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_init(state: *mut c_void) {
    memset(state as *mut u8, 0, KECCAK1600_STATEBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_xor_bytes(
    state: *mut c_void,
    data: *const u8,
    offset: usize,
    length: usize,
) {
    let st = state as *mut u8;
    let mut data = data;
    let mut offset = offset;
    let mut length = length;

    while length > 0 && (offset & 7) != 0 {
        *st.add(offset) ^= *data;
        data = data.add(1);
        offset += 1;
        length -= 1;
    }
    while length >= 8 {
        let t: u64 = load64_le(st.add(offset) as *const u8) ^ load64_le(data);
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
    state: *const c_void,
    data: *mut u8,
    offset: usize,
    length: usize,
) {
    let st = state as *const u8;

    memcpy(data, st.add(offset), length);
}
