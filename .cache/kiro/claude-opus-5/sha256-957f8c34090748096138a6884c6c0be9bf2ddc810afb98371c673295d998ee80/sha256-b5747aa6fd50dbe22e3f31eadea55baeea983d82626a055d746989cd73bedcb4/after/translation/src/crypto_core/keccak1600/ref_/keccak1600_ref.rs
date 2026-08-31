//! Translation of c_src/libsodium/crypto_core/keccak1600/ref/keccak1600_ref.c

use crate::common::{load64_le, rotl64, store64_le};
use core::ffi::c_void;

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

// The C source uses a set of preprocessor macros (KECCAK_DECLARE_STATE,
// KECCAK_PREPARE_THETA, KECCAK_THETA_RHO_PI_CHI_IOTA[_PRE],
// KECCAK_COPY_FROM_STATE, KECCAK_COPY_TO_STATE) that operate on ~75 named
// local u64 variables. We reproduce them as Rust macro_rules! that manipulate
// those same locals, so the emitted arithmetic and evaluation order match the
// C exactly.

macro_rules! keccak_prepare_theta {
    (
        $Aba:ident,$Abe:ident,$Abi:ident,$Abo:ident,$Abu:ident,
        $Aga:ident,$Age:ident,$Agi:ident,$Ago:ident,$Agu:ident,
        $Aka:ident,$Ake:ident,$Aki:ident,$Ako:ident,$Aku:ident,
        $Ama:ident,$Ame:ident,$Ami:ident,$Amo:ident,$Amu:ident,
        $Asa:ident,$Ase:ident,$Asi:ident,$Aso:ident,$Asu:ident,
        $Ca:ident,$Ce:ident,$Ci:ident,$Co:ident,$Cu:ident
    ) => {
        $Ca = $Aba ^ $Aga ^ $Aka ^ $Ama ^ $Asa;
        $Ce = $Abe ^ $Age ^ $Ake ^ $Ame ^ $Ase;
        $Ci = $Abi ^ $Agi ^ $Aki ^ $Ami ^ $Asi;
        $Co = $Abo ^ $Ago ^ $Ako ^ $Amo ^ $Aso;
        $Cu = $Abu ^ $Agu ^ $Aku ^ $Amu ^ $Asu;
    };
}

// KECCAK_THETA_RHO_PI_CHI_IOTA_PRE(round_idx, A, E)
// Recomputes C{a,e,i,o,u} from the freshly written E lanes.
macro_rules! keccak_round_pre {
    (
        $round_idx:expr,
        // A## lanes (source)
        $Aba:ident,$Abe:ident,$Abi:ident,$Abo:ident,$Abu:ident,
        $Aga:ident,$Age:ident,$Agi:ident,$Ago:ident,$Agu:ident,
        $Aka:ident,$Ake:ident,$Aki:ident,$Ako:ident,$Aku:ident,
        $Ama:ident,$Ame:ident,$Ami:ident,$Amo:ident,$Amu:ident,
        $Asa:ident,$Ase:ident,$Asi:ident,$Aso:ident,$Asu:ident,
        // E## lanes (dest)
        $Eba:ident,$Ebe:ident,$Ebi:ident,$Ebo:ident,$Ebu:ident,
        $Ega:ident,$Ege:ident,$Egi:ident,$Ego:ident,$Egu:ident,
        $Eka:ident,$Eke:ident,$Eki:ident,$Eko:ident,$Eku:ident,
        $Ema:ident,$Eme:ident,$Emi:ident,$Emo:ident,$Emu:ident,
        $Esa:ident,$Ese:ident,$Esi:ident,$Eso:ident,$Esu:ident,
        // B temporaries
        $Bba:ident,$Bbe:ident,$Bbi:ident,$Bbo:ident,$Bbu:ident,
        $Bga:ident,$Bge:ident,$Bgi:ident,$Bgo:ident,$Bgu:ident,
        $Bka:ident,$Bke:ident,$Bki:ident,$Bko:ident,$Bku:ident,
        $Bma:ident,$Bme:ident,$Bmi:ident,$Bmo:ident,$Bmu:ident,
        $Bsa:ident,$Bse:ident,$Bsi:ident,$Bso:ident,$Bsu:ident,
        // C/D
        $Ca:ident,$Ce:ident,$Ci:ident,$Co:ident,$Cu:ident,
        $Da:ident,$De:ident,$Di:ident,$Do:ident,$Du:ident
    ) => {
        $Da = $Cu ^ rotl64($Ce, 1);
        $De = $Ca ^ rotl64($Ci, 1);
        $Di = $Ce ^ rotl64($Co, 1);
        $Do = $Ci ^ rotl64($Cu, 1);
        $Du = $Co ^ rotl64($Ca, 1);

        $Aba ^= $Da;
        $Bba = $Aba;
        $Age ^= $De;
        $Bbe = rotl64($Age, 44);
        $Aki ^= $Di;
        $Bbi = rotl64($Aki, 43);
        $Amo ^= $Do;
        $Bbo = rotl64($Amo, 21);
        $Asu ^= $Du;
        $Bbu = rotl64($Asu, 14);
        $Eba = $Bba ^ ((!$Bbe) & $Bbi);
        $Eba ^= keccak_round_constants[$round_idx];
        $Ca = $Eba;
        $Ebe = $Bbe ^ ((!$Bbi) & $Bbo);
        $Ce = $Ebe;
        $Ebi = $Bbi ^ ((!$Bbo) & $Bbu);
        $Ci = $Ebi;
        $Ebo = $Bbo ^ ((!$Bbu) & $Bba);
        $Co = $Ebo;
        $Ebu = $Bbu ^ ((!$Bba) & $Bbe);
        $Cu = $Ebu;

        $Abo ^= $Do;
        $Bga = rotl64($Abo, 28);
        $Agu ^= $Du;
        $Bge = rotl64($Agu, 20);
        $Aka ^= $Da;
        $Bgi = rotl64($Aka, 3);
        $Ame ^= $De;
        $Bgo = rotl64($Ame, 45);
        $Asi ^= $Di;
        $Bgu = rotl64($Asi, 61);
        $Ega = $Bga ^ ((!$Bge) & $Bgi);
        $Ca ^= $Ega;
        $Ege = $Bge ^ ((!$Bgi) & $Bgo);
        $Ce ^= $Ege;
        $Egi = $Bgi ^ ((!$Bgo) & $Bgu);
        $Ci ^= $Egi;
        $Ego = $Bgo ^ ((!$Bgu) & $Bga);
        $Co ^= $Ego;
        $Egu = $Bgu ^ ((!$Bga) & $Bge);
        $Cu ^= $Egu;

        $Abe ^= $De;
        $Bka = rotl64($Abe, 1);
        $Agi ^= $Di;
        $Bke = rotl64($Agi, 6);
        $Ako ^= $Do;
        $Bki = rotl64($Ako, 25);
        $Amu ^= $Du;
        $Bko = rotl64($Amu, 8);
        $Asa ^= $Da;
        $Bku = rotl64($Asa, 18);
        $Eka = $Bka ^ ((!$Bke) & $Bki);
        $Ca ^= $Eka;
        $Eke = $Bke ^ ((!$Bki) & $Bko);
        $Ce ^= $Eke;
        $Eki = $Bki ^ ((!$Bko) & $Bku);
        $Ci ^= $Eki;
        $Eko = $Bko ^ ((!$Bku) & $Bka);
        $Co ^= $Eko;
        $Eku = $Bku ^ ((!$Bka) & $Bke);
        $Cu ^= $Eku;

        $Abu ^= $Du;
        $Bma = rotl64($Abu, 27);
        $Aga ^= $Da;
        $Bme = rotl64($Aga, 36);
        $Ake ^= $De;
        $Bmi = rotl64($Ake, 10);
        $Ami ^= $Di;
        $Bmo = rotl64($Ami, 15);
        $Aso ^= $Do;
        $Bmu = rotl64($Aso, 56);
        $Ema = $Bma ^ ((!$Bme) & $Bmi);
        $Ca ^= $Ema;
        $Eme = $Bme ^ ((!$Bmi) & $Bmo);
        $Ce ^= $Eme;
        $Emi = $Bmi ^ ((!$Bmo) & $Bmu);
        $Ci ^= $Emi;
        $Emo = $Bmo ^ ((!$Bmu) & $Bma);
        $Co ^= $Emo;
        $Emu = $Bmu ^ ((!$Bma) & $Bme);
        $Cu ^= $Emu;

        $Abi ^= $Di;
        $Bsa = rotl64($Abi, 62);
        $Ago ^= $Do;
        $Bse = rotl64($Ago, 55);
        $Aku ^= $Du;
        $Bsi = rotl64($Aku, 39);
        $Ama ^= $Da;
        $Bso = rotl64($Ama, 41);
        $Ase ^= $De;
        $Bsu = rotl64($Ase, 2);
        $Esa = $Bsa ^ ((!$Bse) & $Bsi);
        $Ca ^= $Esa;
        $Ese = $Bse ^ ((!$Bsi) & $Bso);
        $Ce ^= $Ese;
        $Esi = $Bsi ^ ((!$Bso) & $Bsu);
        $Ci ^= $Esi;
        $Eso = $Bso ^ ((!$Bsu) & $Bsa);
        $Co ^= $Eso;
        $Esu = $Bsu ^ ((!$Bsa) & $Bse);
        $Cu ^= $Esu;
    };
}

// KECCAK_THETA_RHO_PI_CHI_IOTA(round_idx, A, E) — final round, no C recompute.
macro_rules! keccak_round {
    (
        $round_idx:expr,
        $Aba:ident,$Abe:ident,$Abi:ident,$Abo:ident,$Abu:ident,
        $Aga:ident,$Age:ident,$Agi:ident,$Ago:ident,$Agu:ident,
        $Aka:ident,$Ake:ident,$Aki:ident,$Ako:ident,$Aku:ident,
        $Ama:ident,$Ame:ident,$Ami:ident,$Amo:ident,$Amu:ident,
        $Asa:ident,$Ase:ident,$Asi:ident,$Aso:ident,$Asu:ident,
        $Eba:ident,$Ebe:ident,$Ebi:ident,$Ebo:ident,$Ebu:ident,
        $Ega:ident,$Ege:ident,$Egi:ident,$Ego:ident,$Egu:ident,
        $Eka:ident,$Eke:ident,$Eki:ident,$Eko:ident,$Eku:ident,
        $Ema:ident,$Eme:ident,$Emi:ident,$Emo:ident,$Emu:ident,
        $Esa:ident,$Ese:ident,$Esi:ident,$Eso:ident,$Esu:ident,
        $Bba:ident,$Bbe:ident,$Bbi:ident,$Bbo:ident,$Bbu:ident,
        $Bga:ident,$Bge:ident,$Bgi:ident,$Bgo:ident,$Bgu:ident,
        $Bka:ident,$Bke:ident,$Bki:ident,$Bko:ident,$Bku:ident,
        $Bma:ident,$Bme:ident,$Bmi:ident,$Bmo:ident,$Bmu:ident,
        $Bsa:ident,$Bse:ident,$Bsi:ident,$Bso:ident,$Bsu:ident,
        $Ca:ident,$Ce:ident,$Ci:ident,$Co:ident,$Cu:ident,
        $Da:ident,$De:ident,$Di:ident,$Do:ident,$Du:ident
    ) => {
        $Da = $Cu ^ rotl64($Ce, 1);
        $De = $Ca ^ rotl64($Ci, 1);
        $Di = $Ce ^ rotl64($Co, 1);
        $Do = $Ci ^ rotl64($Cu, 1);
        $Du = $Co ^ rotl64($Ca, 1);

        $Aba ^= $Da;
        $Bba = $Aba;
        $Age ^= $De;
        $Bbe = rotl64($Age, 44);
        $Aki ^= $Di;
        $Bbi = rotl64($Aki, 43);
        $Amo ^= $Do;
        $Bbo = rotl64($Amo, 21);
        $Asu ^= $Du;
        $Bbu = rotl64($Asu, 14);
        $Eba = $Bba ^ ((!$Bbe) & $Bbi);
        $Eba ^= keccak_round_constants[$round_idx];
        $Ebe = $Bbe ^ ((!$Bbi) & $Bbo);
        $Ebi = $Bbi ^ ((!$Bbo) & $Bbu);
        $Ebo = $Bbo ^ ((!$Bbu) & $Bba);
        $Ebu = $Bbu ^ ((!$Bba) & $Bbe);

        $Abo ^= $Do;
        $Bga = rotl64($Abo, 28);
        $Agu ^= $Du;
        $Bge = rotl64($Agu, 20);
        $Aka ^= $Da;
        $Bgi = rotl64($Aka, 3);
        $Ame ^= $De;
        $Bgo = rotl64($Ame, 45);
        $Asi ^= $Di;
        $Bgu = rotl64($Asi, 61);
        $Ega = $Bga ^ ((!$Bge) & $Bgi);
        $Ege = $Bge ^ ((!$Bgi) & $Bgo);
        $Egi = $Bgi ^ ((!$Bgo) & $Bgu);
        $Ego = $Bgo ^ ((!$Bgu) & $Bga);
        $Egu = $Bgu ^ ((!$Bga) & $Bge);

        $Abe ^= $De;
        $Bka = rotl64($Abe, 1);
        $Agi ^= $Di;
        $Bke = rotl64($Agi, 6);
        $Ako ^= $Do;
        $Bki = rotl64($Ako, 25);
        $Amu ^= $Du;
        $Bko = rotl64($Amu, 8);
        $Asa ^= $Da;
        $Bku = rotl64($Asa, 18);
        $Eka = $Bka ^ ((!$Bke) & $Bki);
        $Eke = $Bke ^ ((!$Bki) & $Bko);
        $Eki = $Bki ^ ((!$Bko) & $Bku);
        $Eko = $Bko ^ ((!$Bku) & $Bka);
        $Eku = $Bku ^ ((!$Bka) & $Bke);

        $Abu ^= $Du;
        $Bma = rotl64($Abu, 27);
        $Aga ^= $Da;
        $Bme = rotl64($Aga, 36);
        $Ake ^= $De;
        $Bmi = rotl64($Ake, 10);
        $Ami ^= $Di;
        $Bmo = rotl64($Ami, 15);
        $Aso ^= $Do;
        $Bmu = rotl64($Aso, 56);
        $Ema = $Bma ^ ((!$Bme) & $Bmi);
        $Eme = $Bme ^ ((!$Bmi) & $Bmo);
        $Emi = $Bmi ^ ((!$Bmo) & $Bmu);
        $Emo = $Bmo ^ ((!$Bmu) & $Bma);
        $Emu = $Bmu ^ ((!$Bma) & $Bme);

        $Abi ^= $Di;
        $Bsa = rotl64($Abi, 62);
        $Ago ^= $Do;
        $Bse = rotl64($Ago, 55);
        $Aku ^= $Du;
        $Bsi = rotl64($Aku, 39);
        $Ama ^= $Da;
        $Bso = rotl64($Ama, 41);
        $Ase ^= $De;
        $Bsu = rotl64($Ase, 2);
        $Esa = $Bsa ^ ((!$Bse) & $Bsi);
        $Ese = $Bse ^ ((!$Bsi) & $Bso);
        $Esi = $Bsi ^ ((!$Bso) & $Bsu);
        $Eso = $Bso ^ ((!$Bsu) & $Bsa);
        $Esu = $Bsu ^ ((!$Bsa) & $Bse);
    };
}

unsafe fn keccakf_24_rounds(st: &mut [u64; 25]) {
    let state = &*st;

    let (mut Aba, mut Abe, mut Abi, mut Abo, mut Abu): (u64, u64, u64, u64, u64);
    let (mut Aga, mut Age, mut Agi, mut Ago, mut Agu): (u64, u64, u64, u64, u64);
    let (mut Aka, mut Ake, mut Aki, mut Ako, mut Aku): (u64, u64, u64, u64, u64);
    let (mut Ama, mut Ame, mut Ami, mut Amo, mut Amu): (u64, u64, u64, u64, u64);
    let (mut Asa, mut Ase, mut Asi, mut Aso, mut Asu): (u64, u64, u64, u64, u64);
    let (mut Bba, mut Bbe, mut Bbi, mut Bbo, mut Bbu): (u64, u64, u64, u64, u64);
    let (mut Bga, mut Bge, mut Bgi, mut Bgo, mut Bgu): (u64, u64, u64, u64, u64);
    let (mut Bka, mut Bke, mut Bki, mut Bko, mut Bku): (u64, u64, u64, u64, u64);
    let (mut Bma, mut Bme, mut Bmi, mut Bmo, mut Bmu): (u64, u64, u64, u64, u64);
    let (mut Bsa, mut Bse, mut Bsi, mut Bso, mut Bsu): (u64, u64, u64, u64, u64);
    let (mut Ca, mut Ce, mut Ci, mut Co, mut Cu): (u64, u64, u64, u64, u64);
    let (mut Da, mut De, mut Di, mut Do, mut Du): (u64, u64, u64, u64, u64);
    let (mut Eba, mut Ebe, mut Ebi, mut Ebo, mut Ebu): (u64, u64, u64, u64, u64);
    let (mut Ega, mut Ege, mut Egi, mut Ego, mut Egu): (u64, u64, u64, u64, u64);
    let (mut Eka, mut Eke, mut Eki, mut Eko, mut Eku): (u64, u64, u64, u64, u64);
    let (mut Ema, mut Eme, mut Emi, mut Emo, mut Emu): (u64, u64, u64, u64, u64);
    let (mut Esa, mut Ese, mut Esi, mut Eso, mut Esu): (u64, u64, u64, u64, u64);

    // KECCAK_COPY_FROM_STATE(A, state)
    Aba = state[0];
    Abe = state[1];
    Abi = state[2];
    Abo = state[3];
    Abu = state[4];
    Aga = state[5];
    Age = state[6];
    Agi = state[7];
    Ago = state[8];
    Agu = state[9];
    Aka = state[10];
    Ake = state[11];
    Aki = state[12];
    Ako = state[13];
    Aku = state[14];
    Ama = state[15];
    Ame = state[16];
    Ami = state[17];
    Amo = state[18];
    Amu = state[19];
    Asa = state[20];
    Ase = state[21];
    Asi = state[22];
    Aso = state[23];
    Asu = state[24];

    keccak_prepare_theta!(
        Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami,
        Amo, Amu, Asa, Ase, Asi, Aso, Asu, Ca, Ce, Ci, Co, Cu
    );

    macro_rules! r_pre_ae {
        ($idx:expr) => {
            keccak_round_pre!(
                $idx, Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku,
                Ama, Ame, Ami, Amo, Amu, Asa, Ase, Asi, Aso, Asu, Eba, Ebe, Ebi, Ebo, Ebu, Ega,
                Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku, Ema, Eme, Emi, Emo, Emu, Esa, Ese,
                Esi, Eso, Esu, Bba, Bbe, Bbi, Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki,
                Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa, Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da,
                De, Di, Do, Du
            );
        };
    }
    macro_rules! r_pre_ea {
        ($idx:expr) => {
            keccak_round_pre!(
                $idx, Eba, Ebe, Ebi, Ebo, Ebu, Ega, Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku,
                Ema, Eme, Emi, Emo, Emu, Esa, Ese, Esi, Eso, Esu, Aba, Abe, Abi, Abo, Abu, Aga,
                Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami, Amo, Amu, Asa, Ase,
                Asi, Aso, Asu, Bba, Bbe, Bbi, Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki,
                Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa, Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da,
                De, Di, Do, Du
            );
        };
    }

    r_pre_ae!(0);
    r_pre_ea!(1);
    r_pre_ae!(2);
    r_pre_ea!(3);
    r_pre_ae!(4);
    r_pre_ea!(5);
    r_pre_ae!(6);
    r_pre_ea!(7);
    r_pre_ae!(8);
    r_pre_ea!(9);
    r_pre_ae!(10);
    r_pre_ea!(11);
    r_pre_ae!(12);
    r_pre_ea!(13);
    r_pre_ae!(14);
    r_pre_ea!(15);
    r_pre_ae!(16);
    r_pre_ea!(17);
    r_pre_ae!(18);
    r_pre_ea!(19);
    r_pre_ae!(20);
    r_pre_ea!(21);
    r_pre_ae!(22);

    // KECCAK_THETA_RHO_PI_CHI_IOTA(23, E, A)
    keccak_round!(
        23, Eba, Ebe, Ebi, Ebo, Ebu, Ega, Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku, Ema, Eme,
        Emi, Emo, Emu, Esa, Ese, Esi, Eso, Esu, Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu,
        Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami, Amo, Amu, Asa, Ase, Asi, Aso, Asu, Bba, Bbe, Bbi,
        Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki, Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa,
        Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da, De, Di, Do, Du
    );

    // KECCAK_COPY_TO_STATE(state, A)
    st[0] = Aba;
    st[1] = Abe;
    st[2] = Abi;
    st[3] = Abo;
    st[4] = Abu;
    st[5] = Aga;
    st[6] = Age;
    st[7] = Agi;
    st[8] = Ago;
    st[9] = Agu;
    st[10] = Aka;
    st[11] = Ake;
    st[12] = Aki;
    st[13] = Ako;
    st[14] = Aku;
    st[15] = Ama;
    st[16] = Ame;
    st[17] = Ami;
    st[18] = Amo;
    st[19] = Amu;
    st[20] = Asa;
    st[21] = Ase;
    st[22] = Asi;
    st[23] = Aso;
    st[24] = Asu;
}

unsafe fn keccakf_12_rounds(st: &mut [u64; 25]) {
    let state = &*st;

    let (mut Aba, mut Abe, mut Abi, mut Abo, mut Abu): (u64, u64, u64, u64, u64);
    let (mut Aga, mut Age, mut Agi, mut Ago, mut Agu): (u64, u64, u64, u64, u64);
    let (mut Aka, mut Ake, mut Aki, mut Ako, mut Aku): (u64, u64, u64, u64, u64);
    let (mut Ama, mut Ame, mut Ami, mut Amo, mut Amu): (u64, u64, u64, u64, u64);
    let (mut Asa, mut Ase, mut Asi, mut Aso, mut Asu): (u64, u64, u64, u64, u64);
    let (mut Bba, mut Bbe, mut Bbi, mut Bbo, mut Bbu): (u64, u64, u64, u64, u64);
    let (mut Bga, mut Bge, mut Bgi, mut Bgo, mut Bgu): (u64, u64, u64, u64, u64);
    let (mut Bka, mut Bke, mut Bki, mut Bko, mut Bku): (u64, u64, u64, u64, u64);
    let (mut Bma, mut Bme, mut Bmi, mut Bmo, mut Bmu): (u64, u64, u64, u64, u64);
    let (mut Bsa, mut Bse, mut Bsi, mut Bso, mut Bsu): (u64, u64, u64, u64, u64);
    let (mut Ca, mut Ce, mut Ci, mut Co, mut Cu): (u64, u64, u64, u64, u64);
    let (mut Da, mut De, mut Di, mut Do, mut Du): (u64, u64, u64, u64, u64);
    let (mut Eba, mut Ebe, mut Ebi, mut Ebo, mut Ebu): (u64, u64, u64, u64, u64);
    let (mut Ega, mut Ege, mut Egi, mut Ego, mut Egu): (u64, u64, u64, u64, u64);
    let (mut Eka, mut Eke, mut Eki, mut Eko, mut Eku): (u64, u64, u64, u64, u64);
    let (mut Ema, mut Eme, mut Emi, mut Emo, mut Emu): (u64, u64, u64, u64, u64);
    let (mut Esa, mut Ese, mut Esi, mut Eso, mut Esu): (u64, u64, u64, u64, u64);

    Aba = state[0];
    Abe = state[1];
    Abi = state[2];
    Abo = state[3];
    Abu = state[4];
    Aga = state[5];
    Age = state[6];
    Agi = state[7];
    Ago = state[8];
    Agu = state[9];
    Aka = state[10];
    Ake = state[11];
    Aki = state[12];
    Ako = state[13];
    Aku = state[14];
    Ama = state[15];
    Ame = state[16];
    Ami = state[17];
    Amo = state[18];
    Amu = state[19];
    Asa = state[20];
    Ase = state[21];
    Asi = state[22];
    Aso = state[23];
    Asu = state[24];

    keccak_prepare_theta!(
        Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami,
        Amo, Amu, Asa, Ase, Asi, Aso, Asu, Ca, Ce, Ci, Co, Cu
    );

    macro_rules! r_pre_ae {
        ($idx:expr) => {
            keccak_round_pre!(
                $idx, Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku,
                Ama, Ame, Ami, Amo, Amu, Asa, Ase, Asi, Aso, Asu, Eba, Ebe, Ebi, Ebo, Ebu, Ega,
                Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku, Ema, Eme, Emi, Emo, Emu, Esa, Ese,
                Esi, Eso, Esu, Bba, Bbe, Bbi, Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki,
                Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa, Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da,
                De, Di, Do, Du
            );
        };
    }
    macro_rules! r_pre_ea {
        ($idx:expr) => {
            keccak_round_pre!(
                $idx, Eba, Ebe, Ebi, Ebo, Ebu, Ega, Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku,
                Ema, Eme, Emi, Emo, Emu, Esa, Ese, Esi, Eso, Esu, Aba, Abe, Abi, Abo, Abu, Aga,
                Age, Agi, Ago, Agu, Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami, Amo, Amu, Asa, Ase,
                Asi, Aso, Asu, Bba, Bbe, Bbi, Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki,
                Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa, Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da,
                De, Di, Do, Du
            );
        };
    }

    r_pre_ae!(12);
    r_pre_ea!(13);
    r_pre_ae!(14);
    r_pre_ea!(15);
    r_pre_ae!(16);
    r_pre_ea!(17);
    r_pre_ae!(18);
    r_pre_ea!(19);
    r_pre_ae!(20);
    r_pre_ea!(21);
    r_pre_ae!(22);

    keccak_round!(
        23, Eba, Ebe, Ebi, Ebo, Ebu, Ega, Ege, Egi, Ego, Egu, Eka, Eke, Eki, Eko, Eku, Ema, Eme,
        Emi, Emo, Emu, Esa, Ese, Esi, Eso, Esu, Aba, Abe, Abi, Abo, Abu, Aga, Age, Agi, Ago, Agu,
        Aka, Ake, Aki, Ako, Aku, Ama, Ame, Ami, Amo, Amu, Asa, Ase, Asi, Aso, Asu, Bba, Bbe, Bbi,
        Bbo, Bbu, Bga, Bge, Bgi, Bgo, Bgu, Bka, Bke, Bki, Bko, Bku, Bma, Bme, Bmi, Bmo, Bmu, Bsa,
        Bse, Bsi, Bso, Bsu, Ca, Ce, Ci, Co, Cu, Da, De, Di, Do, Du
    );

    st[0] = Aba;
    st[1] = Abe;
    st[2] = Abi;
    st[3] = Abo;
    st[4] = Abu;
    st[5] = Aga;
    st[6] = Age;
    st[7] = Agi;
    st[8] = Ago;
    st[9] = Agu;
    st[10] = Aka;
    st[11] = Ake;
    st[12] = Aki;
    st[13] = Ako;
    st[14] = Aku;
    st[15] = Ama;
    st[16] = Ame;
    st[17] = Ami;
    st[18] = Amo;
    st[19] = Amu;
    st[20] = Asa;
    st[21] = Ase;
    st[22] = Asi;
    st[23] = Aso;
    st[24] = Asu;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_24(state: *mut c_void) {
    let mut st: [u64; 25] = [0u64; 25];
    let mut i: u32 = 0;
    while i < 25 {
        st[i as usize] = load64_le((state as *const u8).add((i as usize) * 8));
        i += 1;
    }

    keccakf_24_rounds(&mut st);

    let mut i: u32 = 0;
    while i < 25 {
        store64_le((state as *mut u8).add((i as usize) * 8), st[i as usize]);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_permute_12(state: *mut c_void) {
    let mut st: [u64; 25] = [0u64; 25];
    let mut i: u32 = 0;
    while i < 25 {
        st[i as usize] = load64_le((state as *const u8).add((i as usize) * 8));
        i += 1;
    }

    keccakf_12_rounds(&mut st);

    let mut i: u32 = 0;
    while i < 25 {
        store64_le((state as *mut u8).add((i as usize) * 8), st[i as usize]);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_keccak1600_ref_init(state: *mut c_void) {
    core::ptr::write_bytes(state as *mut u8, 0, KECCAK1600_STATEBYTES);
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
    state: *const c_void,
    data: *mut u8,
    offset: usize,
    length: usize,
) {
    let st = state as *const u8;
    core::ptr::copy_nonoverlapping(st.add(offset), data, length);
}
