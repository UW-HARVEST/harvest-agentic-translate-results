use crate::common::{load64_le, memset, rotl64, store64_le};

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

/*
 * The C header declares:
 *   typedef struct CRYPTO_ALIGN(16) crypto_core_keccak1600_state {
 *       unsigned char opaque[224];
 *   } crypto_core_keccak1600_state;
 * wrapped in #pragma pack(push, 1). Since the sole member is a byte array,
 * packing is a no-op; the alignment is 16.
 */
#[repr(C, align(16))]
pub struct crypto_core_keccak1600_state {
    pub opaque: [u8; 224],
}

/* ---- keccakf permutation state, expressed with named lanes ---- */

struct KeccakState {
    aba: u64,
    abe: u64,
    abi: u64,
    abo: u64,
    abu: u64,
    aga: u64,
    age: u64,
    agi: u64,
    ago: u64,
    agu: u64,
    aka: u64,
    ake: u64,
    aki: u64,
    ako: u64,
    aku: u64,
    ama: u64,
    ame: u64,
    ami: u64,
    amo: u64,
    amu: u64,
    asa: u64,
    ase: u64,
    asi: u64,
    aso: u64,
    asu: u64,
    eba: u64,
    ebe: u64,
    ebi: u64,
    ebo: u64,
    ebu: u64,
    ega: u64,
    ege: u64,
    egi: u64,
    ego: u64,
    egu: u64,
    eka: u64,
    eke: u64,
    eki: u64,
    eko: u64,
    eku: u64,
    ema: u64,
    eme: u64,
    emi: u64,
    emo: u64,
    emu: u64,
    esa: u64,
    ese: u64,
    esi: u64,
    eso: u64,
    esu: u64,
}

macro_rules! copy_from_state {
    ($s:ident, a, $src:expr) => {{
        let src = $src;
        $s.aba = src[0];
        $s.abe = src[1];
        $s.abi = src[2];
        $s.abo = src[3];
        $s.abu = src[4];
        $s.aga = src[5];
        $s.age = src[6];
        $s.agi = src[7];
        $s.ago = src[8];
        $s.agu = src[9];
        $s.aka = src[10];
        $s.ake = src[11];
        $s.aki = src[12];
        $s.ako = src[13];
        $s.aku = src[14];
        $s.ama = src[15];
        $s.ame = src[16];
        $s.ami = src[17];
        $s.amo = src[18];
        $s.amu = src[19];
        $s.asa = src[20];
        $s.ase = src[21];
        $s.asi = src[22];
        $s.aso = src[23];
        $s.asu = src[24];
    }};
}

macro_rules! copy_to_state {
    ($dst:expr, $s:ident, a) => {{
        let dst = $dst;
        dst[0] = $s.aba;
        dst[1] = $s.abe;
        dst[2] = $s.abi;
        dst[3] = $s.abo;
        dst[4] = $s.abu;
        dst[5] = $s.aga;
        dst[6] = $s.age;
        dst[7] = $s.agi;
        dst[8] = $s.ago;
        dst[9] = $s.agu;
        dst[10] = $s.aka;
        dst[11] = $s.ake;
        dst[12] = $s.aki;
        dst[13] = $s.ako;
        dst[14] = $s.aku;
        dst[15] = $s.ama;
        dst[16] = $s.ame;
        dst[17] = $s.ami;
        dst[18] = $s.amo;
        dst[19] = $s.amu;
        dst[20] = $s.asa;
        dst[21] = $s.ase;
        dst[22] = $s.asi;
        dst[23] = $s.aso;
        dst[24] = $s.asu;
    }};
}

/*
 * The C code uses token-pasting on A/E prefixes to alternate the source and
 * destination register banks each round. In Rust we implement one full round
 * with an explicit (A-fields) -> (E-fields) mapping and a second with the
 * (E-fields) -> (A-fields) mapping, matching the macro expansion exactly.
 *
 * Threading of the Ca/Ce/Ci/Co/Cu accumulators, Da..Du and the Bxx temporaries
 * matches KECCAK_THETA_RHO_PI_CHI_IOTA / _PRE. The "_PRE" variant additionally
 * recomputes the C accumulators for use by the following round's theta step.
 */

macro_rules! round_a_to_e {
    ($s:ident, $round_idx:expr, $ca:ident, $ce:ident, $ci:ident, $co:ident, $cu:ident, pre) => {{
        round_body!(
            $s, $round_idx, $ca, $ce, $ci, $co, $cu, aba, age, aki, amo, asu, abo, agu, aka, ame,
            asi, abe, agi, ako, amu, asa, abu, aga, ake, ami, aso, abi, ago, aku, ama, ase, eba,
            ebe, ebi, ebo, ebu, ega, ege, egi, ego, egu, eka, eke, eki, eko, eku, ema, eme, emi,
            emo, emu, esa, ese, esi, eso, esu, true
        );
    }};
    ($s:ident, $round_idx:expr, $ca:ident, $ce:ident, $ci:ident, $co:ident, $cu:ident, last) => {{
        round_body!(
            $s, $round_idx, $ca, $ce, $ci, $co, $cu, aba, age, aki, amo, asu, abo, agu, aka, ame,
            asi, abe, agi, ako, amu, asa, abu, aga, ake, ami, aso, abi, ago, aku, ama, ase, eba,
            ebe, ebi, ebo, ebu, ega, ege, egi, ego, egu, eka, eke, eki, eko, eku, ema, eme, emi,
            emo, emu, esa, ese, esi, eso, esu, false
        );
    }};
}

macro_rules! round_e_to_a {
    ($s:ident, $round_idx:expr, $ca:ident, $ce:ident, $ci:ident, $co:ident, $cu:ident, pre) => {{
        round_body!(
            $s, $round_idx, $ca, $ce, $ci, $co, $cu, eba, ege, eki, emo, esu, ebo, egu, eka, eme,
            esi, ebe, egi, eko, emu, esa, ebu, ega, eke, emi, eso, ebi, ego, eku, ema, ese, aba,
            abe, abi, abo, abu, aga, age, agi, ago, agu, aka, ake, aki, ako, aku, ama, ame, ami,
            amo, amu, asa, ase, asi, aso, asu, true
        );
    }};
    ($s:ident, $round_idx:expr, $ca:ident, $ce:ident, $ci:ident, $co:ident, $cu:ident, last) => {{
        round_body!(
            $s, $round_idx, $ca, $ce, $ci, $co, $cu, eba, ege, eki, emo, esu, ebo, egu, eka, eme,
            esi, ebe, egi, eko, emu, esa, ebu, ega, eke, emi, eso, ebi, ego, eku, ema, ese, aba,
            abe, abi, abo, abu, aga, age, agi, ago, agu, aka, ake, aki, ako, aku, ama, ame, ami,
            amo, amu, asa, ase, asi, aso, asu, false
        );
    }};
}

/*
 * round_body implements one Keccak round.
 * The A-bank lane names are passed positionally in the exact order the C macro
 * references them; the E-bank output lane names follow. `$pre` selects whether
 * to recompute the C accumulators (true) or not (false, final round).
 */
macro_rules! round_body {
    ($s:ident, $round_idx:expr,
     $ca:ident, $ce:ident, $ci:ident, $co:ident, $cu:ident,
     // A-bank lanes, grouped by column as used in C
     $aba:ident, $age:ident, $aki:ident, $amo:ident, $asu:ident,
     $abo:ident, $agu:ident, $aka:ident, $ame:ident, $asi:ident,
     $abe:ident, $agi:ident, $ako:ident, $amu:ident, $asa:ident,
     $abu:ident, $aga:ident, $ake:ident, $ami:ident, $aso:ident,
     $abi:ident, $ago:ident, $aku:ident, $ama:ident, $ase:ident,
     // E-bank output lanes
     $eba:ident, $ebe:ident, $ebi:ident, $ebo:ident, $ebu:ident,
     $ega:ident, $ege:ident, $egi:ident, $ego:ident, $egu:ident,
     $eka:ident, $eke:ident, $eki:ident, $eko:ident, $eku:ident,
     $ema:ident, $eme:ident, $emi:ident, $emo:ident, $emu:ident,
     $esa:ident, $ese:ident, $esi:ident, $eso:ident, $esu:ident,
     $pre:expr) => {{
        let da = $cu ^ rotl64($ce, 1);
        let de = $ca ^ rotl64($ci, 1);
        let di = $ce ^ rotl64($co, 1);
        let dodo = $ci ^ rotl64($cu, 1);
        let du = $co ^ rotl64($ca, 1);

        $s.$aba ^= da;
        let bba = $s.$aba;
        $s.$age ^= de;
        let bbe = rotl64($s.$age, 44);
        $s.$aki ^= di;
        let bbi = rotl64($s.$aki, 43);
        $s.$amo ^= dodo;
        let bbo = rotl64($s.$amo, 21);
        $s.$asu ^= du;
        let bbu = rotl64($s.$asu, 14);
        $s.$eba = bba ^ ((!bbe) & bbi);
        $s.$eba ^= keccak_round_constants[$round_idx];
        $ca = $s.$eba;
        $s.$ebe = bbe ^ ((!bbi) & bbo);
        $ce = $s.$ebe;
        $s.$ebi = bbi ^ ((!bbo) & bbu);
        $ci = $s.$ebi;
        $s.$ebo = bbo ^ ((!bbu) & bba);
        $co = $s.$ebo;
        $s.$ebu = bbu ^ ((!bba) & bbe);
        $cu = $s.$ebu;

        $s.$abo ^= dodo;
        let bga = rotl64($s.$abo, 28);
        $s.$agu ^= du;
        let bge = rotl64($s.$agu, 20);
        $s.$aka ^= da;
        let bgi = rotl64($s.$aka, 3);
        $s.$ame ^= de;
        let bgo = rotl64($s.$ame, 45);
        $s.$asi ^= di;
        let bgu = rotl64($s.$asi, 61);
        $s.$ega = bga ^ ((!bge) & bgi);
        $ca ^= $s.$ega;
        $s.$ege = bge ^ ((!bgi) & bgo);
        $ce ^= $s.$ege;
        $s.$egi = bgi ^ ((!bgo) & bgu);
        $ci ^= $s.$egi;
        $s.$ego = bgo ^ ((!bgu) & bga);
        $co ^= $s.$ego;
        $s.$egu = bgu ^ ((!bga) & bge);
        $cu ^= $s.$egu;

        $s.$abe ^= de;
        let bka = rotl64($s.$abe, 1);
        $s.$agi ^= di;
        let bke = rotl64($s.$agi, 6);
        $s.$ako ^= dodo;
        let bki = rotl64($s.$ako, 25);
        $s.$amu ^= du;
        let bko = rotl64($s.$amu, 8);
        $s.$asa ^= da;
        let bku = rotl64($s.$asa, 18);
        $s.$eka = bka ^ ((!bke) & bki);
        $ca ^= $s.$eka;
        $s.$eke = bke ^ ((!bki) & bko);
        $ce ^= $s.$eke;
        $s.$eki = bki ^ ((!bko) & bku);
        $ci ^= $s.$eki;
        $s.$eko = bko ^ ((!bku) & bka);
        $co ^= $s.$eko;
        $s.$eku = bku ^ ((!bka) & bke);
        $cu ^= $s.$eku;

        $s.$abu ^= du;
        let bma = rotl64($s.$abu, 27);
        $s.$aga ^= da;
        let bme = rotl64($s.$aga, 36);
        $s.$ake ^= de;
        let bmi = rotl64($s.$ake, 10);
        $s.$ami ^= di;
        let bmo = rotl64($s.$ami, 15);
        $s.$aso ^= dodo;
        let bmu = rotl64($s.$aso, 56);
        $s.$ema = bma ^ ((!bme) & bmi);
        $ca ^= $s.$ema;
        $s.$eme = bme ^ ((!bmi) & bmo);
        $ce ^= $s.$eme;
        $s.$emi = bmi ^ ((!bmo) & bmu);
        $ci ^= $s.$emi;
        $s.$emo = bmo ^ ((!bmu) & bma);
        $co ^= $s.$emo;
        $s.$emu = bmu ^ ((!bma) & bme);
        $cu ^= $s.$emu;

        $s.$abi ^= di;
        let bsa = rotl64($s.$abi, 62);
        $s.$ago ^= dodo;
        let bse = rotl64($s.$ago, 55);
        $s.$aku ^= du;
        let bsi = rotl64($s.$aku, 39);
        $s.$ama ^= da;
        let bso = rotl64($s.$ama, 41);
        $s.$ase ^= de;
        let bsu = rotl64($s.$ase, 2);
        $s.$esa = bsa ^ ((!bse) & bsi);
        $ca ^= $s.$esa;
        $s.$ese = bse ^ ((!bsi) & bso);
        $ce ^= $s.$ese;
        $s.$esi = bsi ^ ((!bso) & bsu);
        $ci ^= $s.$esi;
        $s.$eso = bso ^ ((!bsu) & bsa);
        $co ^= $s.$eso;
        $s.$esu = bsu ^ ((!bsa) & bse);
        $cu ^= $s.$esu;

        let _ = $pre;
    }};
}

fn keccakf_24_rounds(st: &mut [u64; 25]) {
    let mut s: KeccakState = unsafe { core::mem::zeroed() };

    copy_from_state!(s, a, *st);

    let mut ca = s.aba ^ s.aga ^ s.aka ^ s.ama ^ s.asa;
    let mut ce = s.abe ^ s.age ^ s.ake ^ s.ame ^ s.ase;
    let mut ci = s.abi ^ s.agi ^ s.aki ^ s.ami ^ s.asi;
    let mut co = s.abo ^ s.ago ^ s.ako ^ s.amo ^ s.aso;
    let mut cu = s.abu ^ s.agu ^ s.aku ^ s.amu ^ s.asu;

    round_a_to_e!(s, 0, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 1, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 2, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 3, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 4, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 5, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 6, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 7, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 8, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 9, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 10, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 11, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 12, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 13, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 14, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 15, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 16, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 17, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 18, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 19, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 20, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 21, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 22, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 23, ca, ce, ci, co, cu, last);

    copy_to_state!(&mut *st, s, a);
}

fn keccakf_12_rounds(st: &mut [u64; 25]) {
    let mut s: KeccakState = unsafe { core::mem::zeroed() };

    copy_from_state!(s, a, *st);

    let mut ca = s.aba ^ s.aga ^ s.aka ^ s.ama ^ s.asa;
    let mut ce = s.abe ^ s.age ^ s.ake ^ s.ame ^ s.ase;
    let mut ci = s.abi ^ s.agi ^ s.aki ^ s.ami ^ s.asi;
    let mut co = s.abo ^ s.ago ^ s.ako ^ s.amo ^ s.aso;
    let mut cu = s.abu ^ s.agu ^ s.aku ^ s.amu ^ s.asu;

    round_a_to_e!(s, 12, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 13, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 14, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 15, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 16, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 17, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 18, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 19, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 20, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 21, ca, ce, ci, co, cu, pre);
    round_a_to_e!(s, 22, ca, ce, ci, co, cu, pre);
    round_e_to_a!(s, 23, ca, ce, ci, co, cu, last);

    copy_to_state!(&mut *st, s, a);
}

/* ---- reference _sodium_-prefixed exported functions ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_24(state: *mut core::ffi::c_void) {
    let mut st: [u64; 25] = [0; 25];
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
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_12(state: *mut core::ffi::c_void) {
    let mut st: [u64; 25] = [0; 25];
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
pub unsafe extern "C" fn _sodium_keccak1600_ref_init(state: *mut core::ffi::c_void) {
    memset(state as *mut u8, 0, KECCAK1600_STATEBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_xor_bytes(
    state: *mut core::ffi::c_void,
    mut data: *const u8,
    mut offset: usize,
    mut length: usize,
) {
    let st = state as *mut u8;
    let mut t: u64;

    while length > 0 && (offset & 7) != 0 {
        *st.add(offset) ^= *data;
        data = data.add(1);
        offset += 1;
        length -= 1;
    }
    while length >= 8 {
        t = load64_le(st.add(offset)) ^ load64_le(data);
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

    crate::common::memcpy(data, st.add(offset), length);
}

/* ---- public crypto_core_keccak1600 API (from keccak1600.c) ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_keccak1600_statebytes() -> usize {
    core::mem::size_of::<crypto_core_keccak1600_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_init(state: *mut crypto_core_keccak1600_state) {
    _sodium_keccak1600_ref_init((*state).opaque.as_mut_ptr() as *mut core::ffi::c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_xor_bytes(
    state: *mut crypto_core_keccak1600_state,
    bytes: *const u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_xor_bytes(
        (*state).opaque.as_mut_ptr() as *mut core::ffi::c_void,
        bytes,
        offset,
        length,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_extract_bytes(
    state: *const crypto_core_keccak1600_state,
    bytes: *mut u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_extract_bytes(
        (*state).opaque.as_ptr() as *const core::ffi::c_void,
        bytes,
        offset,
        length,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_24(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_24((*state).opaque.as_mut_ptr() as *mut core::ffi::c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_12(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_12((*state).opaque.as_mut_ptr() as *mut core::ffi::c_void);
}
