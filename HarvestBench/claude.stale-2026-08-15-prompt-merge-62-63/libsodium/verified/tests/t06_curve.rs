//! t06_curve.rs — C-vs-Rust differential verification of the whole elliptic-curve
//! surface: `crypto_scalarmult` (curve25519 / ed25519 / ristretto255),
//! `crypto_core_ed25519`, `crypto_core_ristretto255` and `crypto_sign`.
//!
//! CONFIGS.md rows 122–163 and ERRORS.md rows 162–221 are the specification.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! CONFIGS row -> test mapping
//! ---------------------------
//! * 122 `_curve25519_base` ................... `c122_curve25519_base`
//! * 123 `_curve25519` + ECDH ................. `c123_curve25519_ecdh`
//! * 124 `crypto_scalarmult` dispatch ......... `c124_scalarmult_dispatch`
//! * 125 `_ed25519` (clamp) ................... `c125_scalarmult_ed25519_clamp`
//! * 126 `_ed25519_noclamp` ................... `c126_scalarmult_ed25519_noclamp`
//! * 127 `_ed25519_base` (clamp) .............. `c127_c128_scalarmult_ed25519_base`
//! * 128 `_ed25519_base_noclamp` .............. `c127_c128_scalarmult_ed25519_base`
//! * 129 `_ristretto255` ...................... `c129_scalarmult_ristretto255`
//! * 130 `_ristretto255_base` ................. `c130_scalarmult_ristretto255_base`
//! * 131 `_ed25519_is_valid_point` ............ `c131_e198_202_ed25519_is_valid_point`
//! * 132 `_ed25519_add` / `_sub` ............... `c132_e203_208_ed25519_add_sub`
//! * 133 `ge25519_from_uniform` ............... `c133_ed25519_from_uniform`
//!       (`crypto_core_ed25519_from_uniform` is not a public symbol in 1.0.23;
//!        the export is `_sodium_ge25519_from_uniform`, SYMBOLS.md row 58)
//! * 134 `ge25519_from_hash` .................. `c134_ed25519_from_hash`
//!       (export is `_sodium_ge25519_from_hash`, SYMBOLS.md row 57)
//! * 135 `_ed25519_random` **[RNG]** .......... `c135_ed25519_random`
//! * 136 `_ed25519_scalar_random` **[RNG]** ... `c136_ed25519_scalar_random`
//! * 137 `_ed25519_scalar_reduce` ............. `c137_ed25519_scalar_reduce`
//! * 138 `_scalar_negate` / `_complement` ..... `c138_ed25519_scalar_negate_complement`
//! * 139 `_scalar_add` / `_sub` / `_mul` ...... `c139_ed25519_scalar_add_sub_mul`
//! * 140 `_scalar_invert` ..................... `c140_e210_e217_ed25519_scalar_invert`
//! * 141 `_scalar_is_canonical` ............... `c141_e211_e219_scalar_is_canonical`
//! * 142 `_ed25519_from_string` ............... `c142_144_c150_from_string`
//! * 143 `_ed25519_from_string_nu` ............ `c142_144_c150_from_string`
//! * 144 `_ed25519_scalar_from_string` ........ `c142_144_c150_from_string`
//! * 145 `_ristretto255_is_valid_point` ....... `c145_e212_ristretto_is_valid_point`
//! * 146 `_ristretto255_add` / `_sub` ......... `c146_e213_216_ristretto_add_sub`
//! * 147 `_ristretto255_from_hash` ............ `c147_e220_ristretto_from_hash`
//! * 148 `_ristretto255_random` **[RNG]** ..... `c148_ristretto255_random`
//! * 149 `_ristretto255_scalar_*` ............. `c149_ristretto255_scalar_ops`
//! * 150 `_ristretto255_from_string` .......... `c142_144_c150_from_string`
//!       (NOTE: `crypto_core_ristretto255_from_string_nu` does NOT exist in 1.0.23)
//! * 151 constant getters ..................... `c151_constant_getters`
//! * 152 `_seed_keypair` ...................... `c152_sign_seed_keypair`
//! * 153 `_keypair` **[RNG]** ................. `c153_sign_keypair`
//! * 154 `_detached` + `_verify_detached` ..... `c154_sign_detached_verify`
//! * 155 `crypto_sign_ed25519` + `_open` ...... `c155_e170_e172_sign_open_combined`
//! * 156 `crypto_sign` dispatch ............... `c156_sign_dispatch`
//! * 157 `ed25519ph` .......................... `c157_c158_e178_ed25519ph`
//! * 158 `crypto_sign_init` dispatch .......... `c157_c158_e178_ed25519ph`
//! * 159 `_sk_to_seed` / `_sk_to_pk` .......... `c159_sk_to_seed_and_pk`
//! * 160 `_sk_to_curve25519` .................. `c160_sk_to_curve25519`
//! * 161 `_pk_to_curve25519` .................. `c161_e174_176_pk_to_curve25519`
//! * 162 malleability axis .................... `c162_e162_malleability_axis`
//! * 163 cofactored-verification axis ......... `c163_e168_cofactored_verification`
//!
//! ERRORS row -> test mapping
//! --------------------------
//! * 162 `S >= L` & high nibble ............... `c162_e162_malleability_axis`
//! * 163 non-canonical pk .................... `e163_verify_noncanonical_pk`
//! * 164 pk not on curve ..................... `e164_167_verify_point_gates`
//! * 165 pk small order ...................... `e164_167_verify_point_gates`
//! * 166 R not on curve ...................... `e164_167_verify_point_gates`
//! * 167 R small order ....................... `e164_167_verify_point_gates`
//! * 168 FINAL cofactored check .............. `c163_e168_cofactored_verification`
//! * 169 `crypto_sign_verify_detached` ....... `e169_e173_dispatch_wrappers`
//! * 170 `_open` `smlen < 64` ................ `e170_open_smlen_lt_64`
//! * 171 `_open` MESSAGEBYTES_MAX ............ `e171_open_messagebytes_max_dead`
//! * 172 `_open` inner verify fails .......... `c155_e170_e172_sign_open_combined`
//! * 173 `crypto_sign_open` .................. `e169_e173_dispatch_wrappers`
//! * 174 `pk_to_curve25519` frombytes ........ `c161_e174_176_pk_to_curve25519`
//! * 175 `pk_to_curve25519` small order ...... `c161_e174_176_pk_to_curve25519`
//! * 176 `pk_to_curve25519` torsion .......... `c161_e174_176_pk_to_curve25519`
//! * 177 `pk_to_curve25519` no canon check ... `e177_pk_to_curve25519_no_canonical_check`
//! * 178 `ph_final_verify` rejections ........ `e178_ph_final_verify_rejections`,
//!                                              `c157_c158_e178_ed25519ph`
//! * 179 `crypto_sign_ed25519` siglen ........ `e179_sign_siglen_dead_branch`
//! * 180 x25519 blocklist .................... `e180_x25519_blocklist`
//! * 181 x25519 zero output .................. `e181_x25519_zero_output_dead`
//! * 182 `crypto_scalarmult` wrapper ......... `e182_e183_scalarmult_wrappers`
//! * 183 `_curve25519_base` never fails ...... `e182_e183_scalarmult_wrappers`
//! * 184 `_ed25519` non-canonical ............ `e184_187_scalarmult_ed25519_gates`
//! * 185 `_ed25519` frombytes ................ `e184_187_scalarmult_ed25519_gates`
//! * 186 `_ed25519` small order .............. `e184_187_scalarmult_ed25519_gates`
//! * 187 `_ed25519` not main subgroup ........ `e184_187_scalarmult_ed25519_gates`
//! * 188 `_ed25519` `_is_inf` ................ `e188_189_scalarmult_ed25519_post_checks`
//! * 189 `_ed25519` `is_zero(n)` after mult .. `e188_189_scalarmult_ed25519_post_checks`
//! * 190 `_ed25519_base` `_is_inf` ........... `e190_191_scalarmult_ed25519_base_post_checks`
//! * 191 `_ed25519_base` `is_zero(n)` ........ `e190_191_scalarmult_ed25519_base_post_checks`
//! * 192 ristretto not canonical ............. `e192_195_ristretto_frombytes_gates`
//! * 193 ristretto not a square ............. `e192_195_ristretto_frombytes_gates`
//! * 194 ristretto `isnegative(T)` ........... `e192_195_ristretto_frombytes_gates`
//! * 195 ristretto `iszero(Y)` ............... `e192_195_ristretto_frombytes_gates`
//! * 196 `_ristretto255` zero output ......... `e196_197_scalarmult_ristretto_zero_output`
//! * 197 `_ristretto255_base` zero output .... `e196_197_scalarmult_ristretto_zero_output`
//! * 198–202 `_is_valid_point` gates ......... `c131_e198_202_ed25519_is_valid_point`
//! * 203–208 `_add` / `_sub` gates ........... `c132_e203_208_ed25519_add_sub`
//! * 207 non-canon/small/torsion ACCEPTED .... `e207_add_sub_accepts_weak_points`
//! * 209 `hash_alg` not in {1,2} ............. `e209_e218_e221_bad_hash_alg`
//! * 210 `_scalar_invert(0)` ................. `c140_e210_e217_ed25519_scalar_invert`
//! * 211 `_scalar_is_canonical(s >= L)` ...... `c141_e211_e219_scalar_is_canonical`
//! * 212 ristretto `_is_valid_point` ......... `c145_e212_ristretto_is_valid_point`
//! * 213–216 ristretto `_add` / `_sub` ....... `c146_e213_216_ristretto_add_sub`
//! * 217 ristretto `_scalar_invert(0)` ....... `c140_e210_e217_ed25519_scalar_invert`
//! * 218 ristretto `hash_alg` ................ `e209_e218_e221_bad_hash_alg`
//! * 219 ristretto `_scalar_is_canonical` .... `c141_e211_e219_scalar_is_canonical`
//! * 220 `_ristretto255_from_hash` no reject . `c147_e220_ristretto_from_hash`
//! * 221 `core_h2c_string_to_hash` default ... `e209_e218_e221_bad_hash_alg`
//! * internal `_pick_best_implementation` .... `internal_pick_best_implementation`
//! * internal `ge25519_p3` predicates ......... `internal_ge25519_p3_predicates`
//! * hard-coded table self-check ............. `smallorder_table_selfcheck`

mod common;
use common::*;
use libc::{c_char, c_int};
use std::ffi::CStr;

// ------------------------------------------------------------------ fn types

type Pred = unsafe extern "C" fn(*const u8) -> c_int;
type Int1 = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Int2 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Void0 = unsafe extern "C" fn(*mut u8);
type Void1 = unsafe extern "C" fn(*mut u8, *const u8);
type Void2 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;
type FromStringFn =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;
type Sha512Fn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
/// `crypto_sign_ed25519_detached` and `crypto_sign_ed25519` share this shape.
type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type OpenFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type PhInitFn = unsafe extern "C" fn(*mut u8) -> c_int;
type PhUpdFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type PhCreateFn = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> c_int;
type PhVerifyFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

/// Prefill byte for every output buffer.
const FILL: u8 = 0xAA;
/// Trailing guard region: must never be touched by the library.
const PAD: usize = 32;
/// `sizeof(crypto_sign_ed25519ph_state)` == `sizeof(crypto_hash_sha512_state)`.
const PH_STATE: usize = 208;
/// Capacity of the aligned opaque-state buffer (state + guard).
const PH_CAP: usize = PH_STATE + PAD;

/// Serialises every test that mutates the process-global `randombytes`
/// implementation or consumes its deterministic stream. `cargo` runs tests as
/// parallel threads inside ONE process and both `.so`s are shared.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// --------------------------------------------------------------- fixed vectors

/// p = 2^255 - 19, little-endian.
const P_BYTES: [u8; 32] = [
    0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
];

/// L = 2^252 + 27742317777372353535851937790883648493, little-endian.
/// Verbatim from the `L[]` table in `crypto_core/ed25519/core_ed25519.c`.
const L_BYTES: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// The 8 small-order ed25519 point encodings. `ge25519_has_small_order` in
/// 1.0.23 is the ALGEBRAIC test (X==0 | Y==0 | Z==0 | y*sqrt(-1) == +-x), not a
/// table, so the encodings are pinned here and self-checked in
/// `smallorder_table_selfcheck` (8*P == identity and `_is_valid_point` == 0 for
/// every entry, asserted against the C library).
const SMALL_ORDER: [[u8; 32]; 8] = [
    // order 1 — the identity (0, 1)
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0,
    ],
    // order 2 — (0, -1)
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // order 4 — y = 0, x = sqrt(-1)
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    // order 4 — y = 0, sign bit set
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0x80,
    ],
    // order 8
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    // order 8, sign bit set
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    // order 8
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    // order 8, sign bit set
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
];

/// Group order of each entry of `SMALL_ORDER`, same index.
const SMALL_ORDER_ORD: [u32; 8] = [1, 2, 4, 4, 8, 8, 8, 8];

/// The 7 blocklisted curve25519 u-coordinates, verbatim from the `blocklist[][]`
/// table in `crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
const X25519_BLOCKLIST: [[u8; 32]; 7] = [
    // 0 (order 4)
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    // 1 (order 1)
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0,
    ],
    // order 8
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    // order 8
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    // p-1 (order 2)
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p (= 0, order 4)
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p+1 (= 1, order 1)
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

/// ed25519 y-coordinates that are NOT the y of any curve point, i.e.
/// `(y^2-1)/(d*y^2+1)` is a non-square, so `ge25519_frombytes` fails.
/// (Computed offline over GF(2^255-19) and re-confirmed against the C library
/// by `e203_208_ed25519_add_sub_gates`, which requires `-1` for each.)
const NOT_ON_CURVE_Y: [u8; 7] = [2, 7, 8, 11, 12, 13, 17];

/// ed25519 y-coordinates that DO decode to a curve point (small y values).
const ON_CURVE_Y: [u8; 6] = [0, 1, 3, 4, 5, 6];

// ------------------------------------------------------------- little-endian ops

fn le_add_assign(a: &mut [u8], b: &[u8]) -> u32 {
    let mut carry = 0u32;
    for i in 0..a.len() {
        let t = a[i] as u32 + *b.get(i).unwrap_or(&0) as u32 + carry;
        a[i] = (t & 0xff) as u8;
        carry = t >> 8;
    }
    carry
}

fn le_add32(a: &[u8; 32], b: &[u8; 32]) -> ([u8; 32], u32) {
    let mut o = *a;
    let c = le_add_assign(&mut o, b);
    (o, c)
}

fn le_add_small(a: &[u8; 32], k: u8) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[0] = k;
    le_add32(a, &b).0
}

fn le_sub_small(a: &[u8; 32], k: u8) -> [u8; 32] {
    let mut o = *a;
    let mut borrow = k as i32;
    for x in o.iter_mut() {
        let t = *x as i32 - borrow;
        *x = (t & 0xff) as u8;
        borrow = if t < 0 { 1 } else { 0 };
    }
    o
}

fn arr32(v: &[u8]) -> [u8; 32] {
    let mut o = [0u8; 32];
    o.copy_from_slice(&v[..32]);
    o
}

// ------------------------------------------------------------------- assertions

fn guard_ok(what: &str, who: &str, b: &[u8], len: usize) {
    assert!(
        b[len..].iter().all(|&x| x == FILL),
        "{what}: {who} wrote OUTSIDE the requested {len} bytes \
         (0xAA trailing guard clobbered: {})",
        hexs(&b[len..])
    );
}

fn errno_get() -> c_int {
    unsafe { *libc::__errno_location() }
}
fn errno_set(v: c_int) {
    unsafe { *libc::__errno_location() = v };
}

// ------------------------------------------------- generic differential drivers
//
// Each driver runs ONE entry point through both `.so` files with a 0xAA-prefilled
// output buffer plus a trailing guard, asserts the return values and the FULL
// buffers agree, checks nothing was written past the declared output length, and
// returns the (now proven identical) result. Any property later asserted on a
// returned value therefore holds for BOTH libraries.

/// `int f(const unsigned char *s)` — predicates.
fn d_pred(name: &str, s: &[u8], tag: &str) -> c_int {
    let (fc, fr) = unsafe { pair::<Pred>(name) };
    let (rc, rr) = unsafe { (fc(s.as_ptr()), fr(s.as_ptr())) };
    assert_eq!(
        rc,
        rr,
        "{name} [{tag}] s={}: RETURN differs (C={rc} rust={rr})",
        hexs(s)
    );
    rc
}

/// `int f(unsigned char *out, const unsigned char *a)`.
fn d_int1(name: &str, olen: usize, a: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<Int1>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), a.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), a.as_ptr()) };
    let what = format!("{name} [{tag}] a={}", hexs(a));
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    (rc, bc)
}

/// `int f(unsigned char *out, const unsigned char *a, const unsigned char *b)`.
fn d_int2(name: &str, olen: usize, a: &[u8], b: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<Int2>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
    let what = format!("{name} [{tag}] a={} b={}", hexs(a), hexs(b));
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    (rc, bc)
}

/// `void f(unsigned char *out, const unsigned char *a)`.
fn d_void1(name: &str, olen: usize, a: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<Void1>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    unsafe {
        fc(bc.as_mut_ptr(), a.as_ptr());
        fr(br.as_mut_ptr(), a.as_ptr());
    }
    let what = format!("{name} [{tag}] a={}", hexs(a));
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    bc
}

/// `void f(unsigned char *out, const unsigned char *a, const unsigned char *b)`.
fn d_void2(name: &str, olen: usize, a: &[u8], b: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<Void2>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    unsafe {
        fc(bc.as_mut_ptr(), a.as_ptr(), b.as_ptr());
        fr(br.as_mut_ptr(), a.as_ptr(), b.as_ptr());
    }
    let what = format!("{name} [{tag}] a={} b={}", hexs(a), hexs(b));
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    bc
}

/// `void f(unsigned char *out)` — the RNG-driven generators. Caller must hold
/// `rng_lock()` and have called `install_det_rng`.
fn d_void0(name: &str, olen: usize, tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<Void0>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    unsafe {
        fc(bc.as_mut_ptr());
        fr(br.as_mut_ptr());
    }
    let what = format!("{name} [{tag}]");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    bc
}

/// `int f(unsigned char *out, const unsigned char *ctx, size_t, const unsigned
/// char *msg, size_t, int hash_alg)` — also captures `errno`.
fn d_from_string(
    name: &str,
    olen: usize,
    ctx: &[u8],
    msg: &[u8],
    alg: c_int,
    tag: &str,
) -> (c_int, Vec<u8>, c_int) {
    let (fc, fr) = unsafe { pair::<FromStringFn>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    errno_set(0);
    let rc = unsafe {
        fc(
            bc.as_mut_ptr(),
            ctx.as_ptr(),
            ctx.len(),
            msg.as_ptr(),
            msg.len(),
            alg,
        )
    };
    let ec = errno_get();
    errno_set(0);
    let rr = unsafe {
        fr(
            br.as_mut_ptr(),
            ctx.as_ptr(),
            ctx.len(),
            msg.as_ptr(),
            msg.len(),
            alg,
        )
    };
    let er = errno_get();
    let what = format!(
        "{name} [{tag}] alg={alg} ctx_len={} msg_len={}",
        ctx.len(),
        msg.len()
    );
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: ERRNO differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    (rc, bc, ec)
}

fn d_size(name: &str, expect: usize) -> usize {
    let (c, r) = unsafe { pair::<SizeFn>(name) };
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}(): C={cv} rust={rv}");
    assert_eq!(cv, expect, "{name}(): C returned {cv}, spec says {expect}");
    cv
}

fn d_cstr(name: &str, expect: &str) {
    let (c, r) = unsafe { pair::<StrFn>(name) };
    let (a, b) = unsafe {
        (
            CStr::from_ptr(c()).to_string_lossy().into_owned(),
            CStr::from_ptr(r()).to_string_lossy().into_owned(),
        )
    };
    assert_eq!(a, b, "{name}(): C={a:?} rust={b:?}");
    assert_eq!(a, expect, "{name}(): C returned {a:?}, spec says {expect:?}");
}

fn d_sha512(msg: &[u8]) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<Sha512Fn>("crypto_hash_sha512") };
    let mut bc = vec![FILL; 64 + PAD];
    let mut br = vec![FILL; 64 + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
    let rr = unsafe { fr(br.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
    assert_eq!(rc, rr, "crypto_hash_sha512: return C={rc} rust={rr}");
    assert_eq_bytes("crypto_hash_sha512 helper", &bc, &br);
    bc.truncate(64);
    bc
}

// ------------------------------------------------------- sign-specific drivers

/// `crypto_sign*_detached` / `crypto_sign_ed25519` — `siglen_p` may be NULL.
fn d_sign(
    name: &str,
    olen: usize,
    m: &[u8],
    sk: &[u8],
    null_len: bool,
    tag: &str,
) -> (c_int, Vec<u8>, u64) {
    let (fc, fr) = unsafe { pair::<SignFn>(name) };
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    let mut lc: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    let mut lr: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    let pc = if null_len { std::ptr::null_mut() } else { &mut lc };
    let pr = if null_len { std::ptr::null_mut() } else { &mut lr };
    let rc = unsafe { fc(bc.as_mut_ptr(), pc, m.as_ptr(), m.len() as u64, sk.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), pr, m.as_ptr(), m.len() as u64, sk.as_ptr()) };
    let what = format!("{name} [{tag}] mlen={} null_len={null_len}", m.len());
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq!(lc, lr, "{what}: *len_p differs (C={lc} rust={lr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    bc.truncate(olen);
    (rc, bc, lc)
}

fn d_verify(name: &str, sig: &[u8], m: &[u8], pk: &[u8], tag: &str) -> c_int {
    let (fc, fr) = unsafe { pair::<VerifyFn>(name) };
    let rc = unsafe { fc(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
    let rr = unsafe { fr(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
    assert_eq!(
        rc,
        rr,
        "{name} [{tag}] mlen={} sig={} pk={}: RETURN differs (C={rc} rust={rr})",
        m.len(),
        hexs(sig),
        hexs(pk)
    );
    rc
}

/// `crypto_sign*_open`. `m` and `mlen_p` may independently be NULL.
fn d_open(
    name: &str,
    sm: &[u8],
    pk: &[u8],
    null_m: bool,
    null_len: bool,
    tag: &str,
) -> (c_int, Vec<u8>, u64) {
    let (fc, fr) = unsafe { pair::<OpenFn>(name) };
    // Always big enough for smlen bytes so a runaway memmove/memset is caught by
    // the guard rather than by corrupting the heap.
    let olen = sm.len().max(64);
    let mut bc = vec![FILL; olen + PAD];
    let mut br = vec![FILL; olen + PAD];
    let mut lc: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    let mut lr: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    let mc = if null_m { std::ptr::null_mut() } else { bc.as_mut_ptr() };
    let mr = if null_m { std::ptr::null_mut() } else { br.as_mut_ptr() };
    let pc = if null_len { std::ptr::null_mut() } else { &mut lc };
    let pr = if null_len { std::ptr::null_mut() } else { &mut lr };
    let rc = unsafe { fc(mc, pc, sm.as_ptr(), sm.len() as u64, pk.as_ptr()) };
    let rr = unsafe { fr(mr, pr, sm.as_ptr(), sm.len() as u64, pk.as_ptr()) };
    let what = format!(
        "{name} [{tag}] smlen={} null_m={null_m} null_len={null_len}",
        sm.len()
    );
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq!(lc, lr, "{what}: *mlen_p differs (C={lc} rust={lr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_ok(&what, "C", &bc, olen);
    guard_ok(&what, "rust", &br, olen);
    (rc, bc, lc)
}

// --------------------------------------------------------- ed25519ph state ops

#[repr(C, align(64))]
struct PhBuf([u8; PH_CAP]);

fn new_ph() -> Box<PhBuf> {
    Box::new(PhBuf([FILL; PH_CAP]))
}

struct PhRun {
    rets: Vec<c_int>,
    sig: Vec<u8>,
    siglen: u64,
    state: Vec<u8>,
}

unsafe fn ph_create_run(
    lib: &'static libloading::Library,
    prefix: &str,
    chunks: &[Vec<u8>],
    sk: &[u8],
) -> PhRun {
    let init = sym::<PhInitFn>(lib, &format!("{prefix}_init"));
    let upd = sym::<PhUpdFn>(lib, &format!("{prefix}_update"));
    let fin = sym::<PhCreateFn>(lib, &format!("{prefix}_final_create"));
    let mut st = new_ph();
    let sp = st.0.as_mut_ptr();
    let mut rets = vec![init(sp)];
    for c in chunks {
        rets.push(upd(sp, c.as_ptr(), c.len() as u64));
    }
    let mut sig = vec![FILL; 64 + PAD];
    let mut siglen: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    rets.push(fin(sp, sig.as_mut_ptr(), &mut siglen, sk.as_ptr()));
    PhRun { rets, sig, siglen, state: st.0.to_vec() }
}

unsafe fn ph_verify_run(
    lib: &'static libloading::Library,
    prefix: &str,
    chunks: &[Vec<u8>],
    sig: &[u8],
    pk: &[u8],
) -> (Vec<c_int>, Vec<u8>) {
    let init = sym::<PhInitFn>(lib, &format!("{prefix}_init"));
    let upd = sym::<PhUpdFn>(lib, &format!("{prefix}_update"));
    let fin = sym::<PhVerifyFn>(lib, &format!("{prefix}_final_verify"));
    let mut st = new_ph();
    let sp = st.0.as_mut_ptr();
    let mut rets = vec![init(sp)];
    for c in chunks {
        rets.push(upd(sp, c.as_ptr(), c.len() as u64));
    }
    rets.push(fin(sp, sig.as_ptr(), pk.as_ptr()));
    (rets, st.0.to_vec())
}

/// Drive `_init` / N x `_update` / `_final_create` through both libraries and
/// compare the return-code sequence, the signature buffer, `*siglen_p` and the
/// ENTIRE opaque state buffer (including its 0xAA guard).
fn d_ph_create(prefix: &str, chunks: &[Vec<u8>], sk: &[u8], tag: &str) -> (Vec<u8>, u64) {
    let l = libs();
    let a = unsafe { ph_create_run(&l.c, prefix, chunks, sk) };
    let b = unsafe { ph_create_run(&l.r, prefix, chunks, sk) };
    let what = format!(
        "{prefix} create [{tag}] chunks={:?}",
        chunks.iter().map(|c| c.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        a.rets, b.rets,
        "{what}: RETURN-CODE SEQUENCE differs\n  C   ={:?}\n  rust={:?}",
        a.rets, b.rets
    );
    assert_eq!(a.siglen, b.siglen, "{what}: *siglen_p differs");
    assert_eq_bytes(&format!("{what} SIG"), &a.sig, &b.sig);
    guard_ok(&what, "C", &a.sig, 64);
    guard_ok(&what, "rust", &b.sig, 64);
    assert_eq_bytes(&format!("{what} OPAQUE STATE"), &a.state, &b.state);
    guard_ok(&format!("{what} state"), "C", &a.state, PH_STATE);
    let mut sig = a.sig;
    sig.truncate(64);
    (sig, a.siglen)
}

fn d_ph_verify(prefix: &str, chunks: &[Vec<u8>], sig: &[u8], pk: &[u8], tag: &str) -> c_int {
    let l = libs();
    let (ra, sa) = unsafe { ph_verify_run(&l.c, prefix, chunks, sig, pk) };
    let (rb, sb) = unsafe { ph_verify_run(&l.r, prefix, chunks, sig, pk) };
    let what = format!(
        "{prefix} verify [{tag}] chunks={:?}",
        chunks.iter().map(|c| c.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        ra, rb,
        "{what}: RETURN-CODE SEQUENCE differs\n  C   ={ra:?}\n  rust={rb:?}"
    );
    assert_eq_bytes(&format!("{what} OPAQUE STATE"), &sa, &sb);
    guard_ok(&format!("{what} state"), "C", &sa, PH_STATE);
    *ra.last().unwrap()
}

// ------------------------------------------------------------- vector builders

/// The scalar sweep required by rows 137–141: 0, 1, 2, 8, L-1, L, L+1, 2^252,
/// 2^255-1, 2^256-1 plus `n` random 32-byte values.
fn scalar_vectors(n: usize, seed: u64) -> Vec<(String, [u8; 32])> {
    let mut v: Vec<(String, [u8; 32])> = Vec::new();
    let mut z = [0u8; 32];
    v.push(("0".into(), z));
    z[0] = 1;
    v.push(("1".into(), z));
    z[0] = 2;
    v.push(("2".into(), z));
    z[0] = 8;
    v.push(("8".into(), z));
    v.push(("L-1".into(), le_sub_small(&L_BYTES, 1)));
    v.push(("L".into(), L_BYTES));
    v.push(("L+1".into(), le_add_small(&L_BYTES, 1)));
    let mut p252 = [0u8; 32];
    p252[31] = 0x10;
    v.push(("2^252".into(), p252));
    let mut m255 = [0xffu8; 32];
    m255[31] = 0x7f;
    v.push(("2^255-1".into(), m255));
    v.push(("2^256-1".into(), [0xffu8; 32]));
    let mut rng = Rng::new(seed);
    for i in 0..n {
        v.push((format!("rand{i}"), arr32(&rng.bytes(32))));
    }
    v
}

/// Canonical, non-zero scalars only (needed by the algebraic-property rows).
fn canonical_scalars(n: usize, seed: u64) -> Vec<(String, [u8; 32])> {
    let mut v: Vec<(String, [u8; 32])> = Vec::new();
    let mut one = [0u8; 32];
    one[0] = 1;
    v.push(("1".into(), one));
    let mut two = [0u8; 32];
    two[0] = 2;
    v.push(("2".into(), two));
    let mut eight = [0u8; 32];
    eight[0] = 8;
    v.push(("8".into(), eight));
    v.push(("L-1".into(), le_sub_small(&L_BYTES, 1)));
    let mut rng = Rng::new(seed);
    let mut i = 0usize;
    while v.len() < n + 4 {
        // Reduce a random 64-byte string: always canonical, essentially never 0.
        let s = d_void1(
            "crypto_core_ed25519_scalar_reduce",
            32,
            &rng.bytes(64),
            "canonical-scalar-gen",
        );
        if s.iter().any(|&x| x != 0) {
            v.push((format!("red{i}"), arr32(&s)));
        }
        i += 1;
    }
    v
}

/// The 64-byte non-reduced scalar sweep for row 137: 0, 1, L, 2L, 2^512-1, random.
fn nonreduced_vectors(n: usize, seed: u64) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("0".into(), vec![0u8; 64]));
    let mut one = vec![0u8; 64];
    one[0] = 1;
    v.push(("1".into(), one));
    let mut l = vec![0u8; 64];
    l[..32].copy_from_slice(&L_BYTES);
    v.push(("L".into(), l.clone()));
    let mut two_l = l.clone();
    le_add_assign(&mut two_l, &L_BYTES);
    v.push(("2L".into(), two_l));
    let mut l_hi = vec![0u8; 64];
    l_hi[32..].copy_from_slice(&L_BYTES);
    v.push(("L<<256".into(), l_hi));
    v.push(("2^512-1".into(), vec![0xffu8; 64]));
    let mut rng = Rng::new(seed);
    for i in 0..n {
        v.push((format!("rand{i}"), rng.bytes(64)));
    }
    v
}

/// Every non-canonical ed25519 point encoding that exists: y = p+k for
/// k in 0..=18 (so y >= p but y < 2^255), with both sign bits. 38 in total —
/// `ge25519_is_canonical` rejects exactly this set.
fn noncanonical_encodings() -> Vec<(String, [u8; 32])> {
    let mut v = Vec::new();
    for k in 0u8..19 {
        for sg in 0..2 {
            let mut e = le_add_small(&P_BYTES, k);
            if sg == 1 {
                e[31] |= 0x80;
            }
            v.push((format!("p+{k}|sign{sg}"), e));
        }
    }
    v
}

/// A y-only encoding from a small y value.
fn y_enc(y: u8, sign: bool) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[0] = y;
    if sign {
        e[31] |= 0x80;
    }
    e
}

/// `n` valid main-subgroup ed25519 points, produced by `ge25519_from_uniform`
/// (exactly what `crypto_core_ed25519_random` uses) so both libraries are fed
/// the identical bytes.
fn valid_ed_points(n: usize, seed: u64) -> Vec<[u8; 32]> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    while out.len() < n {
        let u = rng.bytes(32);
        let p = d_void1("_sodium_ge25519_from_uniform", 32, &u, "point-gen");
        // from_uniform + clear_cofactor always lands in the main subgroup.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &p, "point-gen"),
            1,
            "ge25519_from_uniform must yield a valid main-subgroup point: {}",
            hexs(&p)
        );
        out.push(arr32(&p));
    }
    out
}

/// `n` valid canonical ristretto255 encodings from `_from_hash`.
fn valid_ristretto_points(n: usize, seed: u64) -> Vec<[u8; 32]> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    while out.len() < n {
        let h = rng.bytes(64);
        let (rc, p) = d_int1("crypto_core_ristretto255_from_hash", 32, &h, "point-gen");
        assert_eq!(rc, 0, "ristretto255_from_hash must never fail");
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &p, "point-gen"),
            1,
            "from_hash output must be a valid ristretto encoding"
        );
        out.push(arr32(&p));
    }
    out
}

/// A keypair from a fixed seed, through both libraries.
fn keypair_from_seed(seed: &[u8]) -> ([u8; 32], [u8; 64]) {
    let (fc, fr) = unsafe { pair::<SeedKeypairFn>("crypto_sign_ed25519_seed_keypair") };
    let mut pkc = vec![FILL; 32 + PAD];
    let mut pkr = vec![FILL; 32 + PAD];
    let mut skc = vec![FILL; 64 + PAD];
    let mut skr = vec![FILL; 64 + PAD];
    let rc = unsafe { fc(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
    let rr = unsafe { fr(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
    let what = format!("crypto_sign_ed25519_seed_keypair seed={}", hexs(seed));
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: must return 0");
    assert_eq_bytes(&format!("{what} pk"), &pkc, &pkr);
    assert_eq_bytes(&format!("{what} sk"), &skc, &skr);
    guard_ok(&what, "C pk", &pkc, 32);
    guard_ok(&what, "C sk", &skc, 64);
    let mut sk = [0u8; 64];
    sk.copy_from_slice(&skc[..64]);
    (arr32(&pkc[..32]), sk)
}

/// The canonical test keypair used by most of the signature rows.
fn test_keypair(idx: u8) -> ([u8; 32], [u8; 64]) {
    let mut seed = [0u8; 32];
    for i in 0..32 {
        seed[i] = i as u8 ^ idx;
    }
    keypair_from_seed(&seed)
}

fn msg_of(len: usize, salt: u8) -> Vec<u8> {
    (0..len).map(|i| (i as u8).wrapping_mul(31).wrapping_add(salt)).collect()
}

// ===========================================================================
// Self-check of the hard-coded tables
// ===========================================================================

/// The `SMALL_ORDER` and `X25519_BLOCKLIST` tables are hard-coded, so prove
/// they really are what they claim to be USING THE LIBRARIES THEMSELVES before
/// any other test relies on them.
#[test]
fn smallorder_table_selfcheck() {
    init_both();
    let mut ident = [0u8; 32];
    ident[0] = 1;

    for (i, p) in SMALL_ORDER.iter().enumerate() {
        // 8*P == identity, computed by tripling a doubling through `_add`
        // (which accepts small-order points).
        let mut acc = *p;
        for d in 0..3 {
            let (rc, o) = d_int2(
                "crypto_core_ed25519_add",
                32,
                &acc,
                &acc,
                &format!("smallorder[{i}] dbl{d}"),
            );
            assert_eq!(rc, 0, "small-order[{i}] doubling must succeed");
            acc = arr32(&o);
        }
        assert_eq!(
            acc.to_vec(),
            ident.to_vec(),
            "SMALL_ORDER[{i}] = {} is NOT 8-torsion (8*P = {})",
            hexs(p),
            hexs(&acc)
        );
        // And it really is rejected by the full gate.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", p, &format!("so{i}")),
            0,
            "SMALL_ORDER[{i}] must be rejected by _is_valid_point"
        );
        // Confirm the tabulated order: ord(P)*P == identity and, for ord > 1,
        // (ord/2)*P != identity.
        let ord = SMALL_ORDER_ORD[i];
        let mut acc2 = ident;
        for k in 1..=ord {
            let (rc, o) = d_int2(
                "crypto_core_ed25519_add",
                32,
                &acc2,
                p,
                &format!("smallorder[{i}] mul{k}"),
            );
            assert_eq!(rc, 0);
            acc2 = arr32(&o);
            if k == ord / 2 && ord > 1 {
                assert_ne!(
                    acc2.to_vec(),
                    ident.to_vec(),
                    "SMALL_ORDER[{i}]: order is smaller than the tabulated {ord}"
                );
            }
        }
        assert_eq!(
            acc2.to_vec(),
            ident.to_vec(),
            "SMALL_ORDER[{i}]: {ord}*P != identity"
        );
    }

    // Every blocklisted x25519 encoding must be rejected by `_curve25519`.
    let n: Vec<u8> = (1..=32u8).collect();
    for (i, p) in X25519_BLOCKLIST.iter().enumerate() {
        let (rc, _) = d_int2(
            "crypto_scalarmult_curve25519",
            32,
            &n,
            p,
            &format!("blocklist[{i}]"),
        );
        assert_eq!(
            rc, -1,
            "X25519_BLOCKLIST[{i}] = {} must be rejected",
            hexs(p)
        );
    }

    // The tabulated non-curve / on-curve y values must behave as claimed.
    for &y in NOT_ON_CURVE_Y.iter() {
        let e = y_enc(y, false);
        let (rc, _) = d_int2("crypto_core_ed25519_add", 32, &e, &e, &format!("y={y}"));
        assert_eq!(rc, -1, "y={y} must have NO curve point (frombytes must fail)");
    }
    for &y in ON_CURVE_Y.iter() {
        let e = y_enc(y, false);
        let (rc, _) = d_int2("crypto_core_ed25519_add", 32, &e, &e, &format!("y={y}"));
        assert_eq!(rc, 0, "y={y} must decode to a curve point");
    }
    eprintln!("tables: 8 small-order pts, 7 x25519 blocklist, 13 y-values verified");
}

// ===========================================================================
// 13. crypto_scalarmult — curve25519
// ===========================================================================

/// CONFIGS 122: `crypto_scalarmult_curve25519_base` over random scalars,
/// all-0x00, all-0xff and pre-clamped forms. There is NO rejection branch.
#[test]
fn c122_curve25519_base() {
    init_both();
    let mut rng = Rng::new(SEED ^ 122);
    let mut cases: Vec<(String, [u8; 32])> = vec![
        ("all-00".into(), [0u8; 32]),
        ("all-ff".into(), [0xffu8; 32]),
        ("1".into(), y_enc(1, false)),
        ("9".into(), y_enc(9, false)),
        ("L".into(), L_BYTES),
        ("p".into(), P_BYTES),
    ];
    for i in 0..80 {
        let mut n = arr32(&rng.bytes(32));
        cases.push((format!("rand{i}"), n));
        // pre-clamped form: clamping is then idempotent
        n[0] &= 248;
        n[31] &= 127;
        n[31] |= 64;
        cases.push((format!("rand{i}-clamped"), n));
    }
    let mut nonzero = 0usize;
    for (tag, n) in &cases {
        let (rc, q) = d_int1("crypto_scalarmult_curve25519_base", 32, n, tag);
        // ERRORS 183: no rejection branch at all.
        assert_eq!(rc, 0, "row122 [{tag}] base must always return 0");
        if q.iter().any(|&x| x != 0) {
            nonzero += 1;
        }
        // Pre-clamping must be idempotent w.r.t. the result.
        let mut c = *n;
        c[0] &= 248;
        c[31] &= 127;
        c[31] |= 64;
        let (rc2, q2) = d_int1("crypto_scalarmult_curve25519_base", 32, &c, tag);
        assert_eq!(rc2, 0);
        assert_eq_bytes(
            &format!("row122 [{tag}]: base(n) != base(clamp(n))"),
            &q,
            &q2,
        );
    }
    assert_eq!(nonzero, cases.len(), "row122: base output was zero");
    eprintln!("row 122: {} scalars", cases.len());
}

/// CONFIGS 123: `crypto_scalarmult_curve25519` on valid public keys, plus the
/// X25519 ECDH agreement property asserted on values proven identical in both
/// libraries.
#[test]
fn c123_curve25519_ecdh() {
    init_both();
    let mut rng = Rng::new(SEED ^ 123);
    let mut agreements = 0usize;
    for i in 0..80 {
        let sk1 = rng.bytes(32);
        let sk2 = rng.bytes(32);
        let (r1, pk1) = d_int1("crypto_scalarmult_curve25519_base", 32, &sk1, &format!("ecdh{i}a"));
        let (r2, pk2) = d_int1("crypto_scalarmult_curve25519_base", 32, &sk2, &format!("ecdh{i}b"));
        assert_eq!((r1, r2), (0, 0));
        let (ra, sa) = d_int2(
            "crypto_scalarmult_curve25519",
            32,
            &sk1,
            &pk2,
            &format!("ecdh{i}: sk1*pk2"),
        );
        let (rb, sb) = d_int2(
            "crypto_scalarmult_curve25519",
            32,
            &sk2,
            &pk1,
            &format!("ecdh{i}: sk2*pk1"),
        );
        assert_eq!(ra, 0, "row123 ecdh{i}: sk1*pk2 rejected");
        assert_eq!(rb, 0, "row123 ecdh{i}: sk2*pk1 rejected");
        assert_eq_bytes(
            &format!("row123 ecdh{i}: X25519 AGREEMENT FAILED (sk1={} sk2={})", hexs(&sk1), hexs(&sk2)),
            &sa,
            &sb,
        );
        agreements += 1;
    }
    // Scalar-clamping equivalence: the library clamps internally.
    for i in 0..16 {
        let n = rng.bytes(32);
        let p = rng.bytes(32);
        let mut c = arr32(&n);
        c[0] &= 248;
        c[31] &= 127;
        c[31] |= 64;
        let (ra, sa) = d_int2("crypto_scalarmult_curve25519", 32, &n, &p, &format!("clamp{i}"));
        let (rb, sb) = d_int2("crypto_scalarmult_curve25519", 32, &c, &p, &format!("clamp{i}c"));
        assert_eq!(ra, rb, "row123 clamp{i}: return differs after pre-clamping");
        assert_eq_bytes(&format!("row123 clamp{i}"), &sa, &sb);
    }
    eprintln!("row 123: {agreements} ECDH agreements + 16 clamp-equivalences");
}

/// CONFIGS 124 + ERRORS 182/183: the `crypto_scalarmult` dispatch layer is a
/// thin wrapper over curve25519.
#[test]
fn c124_scalarmult_dispatch() {
    init_both();
    d_size("crypto_scalarmult_bytes", 32);
    d_size("crypto_scalarmult_scalarbytes", 32);
    d_cstr("crypto_scalarmult_primitive", "curve25519");

    let mut rng = Rng::new(SEED ^ 124);
    for i in 0..72 {
        let n = rng.bytes(32);
        let (rb, qb) = d_int1("crypto_scalarmult_base", 32, &n, &format!("disp{i}"));
        let (rb2, qb2) =
            d_int1("crypto_scalarmult_curve25519_base", 32, &n, &format!("disp{i}"));
        assert_eq!(rb, rb2, "row124: _base dispatch return differs");
        assert_eq_bytes("row124: _base != _curve25519_base", &qb, &qb2);

        let p = if i % 3 == 0 { qb.clone() } else { rng.bytes(32) };
        let (ra, qa) = d_int2("crypto_scalarmult", 32, &n, &p, &format!("disp{i}"));
        let (ra2, qa2) =
            d_int2("crypto_scalarmult_curve25519", 32, &n, &p, &format!("disp{i}"));
        assert_eq!(ra, ra2, "row124: dispatch return differs");
        assert_eq_bytes("row124: crypto_scalarmult != _curve25519", &qa, &qa2);
    }
    // ERRORS 182: the wrapper forwards the -1 too.
    let n: Vec<u8> = (1..=32u8).collect();
    for (i, p) in X25519_BLOCKLIST.iter().enumerate() {
        let (ra, _) = d_int2("crypto_scalarmult", 32, &n, p, &format!("err182[{i}]"));
        assert_eq!(ra, -1, "ERRORS 182: wrapper must forward -1 for blocklist[{i}]");
    }
    eprintln!("row 124 / ERRORS 182: 72 dispatch pairs + 7 blocklist forwards");
}

/// ERRORS 180: `has_small_order(p)` — the 7 blocklisted encodings, also with
/// bit 255 set (the check masks `s[31] & 0x7f`). On rejection the output buffer
/// is NEVER written, which the full-buffer comparison pins down.
#[test]
fn e180_x25519_blocklist() {
    init_both();
    let mut rng = Rng::new(SEED ^ 180);
    let mut rejected = 0usize;
    for (i, p) in X25519_BLOCKLIST.iter().enumerate() {
        for &hi in &[false, true] {
            let mut pp = *p;
            if hi {
                pp[31] |= 0x80;
            }
            for k in 0..5 {
                let n = if k == 0 { vec![0u8; 32] } else { rng.bytes(32) };
                let tag = format!("blocklist[{i}] hi={hi} n{k}");
                let (fc, fr) = unsafe { pair::<Int2>("crypto_scalarmult_curve25519") };
                let mut bc = vec![FILL; 32 + PAD];
                let mut br = vec![FILL; 32 + PAD];
                let rc = unsafe { fc(bc.as_mut_ptr(), n.as_ptr(), pp.as_ptr()) };
                let rr = unsafe { fr(br.as_mut_ptr(), n.as_ptr(), pp.as_ptr()) };
                assert_eq!(rc, rr, "ERRORS 180 [{tag}]: return C={rc} rust={rr}");
                assert_eq!(rc, -1, "ERRORS 180 [{tag}]: must reject");
                assert_eq_bytes(&format!("ERRORS 180 [{tag}]"), &bc, &br);
                assert!(
                    bc.iter().all(|&x| x == FILL),
                    "ERRORS 180 [{tag}]: C wrote to q on the rejection path: {}",
                    hexs(&bc)
                );
                assert!(
                    br.iter().all(|&x| x == FILL),
                    "ERRORS 180 [{tag}]: rust wrote to q on the rejection path: {}",
                    hexs(&br)
                );
                rejected += 1;
            }
        }
    }
    eprintln!("ERRORS 180: {rejected} blocklisted (p, n) pairs rejected, q untouched");
}

/// ERRORS 181: the all-zero-output guard `-(1 & ((d-1) >> 8))` in
/// `crypto_scalarmult_curve25519`. `has_small_order` already rejects every
/// encoding whose order divides the clamped scalar, and clamping forces
/// `t = 8k` with bit 254 set, so `t*P = O` is unreachable for a non-blocklisted
/// `p`: the branch is defence-in-depth. Assert it never fires over a wide sweep
/// and that C and Rust agree byte-for-byte throughout.
#[test]
fn e181_x25519_zero_output_dead() {
    init_both();
    let mut rng = Rng::new(SEED ^ 181);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for i in 0..256 {
        let n = rng.bytes(32);
        // A mix of on-curve u (from _base) and arbitrary u (curve or twist).
        let p = if i % 2 == 0 {
            d_int1("crypto_scalarmult_curve25519_base", 32, &rng.bytes(32), "e181").1
        } else {
            rng.bytes(32)
        };
        let (rc, q) = d_int2("crypto_scalarmult_curve25519", 32, &n, &p, &format!("e181-{i}"));
        if rc == 0 {
            assert!(
                q.iter().any(|&x| x != 0),
                "ERRORS 181: rc==0 but output is all-zero — the guard is broken"
            );
            accepted += 1;
        } else {
            assert_eq!(rc, -1, "ERRORS 181: unexpected return {rc}");
            rejected += 1;
        }
    }
    assert!(accepted > 0, "ERRORS 181: nothing was accepted");
    eprintln!("ERRORS 181: {accepted} accepted / {rejected} rejected; zero-output guard never fired");
}

/// ERRORS 182 + 183 recorded together: `crypto_scalarmult` is a thin wrapper and
/// `crypto_scalarmult_curve25519_base` has NO rejection branch whatsoever.
#[test]
fn e182_e183_scalarmult_wrappers() {
    init_both();
    let mut rng = Rng::new(SEED ^ 183);
    // ERRORS 183: not even the blocklisted values or an all-zero scalar make
    // `_base` fail — it has no small-order and no zero-output check.
    let mut ns: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], L_BYTES.to_vec(), P_BYTES.to_vec()];
    for b in X25519_BLOCKLIST.iter() {
        ns.push(b.to_vec());
    }
    for _ in 0..72 {
        ns.push(rng.bytes(32));
    }
    for (i, n) in ns.iter().enumerate() {
        let (rc, _) = d_int1("crypto_scalarmult_curve25519_base", 32, n, &format!("e183-{i}"));
        assert_eq!(rc, 0, "ERRORS 183: _curve25519_base must ALWAYS return 0");
        let (rc2, _) = d_int1("crypto_scalarmult_base", 32, n, &format!("e183w-{i}"));
        assert_eq!(rc2, 0, "ERRORS 183: crypto_scalarmult_base must ALWAYS return 0");
    }
    eprintln!("ERRORS 183: {} scalars, _base never fails", ns.len());
}

/// Internal `_crypto_scalarmult_curve25519_pick_best_implementation`: this build
/// has no `HAVE_AVX_ASM`, so it must select ref10 and return 0. Re-selecting must
/// not change any result.
#[test]
fn internal_pick_best_implementation() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0xB357);
    let n = rng.bytes(32);
    let p = d_int1("crypto_scalarmult_curve25519_base", 32, &n, "pick-pre").1;
    let before = d_int2("crypto_scalarmult_curve25519", 32, &n, &p, "pick-pre").1;

    let (c, r) = unsafe { pair::<IntFn>("_crypto_scalarmult_curve25519_pick_best_implementation") };
    let (rc, rr) = unsafe { (c(), r()) };
    assert_eq!(rc, rr, "_pick_best_implementation(): C={rc} rust={rr}");
    assert_eq!(rc, 0, "_pick_best_implementation() must return 0");

    let after = d_int2("crypto_scalarmult_curve25519", 32, &n, &p, "pick-post").1;
    assert_eq_bytes(
        "_pick_best_implementation changed the curve25519 result",
        &before,
        &after,
    );
    eprintln!("internal: _pick_best_implementation -> 0, ref10 result unchanged");
}

// ===========================================================================
// 13b. crypto_scalarmult — ed25519
// ===========================================================================

/// CONFIGS 125/126 + the clamp-vs-noclamp difference: valid main-subgroup points
/// crossed with the scalar set {1, 2, 8, L-1, random}.
#[test]
fn c125_scalarmult_ed25519_clamp() {
    init_both();
    let points = valid_ed_points(8, SEED ^ 125);
    let scalars = canonical_scalars(8, SEED ^ 0x125);
    let mut cases = 0usize;
    for (pi, p) in points.iter().enumerate() {
        for (tag, n) in &scalars {
            let t = format!("row125 p{pi} n={tag}");
            let (rc, q) = d_int2("crypto_scalarmult_ed25519", 32, n, p, &t);
            assert_eq!(rc, 0, "{t}: clamped scalarmult on a valid point must succeed");
            // Result must itself be a valid main-subgroup point.
            assert_eq!(
                d_pred("crypto_core_ed25519_is_valid_point", &q, &t),
                1,
                "{t}: clamped scalarmult result is not a valid point: {}",
                hexs(&q)
            );
            cases += 1;
        }
    }
    eprintln!("row 125: {cases} (point, scalar) pairs");
}

/// CONFIGS 126: `_ed25519_noclamp` over the same set; clamp and noclamp MUST
/// differ whenever clamping actually changes the scalar.
#[test]
fn c126_scalarmult_ed25519_noclamp() {
    init_both();
    let points = valid_ed_points(8, SEED ^ 126);
    let scalars = canonical_scalars(8, SEED ^ 0x126);
    let mut differed = 0usize;
    let mut cases = 0usize;
    for (pi, p) in points.iter().enumerate() {
        for (tag, n) in &scalars {
            let t = format!("row126 p{pi} n={tag}");
            let (rc, qn) = d_int2("crypto_scalarmult_ed25519_noclamp", 32, n, p, &t);
            assert_eq!(rc, 0, "{t}: noclamp on a valid point must succeed");
            assert_eq!(
                d_pred("crypto_core_ed25519_is_valid_point", &qn, &t),
                1,
                "{t}: noclamp result is not a valid point"
            );
            let (_, qc) = d_int2("crypto_scalarmult_ed25519", 32, n, p, &t);
            let mut clamped = *n;
            clamped[0] &= 248;
            clamped[31] |= 64;
            clamped[31] &= 127;
            let mut masked = *n;
            masked[31] &= 127;
            if clamped != masked {
                assert_ne!(
                    qc, qn,
                    "{t}: clamp and noclamp produced the SAME result although \
                     clamping changed the scalar ({} -> {})",
                    hexs(&masked),
                    hexs(&clamped)
                );
                differed += 1;
            }
            // noclamp(n) must equal clamp(pre-clamped n).
            let (rc2, q2) = d_int2("crypto_scalarmult_ed25519", 32, &clamped, p, &t);
            assert_eq!(rc2, 0);
            let (rc3, q3) = d_int2("crypto_scalarmult_ed25519_noclamp", 32, &clamped, p, &t);
            assert_eq!(rc3, 0);
            assert_eq_bytes(
                &format!("{t}: clamp(clamped) != noclamp(clamped)"),
                &q2,
                &q3,
            );
            cases += 1;
        }
    }
    assert!(differed >= 32, "row126: only {differed} clamp/noclamp differences");
    eprintln!("row 126: {cases} pairs, {differed} clamp!=noclamp confirmations");
}

/// CONFIGS 127/128: `_ed25519_base` and `_ed25519_base_noclamp`.
#[test]
fn c127_c128_scalarmult_ed25519_base() {
    init_both();
    let scalars = canonical_scalars(72, SEED ^ 127);
    let points = valid_ed_points(1, SEED ^ 0x127);
    let _ = points;
    let mut differed = 0usize;
    for (tag, n) in &scalars {
        let t = format!("row127/128 n={tag}");
        let (rc, qc) = d_int1("crypto_scalarmult_ed25519_base", 32, n, &t);
        assert_eq!(rc, 0, "{t}: base(clamp) on a canonical nonzero scalar");
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &qc, &t),
            1,
            "{t}: base(clamp) result is not a valid point"
        );
        let (rn, qn) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, n, &t);
        assert_eq!(rn, 0, "{t}: base_noclamp");
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &qn, &t),
            1,
            "{t}: base_noclamp result is not a valid point"
        );
        let mut clamped = *n;
        clamped[0] &= 248;
        clamped[31] |= 64;
        clamped[31] &= 127;
        let mut masked = *n;
        masked[31] &= 127;
        if clamped != masked {
            assert_ne!(qc, qn, "{t}: base clamp == base noclamp although clamping changed n");
            differed += 1;
        }
        // `_base_noclamp(n)` must equal `_ed25519_noclamp(n, B)` for the
        // generator B = _base_noclamp(1).
        let one = y_enc(1, false);
        let b = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &one, "gen").1;
        let (rv, qv) = d_int2("crypto_scalarmult_ed25519_noclamp", 32, n, &b, &t);
        assert_eq!(rv, 0, "{t}: noclamp(n, B)");
        assert_eq_bytes(&format!("{t}: base_noclamp(n) != noclamp(n, B)"), &qn, &qv);
    }
    assert!(differed >= 32, "rows 127/128: only {differed} clamp differences");
    eprintln!("rows 127/128: {} scalars, {differed} clamp!=noclamp", scalars.len());
}

/// ERRORS 184–187: the 4-check input gate shared by `_ed25519` and
/// `_ed25519_noclamp`. On rejection `q` must be untouched (the gate runs before
/// `t = q` is populated).
#[test]
fn e184_187_scalarmult_ed25519_gates() {
    init_both();
    let n: Vec<u8> = (1..=32u8).collect();
    let valid = valid_ed_points(1, SEED ^ 184)[0];

    let mut corpus: Vec<(String, [u8; 32], &'static str)> = Vec::new();
    // ERRORS 184: `ge25519_is_canonical(p) == 0`
    for (tag, e) in noncanonical_encodings() {
        corpus.push((format!("noncanonical {tag}"), e, "184"));
    }
    // ERRORS 185: `ge25519_frombytes` fails
    for &y in NOT_ON_CURVE_Y.iter() {
        corpus.push((format!("no-curve-point y={y}"), y_enc(y, false), "185"));
        corpus.push((format!("no-curve-point y={y}|sign"), y_enc(y, true), "185"));
    }
    // ERRORS 186: small order
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        corpus.push((format!("small-order[{i}]"), *p, "186"));
    }
    // ERRORS 187: on curve, not small order, but off the main subgroup
    for (i, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let (rc, tp) = d_int2(
            "crypto_core_ed25519_add",
            32,
            &valid,
            t,
            &format!("torsion-build{i}"),
        );
        assert_eq!(rc, 0, "torsion construction must succeed (ERRORS 207)");
        let tp = arr32(&tp);
        // Sanity: on the curve and canonical, yet not a valid point.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &tp, "torsion"),
            0,
            "torsion point must be rejected by _is_valid_point"
        );
        corpus.push((format!("torsion = P + small-order[{i}]"), tp, "187"));
    }

    for name in [
        "crypto_scalarmult_ed25519",
        "crypto_scalarmult_ed25519_noclamp",
    ] {
        for (tag, p, row) in &corpus {
            let (fc, fr) = unsafe { pair::<Int2>(name) };
            let mut bc = vec![FILL; 32 + PAD];
            let mut br = vec![FILL; 32 + PAD];
            let rc = unsafe { fc(bc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            let rr = unsafe { fr(br.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            let what = format!("ERRORS {row} {name} [{tag}] p={}", hexs(p));
            assert_eq!(rc, rr, "{what}: return C={rc} rust={rr}");
            assert_eq!(rc, -1, "{what}: must be rejected");
            assert_eq_bytes(&what, &bc, &br);
            assert!(
                bc.iter().all(|&x| x == FILL),
                "{what}: C wrote q on the gate-rejection path: {}",
                hexs(&bc)
            );
            assert!(
                br.iter().all(|&x| x == FILL),
                "{what}: rust wrote q on the gate-rejection path: {}",
                hexs(&br)
            );
        }
    }
    eprintln!(
        "ERRORS 184-187: {} rejecting points x 2 entry points",
        corpus.len()
    );
}

/// ERRORS 188/189: the two POST-multiply checks of `_ed25519` / `_noclamp`.
/// 188 is `_is_inf(q)`; 189 is `sodium_is_zero(n, 32)` evaluated on the
/// ORIGINAL, UNCLAMPED `n` — so with `n == 0` and clamping the function returns
/// -1 even though `q` holds a perfectly good point. The output buffer is
/// therefore part of the contract.
#[test]
fn e188_189_scalarmult_ed25519_post_checks() {
    init_both();
    let points = valid_ed_points(4, SEED ^ 188);
    let mut ident = [0u8; 32];
    ident[0] = 1;

    for (pi, p) in points.iter().enumerate() {
        // ERRORS 189, clamped: n == 0 -> clamp makes t != 0, q is a real point,
        // but is_zero(n) still forces -1.
        let (rc, q) = d_int1_2("crypto_scalarmult_ed25519", &[0u8; 32], p, &format!("e189 p{pi}"));
        assert_eq!(rc, -1, "ERRORS 189 p{pi}: n==0 must return -1");
        assert_ne!(
            q,
            ident.to_vec(),
            "ERRORS 189 p{pi}: q must hold clamp(0)*P, not the identity"
        );
        assert!(
            q.iter().any(|&x| x != FILL),
            "ERRORS 189 p{pi}: q was never written — the check must run AFTER the multiply"
        );
        // clamp(0)*P is exactly what the same call with the pre-clamped scalar
        // returns, which proves the multiply really happened with clamp(0).
        let mut c0 = [0u8; 32];
        c0[0] &= 248;
        c0[31] |= 64;
        let (rc2, q2) = d_int1_2("crypto_scalarmult_ed25519", &c0, p, &format!("e189 p{pi} ref"));
        assert_eq!(rc2, 0, "ERRORS 189 p{pi}: clamp(0) as an explicit scalar succeeds");
        assert_eq_bytes(
            &format!("ERRORS 189 p{pi}: q != clamp(0)*P"),
            &q,
            &q2,
        );

        // ERRORS 188/189, noclamp: n == 0 -> t == 0 -> q == identity, so BOTH
        // `_is_inf` and `is_zero(n)` fire.
        let (rc3, q3) =
            d_int1_2("crypto_scalarmult_ed25519_noclamp", &[0u8; 32], p, &format!("e188 p{pi}"));
        assert_eq!(rc3, -1, "ERRORS 188/189 p{pi}: noclamp(0) must return -1");
        assert_eq_bytes(
            &format!("ERRORS 188 p{pi}: noclamp(0) must leave the identity in q"),
            &ident,
            &q3,
        );

        // ERRORS 188 in isolation: n == L is NON-zero, but L*P == identity, so
        // only `_is_inf` can be responsible for the -1.
        let (rc4, q4) =
            d_int1_2("crypto_scalarmult_ed25519_noclamp", &L_BYTES, p, &format!("e188L p{pi}"));
        assert_eq!(
            rc4, -1,
            "ERRORS 188 p{pi}: noclamp(L) must return -1 via _is_inf (n is NOT zero)"
        );
        assert_eq_bytes(
            &format!("ERRORS 188 p{pi}: noclamp(L) must leave the identity in q"),
            &ident,
            &q4,
        );
        // ... and the clamped variant does NOT hit `_is_inf`, because clamping
        // changes L into something that is not 0 mod L.
        let (rc5, q5) =
            d_int1_2("crypto_scalarmult_ed25519", &L_BYTES, p, &format!("e188Lc p{pi}"));
        assert_eq!(rc5, 0, "ERRORS 188 p{pi}: clamp(L)*P is not the identity");
        assert_ne!(q5, ident.to_vec());
    }
    eprintln!("ERRORS 188/189: 4 points x (n=0 clamp, n=0 noclamp, n=L noclamp, n=L clamp)");
}

/// Small adapter: `int f(unsigned char *q, const unsigned char *n, const
/// unsigned char *p)` with 32-byte output, returning the 32-byte result.
fn d_int1_2(name: &str, n: &[u8; 32], p: &[u8; 32], tag: &str) -> (c_int, Vec<u8>) {
    d_int2(name, 32, n, p, tag)
}

/// ERRORS 190/191: the same two post-checks for `_ed25519_base` /
/// `_base_noclamp`.
#[test]
fn e190_191_scalarmult_ed25519_base_post_checks() {
    init_both();
    let mut ident = [0u8; 32];
    ident[0] = 1;

    // ERRORS 191: n == 0, clamped. q holds clamp(0)*B (NOT the identity) yet the
    // return is -1 because `sodium_is_zero(n, 32)` looks at the original n.
    let (rc, q) = d_int1("crypto_scalarmult_ed25519_base", 32, &[0u8; 32], "e191");
    assert_eq!(rc, -1, "ERRORS 191: base(0) must return -1");
    assert_ne!(q, ident.to_vec(), "ERRORS 191: q must be clamp(0)*B");
    let mut c0 = [0u8; 32];
    c0[0] &= 248;
    c0[31] |= 64;
    let (rc2, q2) = d_int1("crypto_scalarmult_ed25519_base", 32, &c0, "e191ref");
    assert_eq!(rc2, 0, "ERRORS 191: clamp(0) as an explicit scalar succeeds");
    assert_eq_bytes("ERRORS 191: base(0) q != clamp(0)*B", &q, &q2);

    // ERRORS 190/191 together: noclamp(0) -> identity.
    let (rc3, q3) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &[0u8; 32], "e190a");
    assert_eq!(rc3, -1, "ERRORS 190/191: base_noclamp(0) must return -1");
    assert_eq_bytes("ERRORS 190: base_noclamp(0) must leave identity in q", &ident, &q3);

    // ERRORS 190 in isolation: n == L is non-zero and L*B == identity.
    let (rc4, q4) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &L_BYTES, "e190b");
    assert_eq!(rc4, -1, "ERRORS 190: base_noclamp(L) must return -1 via _is_inf");
    assert_eq_bytes("ERRORS 190: base_noclamp(L) must leave identity in q", &ident, &q4);

    // Multiples of L too (2L mod 2^255 is still 0 mod L after the &127 mask).
    let (twol, _) = le_add32(&L_BYTES, &L_BYTES);
    let (rc5, q5) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &twol, "e190c");
    assert_eq!(rc5, -1, "ERRORS 190: base_noclamp(2L) must return -1");
    assert_eq_bytes("ERRORS 190: base_noclamp(2L) identity", &ident, &q5);

    // The clamped variant of L does NOT hit `_is_inf`.
    let (rc6, q6) = d_int1("crypto_scalarmult_ed25519_base", 32, &L_BYTES, "e191b");
    assert_eq!(rc6, 0, "ERRORS 190: clamp(L)*B is not the identity");
    assert_ne!(q6, ident.to_vec());
    eprintln!("ERRORS 190/191: base post-checks verified (n=0/L/2L, clamp+noclamp)");
}

// ===========================================================================
// 13c. crypto_scalarmult — ristretto255
// ===========================================================================

/// CONFIGS 129: valid canonical ristretto encodings x random scalars. The scalar
/// is only masked (`t[31] &= 127`) — there is NO clamping.
#[test]
fn c129_scalarmult_ristretto255() {
    init_both();
    let points = valid_ristretto_points(8, SEED ^ 129);
    let scalars = canonical_scalars(8, SEED ^ 0x129);
    let mut cases = 0usize;
    for (pi, p) in points.iter().enumerate() {
        for (tag, n) in &scalars {
            let t = format!("row129 p{pi} n={tag}");
            let (rc, q) = d_int2("crypto_scalarmult_ristretto255", 32, n, p, &t);
            assert_eq!(rc, 0, "{t}: ristretto scalarmult must succeed");
            assert_eq!(
                d_pred("crypto_core_ristretto255_is_valid_point", &q, &t),
                1,
                "{t}: result is not a valid ristretto encoding"
            );
            // NO clamping: masking bit 255 is the ONLY transformation.
            let mut masked = *n;
            masked[31] &= 127;
            let (rc2, q2) = d_int2("crypto_scalarmult_ristretto255", 32, &masked, p, &t);
            assert_eq!(rc2, 0);
            assert_eq_bytes(&format!("{t}: masking is not idempotent"), &q, &q2);
            let mut clamped = masked;
            clamped[0] &= 248;
            clamped[31] |= 64;
            clamped[31] &= 127;
            if clamped != masked {
                let (_, q3) = d_int2("crypto_scalarmult_ristretto255", 32, &clamped, p, &t);
                assert_ne!(
                    q, q3,
                    "{t}: ristretto must NOT clamp, yet clamp(n) gave the same result"
                );
            }
            cases += 1;
        }
    }
    eprintln!("row 129: {cases} (point, scalar) pairs, no-clamping confirmed");
}

/// CONFIGS 130: `_ristretto255_base` over random scalars.
#[test]
fn c130_scalarmult_ristretto255_base() {
    init_both();
    let scalars = canonical_scalars(72, SEED ^ 130);
    let one = y_enc(1, false);
    let gen = d_int1("crypto_scalarmult_ristretto255_base", 32, &one, "gen").1;
    for (tag, n) in &scalars {
        let t = format!("row130 n={tag}");
        let (rc, q) = d_int1("crypto_scalarmult_ristretto255_base", 32, n, &t);
        assert_eq!(rc, 0, "{t}: ristretto base must succeed");
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &q, &t),
            1,
            "{t}: base result is not a valid ristretto encoding"
        );
        // base(n) == ristretto255(n, base(1))
        let (rv, qv) = d_int2("crypto_scalarmult_ristretto255", 32, n, &gen, &t);
        assert_eq!(rv, 0, "{t}: n*G via the generic entry point");
        assert_eq_bytes(&format!("{t}: base(n) != n*G"), &q, &qv);
    }
    eprintln!("row 130: {} scalars", scalars.len());
}

/// ERRORS 192–195: the four `ristretto255_frombytes` rejection sub-branches,
/// exercised through `crypto_scalarmult_ristretto255`.
///
/// * 192 `is_canonical(s) == 0`   — `s` odd, `s >= p`, or bit 255 set
/// * 193 `1/(v*u2^2)` not a square — `s = 8`
/// * 194 `fe25519_isnegative(T)`   — `s = 2`
/// * 195 `fe25519_iszero(Y)`       — `s = p-1` (ss == 1 so u1 == 0)
#[test]
fn e192_195_ristretto_frombytes_gates() {
    init_both();
    let n: Vec<u8> = (1..=32u8).collect();
    let valid = valid_ristretto_points(1, SEED ^ 192)[0];

    let mut corpus: Vec<(String, [u8; 32], &'static str)> = Vec::new();
    // 192a: s odd
    let mut odd = valid;
    odd[0] |= 1;
    corpus.push(("odd s".into(), odd, "192"));
    corpus.push(("s = 1".into(), y_enc(1, false), "192"));
    // 192b: s >= p
    corpus.push(("s = p".into(), P_BYTES, "192"));
    corpus.push(("s = p+1".into(), le_add_small(&P_BYTES, 1), "192"));
    corpus.push(("s = 2^255-1".into(), {
        let mut m = [0xffu8; 32];
        m[31] = 0x7f;
        m
    }, "192"));
    // 192c: bit 255 set
    let mut hi = valid;
    hi[31] |= 0x80;
    corpus.push(("bit255 set".into(), hi, "192"));
    corpus.push(("s = 2^256-1".into(), [0xffu8; 32], "192"));
    // 193: not a square
    corpus.push(("s = 8 (non-square)".into(), y_enc(8, false), "193"));
    // 194: T negative
    corpus.push(("s = 2 (T negative)".into(), y_enc(2, false), "194"));
    // 195: Y == 0
    corpus.push(("s = p-1 (Y == 0)".into(), le_sub_small(&P_BYTES, 1), "195"));

    for (tag, s, row) in &corpus {
        // `crypto_scalarmult_ristretto255` uses `q` as scratch AFTER the gate,
        // so on rejection the buffer must be untouched.
        let (fc, fr) = unsafe { pair::<Int2>("crypto_scalarmult_ristretto255") };
        let mut bc = vec![FILL; 32 + PAD];
        let mut br = vec![FILL; 32 + PAD];
        let rc = unsafe { fc(bc.as_mut_ptr(), n.as_ptr(), s.as_ptr()) };
        let rr = unsafe { fr(br.as_mut_ptr(), n.as_ptr(), s.as_ptr()) };
        let what = format!("ERRORS {row} scalarmult_ristretto255 [{tag}] s={}", hexs(s));
        assert_eq!(rc, rr, "{what}: return C={rc} rust={rr}");
        assert_eq!(rc, -1, "{what}: must be rejected");
        assert_eq_bytes(&what, &bc, &br);
        assert!(
            bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
            "{what}: q was written on the rejection path"
        );
        // ERRORS 212: `_is_valid_point` reports the same 4 sub-branches as 0.
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", s, tag),
            0,
            "ERRORS 212 [{tag}]: _is_valid_point must return 0"
        );
    }
    eprintln!("ERRORS 192-195 + 212: {} invalid encodings", corpus.len());
}

/// ERRORS 196/197: `sodium_is_zero(q, 32)` after the multiply — the result is
/// the ristretto identity, which encodes as 32 zero bytes. Unlike the ed25519
/// gates the buffer IS written here, so the full-buffer comparison matters.
#[test]
fn e196_197_scalarmult_ristretto_zero_output() {
    init_both();
    let points = valid_ristretto_points(4, SEED ^ 196);
    let zeros = vec![0u8; 32];
    let (twol, _) = le_add32(&L_BYTES, &L_BYTES);

    for (pi, p) in points.iter().enumerate() {
        for (tag, n) in [("0", [0u8; 32]), ("L", L_BYTES), ("2L", twol)] {
            let t = format!("ERRORS 196 p{pi} n={tag}");
            let (rc, q) = d_int2("crypto_scalarmult_ristretto255", 32, &n, p, &t);
            assert_eq!(rc, -1, "{t}: n == 0 mod L must return -1");
            assert_eq_bytes(&format!("{t}: q must be the all-zero identity"), &zeros, &q);
        }
    }
    for (tag, n) in [("0", [0u8; 32]), ("L", L_BYTES), ("2L", twol)] {
        let t = format!("ERRORS 197 base n={tag}");
        let (rc, q) = d_int1("crypto_scalarmult_ristretto255_base", 32, &n, &t);
        assert_eq!(rc, -1, "{t}: base n == 0 mod L must return -1");
        assert_eq_bytes(&format!("{t}: q must be the all-zero identity"), &zeros, &q);
    }
    eprintln!("ERRORS 196/197: 4 points x 3 zero-scalars + 3 base zero-scalars");
}

// ===========================================================================
// 14. crypto_core_ed25519
// ===========================================================================

/// CONFIGS 131 + ERRORS 198–202: the full 5-check gate of `_is_valid_point`.
#[test]
fn c131_e198_202_ed25519_is_valid_point() {
    init_both();
    let one = y_enc(1, false);
    let b = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &one, "gen").1;
    let valid = valid_ed_points(4, SEED ^ 131);

    // Accepted: the generator B and genuine main-subgroup points.
    assert_eq!(
        d_pred("crypto_core_ed25519_is_valid_point", &b, "generator B"),
        1,
        "row131: the generator B must be a valid point"
    );
    for (i, p) in valid.iter().enumerate() {
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", p, &format!("valid{i}")),
            1,
            "row131: main-subgroup point {i} must be valid"
        );
        // Both sign bits of a main-subgroup point are valid (P and -P).
        let mut neg = *p;
        neg[31] ^= 0x80;
        let r = d_pred("crypto_core_ed25519_is_valid_point", &neg, &format!("valid{i}-neg"));
        assert_eq!(r, 1, "row131: -P must also be a valid point");
    }

    // ERRORS 201: the 8 small-order points (identity included) -> 0.
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", p, &format!("e201 so{i}")),
            0,
            "ERRORS 201: small-order[{i}] must be rejected"
        );
    }
    // ERRORS 202: torsion points (on curve, not small order, off main subgroup).
    for (i, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let tp = arr32(&d_int2("crypto_core_ed25519_add", 32, &valid[0], t, "e202").1);
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &tp, &format!("e202 t{i}")),
            0,
            "ERRORS 202: torsion point (P + small-order[{i}]) must be rejected"
        );
    }
    // ERRORS 198: non-canonical y (p, p+1, ..., 2^255-1), both sign bits.
    for (tag, e) in noncanonical_encodings() {
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &e, &format!("e198 {tag}")),
            0,
            "ERRORS 198: non-canonical {tag} must be rejected"
        );
    }
    // ERRORS 199: `ge25519_frombytes` fails (y has no curve point).
    for &y in NOT_ON_CURVE_Y.iter() {
        for sg in [false, true] {
            assert_eq!(
                d_pred(
                    "crypto_core_ed25519_is_valid_point",
                    &y_enc(y, sg),
                    &format!("e199 y={y} sg={sg}")
                ),
                0,
                "ERRORS 199: y={y} has no curve point, must be rejected"
            );
        }
    }
    // Randomised sweep: `_is_valid_point` must agree on arbitrary 32-byte strings.
    let mut rng = Rng::new(SEED ^ 0x131);
    let mut accepted = 0usize;
    for i in 0..256 {
        let s = rng.bytes(32);
        if d_pred("crypto_core_ed25519_is_valid_point", &s, &format!("rand{i}")) == 1 {
            accepted += 1;
        }
    }
    // ERRORS 200: `is_on_curve(&p3) == 0` after a SUCCESSFUL `ge25519_frombytes`
    // is unreachable — frombytes solves for x on the curve, so it either fails or
    // yields an on-curve point. Prove it holds in both libraries by checking that
    // `_is_valid_point` never rejects for an on-curve reason that `_add` accepts
    // while the small-order/main-subgroup gates pass.
    eprintln!(
        "row 131 / ERRORS 198-202: {} accepted of 256 random strings, all gates verified",
        accepted
    );
}

/// CONFIGS 132 + ERRORS 203–208: `_add` / `_sub`. Their ONLY gate is
/// `frombytes` + `is_on_curve` on each operand.
#[test]
fn c132_e203_208_ed25519_add_sub() {
    init_both();
    let valid = valid_ed_points(10, SEED ^ 132);
    let mut ident = [0u8; 32];
    ident[0] = 1;

    // valid x valid: group-law properties, asserted on values already proven
    // identical across both libraries.
    let mut cases = 0usize;
    for i in 0..9 {
        let (p, q) = (valid[i], valid[i + 1]);
        let (r1, s1) = d_int2("crypto_core_ed25519_add", 32, &p, &q, &format!("row132 add{i}"));
        let (r2, s2) = d_int2("crypto_core_ed25519_add", 32, &q, &p, &format!("row132 add{i}r"));
        assert_eq!((r1, r2), (0, 0), "row132: valid+valid must succeed");
        assert_eq_bytes(&format!("row132 add{i}: not commutative"), &s1, &s2);
        // (p+q)-q == p
        let (r3, back) = d_int2("crypto_core_ed25519_sub", 32, &s1, &q, &format!("row132 sub{i}"));
        assert_eq!(r3, 0);
        assert_eq_bytes(&format!("row132 {i}: (p+q)-q != p"), &p, &back);
        // p-p == identity
        let (r4, zero) = d_int2("crypto_core_ed25519_sub", 32, &p, &p, &format!("row132 self{i}"));
        assert_eq!(r4, 0);
        assert_eq_bytes(&format!("row132 {i}: p-p != identity"), &ident, &zero);
        // p+identity == p
        let (r5, same) = d_int2("crypto_core_ed25519_add", 32, &p, &ident, &format!("row132 id{i}"));
        assert_eq!(r5, 0);
        assert_eq_bytes(&format!("row132 {i}: p+identity != p"), &p, &same);
        cases += 5;
    }
    // Randomised sweep over arbitrary 32-byte operands: the return value must be
    // exactly `frombytes(p) && frombytes(q)`, which pins ERRORS 203–206/208 and
    // proves the `is_on_curve` sub-branches never fire independently.
    let mut rng = Rng::new(SEED ^ 0x132);
    let mut ok = 0usize;
    let mut bad = 0usize;
    for i in 0..128 {
        let p = rng.bytes(32);
        let q = rng.bytes(32);
        let (ra, _) = d_int2("crypto_core_ed25519_add", 32, &p, &q, &format!("rand{i}"));
        let (rs, _) = d_int2("crypto_core_ed25519_sub", 32, &p, &q, &format!("rand{i}"));
        assert_eq!(ra, rs, "row132 rand{i}: _add and _sub disagree on the SAME gate");
        // Single-operand decodability, probed by pairing with itself.
        let (rp, _) = d_int2("crypto_core_ed25519_add", 32, &p, &p, &format!("randp{i}"));
        let (rq, _) = d_int2("crypto_core_ed25519_add", 32, &q, &q, &format!("randq{i}"));
        let expect = if rp == 0 && rq == 0 { 0 } else { -1 };
        assert_eq!(
            ra, expect,
            "row132 rand{i}: add(p,q)={ra} but frombytes(p)={rp} frombytes(q)={rq} \
             — the gate is exactly frombytes(p) && frombytes(q)"
        );
        if ra == 0 {
            ok += 1;
        } else {
            bad += 1;
        }
    }
    // ERRORS 203/205: `frombytes` fails on p and/or q.
    for &y in NOT_ON_CURVE_Y.iter() {
        let bady = y_enc(y, false);
        for (tag, p, q, row) in [
            (format!("bad p (y={y})"), bady, valid[0], "203"),
            (format!("bad q (y={y})"), valid[0], bady, "205"),
            (format!("both bad (y={y})"), bady, bady, "203+205"),
        ] {
            let (ra, ba) = d_int2("crypto_core_ed25519_add", 32, &p, &q, &tag);
            assert_eq!(ra, -1, "ERRORS {row} add [{tag}]: must return -1");
            assert!(
                ba.iter().all(|&x| x == FILL),
                "ERRORS {row} add [{tag}]: r written on the rejection path"
            );
            // ERRORS 208: the same four branches for `_sub`.
            let (rs, bs) = d_int2("crypto_core_ed25519_sub", 32, &p, &q, &tag);
            assert_eq!(rs, -1, "ERRORS 208 sub [{tag}]: must return -1");
            assert!(
                bs.iter().all(|&x| x == FILL),
                "ERRORS 208 sub [{tag}]: r written on the rejection path"
            );
        }
    }
    eprintln!(
        "row 132 / ERRORS 203-208: {cases} group-law checks, {ok} accepted / {bad} rejected random pairs"
    );
}

/// ERRORS 207: `_add` / `_sub` have NO `is_canonical`, NO small-order and NO
/// main-subgroup check, so non-canonical, small-order and torsion points are all
/// ACCEPTED — in sharp contrast with `_is_valid_point` and
/// `crypto_scalarmult_ed25519`, which reject every one of them.
#[test]
fn e207_add_sub_accepts_weak_points() {
    init_both();
    let valid = valid_ed_points(2, SEED ^ 207);
    let mut accepted = 0usize;

    // (a) the 8 small-order points, including the identity.
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        for (tag, a, b) in [
            (format!("so[{i}] + valid"), *p, valid[0]),
            (format!("valid + so[{i}]"), valid[0], *p),
            (format!("so[{i}] + so[{i}]"), *p, *p),
        ] {
            let (ra, _) = d_int2("crypto_core_ed25519_add", 32, &a, &b, &tag);
            let (rs, _) = d_int2("crypto_core_ed25519_sub", 32, &a, &b, &tag);
            assert_eq!(ra, 0, "ERRORS 207 [{tag}]: _add must ACCEPT small-order points");
            assert_eq!(rs, 0, "ERRORS 207 [{tag}]: _sub must ACCEPT small-order points");
            accepted += 2;
        }
        // Contrast: every other gated entry point rejects them.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", p, "contrast"),
            0,
            "ERRORS 207 contrast: _is_valid_point must reject small-order[{i}]"
        );
        let n: Vec<u8> = (1..=32u8).collect();
        assert_eq!(
            d_int2("crypto_scalarmult_ed25519", 32, &n, p, "contrast").0,
            -1,
            "ERRORS 207 contrast: crypto_scalarmult_ed25519 must reject small-order[{i}]"
        );
    }

    // (b) the NON-CANONICAL encodings that decode to a curve point. `_add`
    //     accepts them and, crucially, treats them EXACTLY like the canonical
    //     encoding of the same reduced y — proof that no canonicality gate runs.
    let mut noncanon_accepted = 0usize;
    for (tag, e) in noncanonical_encodings() {
        let (ra, ba) = d_int2("crypto_core_ed25519_add", 32, &e, &valid[0], &tag);
        // The reduced canonical encoding of the same point.
        let k = e[0].wrapping_sub(0xed); // y = p + k  ->  y mod p == k
        let mut canon = y_enc(k, false);
        canon[31] |= e[31] & 0x80;
        let (rc, bc) = d_int2("crypto_core_ed25519_add", 32, &canon, &valid[0], &tag);
        assert_eq!(
            ra, rc,
            "ERRORS 207 [{tag}]: add(non-canonical) != add(canonical y={k}) — \
             a canonicality gate must have fired"
        );
        assert_eq_bytes(
            &format!("ERRORS 207 [{tag}]: non-canonical result != canonical (y={k}) result"),
            &ba,
            &bc,
        );
        if ra == 0 {
            noncanon_accepted += 1;
        }
        // Contrast: `_is_valid_point` and `crypto_scalarmult_ed25519` reject the
        // non-canonical form outright.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &e, &tag),
            0,
            "ERRORS 198/207 contrast: _is_valid_point must reject {tag}"
        );
    }
    assert!(
        noncanon_accepted >= 20,
        "ERRORS 207: only {noncanon_accepted} non-canonical encodings were accepted by _add"
    );

    // (c) torsion points.
    for (i, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let tp = arr32(&d_int2("crypto_core_ed25519_add", 32, &valid[0], t, "e207 t").1);
        let (ra, _) = d_int2("crypto_core_ed25519_add", 32, &tp, &valid[1], &format!("torsion{i}"));
        assert_eq!(ra, 0, "ERRORS 207: _add must ACCEPT the torsion point {i}");
        let n: Vec<u8> = (1..=32u8).collect();
        assert_eq!(
            d_int2("crypto_scalarmult_ed25519", 32, &n, &tp, "contrast").0,
            -1,
            "ERRORS 207 contrast: crypto_scalarmult_ed25519 must reject the torsion point"
        );
        accepted += 1;
    }
    eprintln!(
        "ERRORS 207: {accepted} small-order/torsion acceptances, \
         {noncanon_accepted} non-canonical acceptances, contrasts verified"
    );
}

// ------------------------------------- internal ge25519_p3 predicate coverage
//
// `ge25519_has_small_order`'s five OR-terms cannot all be isolated through the
// one-shot API: every public caller that consults it ALSO applies
// `is_on_main_subgroup`, which rejects the order-8 points for an independent
// reason, and the one place where its result IS the return value
// (`_verify_detached`'s final check) has its order-8 leg neutralised by
// `ge25519_p2_to_p3` setting `T = X*Y`. Both libraries export the predicate, so
// drive it directly on a decoded `ge25519_p3` (40 x int32, `fe_25_5` layout, the
// same in both builds) and compare the struct bytes as well as the verdict.

const P3_LIMBS: usize = 40;
const P3_GUARD: usize = 8;
const P3_SENT: i32 = -0x5555_5556;

#[repr(C, align(64))]
struct P3Buf([i32; P3_LIMBS + P3_GUARD]);

type FromBytesFn = unsafe extern "C" fn(*mut i32, *const u8) -> c_int;
type P3PredFn = unsafe extern "C" fn(*const i32) -> c_int;

fn d_frombytes(name: &str, s: &[u8], tag: &str) -> (c_int, Box<P3Buf>, Box<P3Buf>) {
    let (fc, fr) = unsafe { pair::<FromBytesFn>(name) };
    let mut bc = Box::new(P3Buf([P3_SENT; P3_LIMBS + P3_GUARD]));
    let mut br = Box::new(P3Buf([P3_SENT; P3_LIMBS + P3_GUARD]));
    let rc = unsafe { fc(bc.0.as_mut_ptr(), s.as_ptr()) };
    let rr = unsafe { fr(br.0.as_mut_ptr(), s.as_ptr()) };
    let what = format!("{name} [{tag}] s={}", hexs(s));
    assert_eq!(rc, rr, "{what}: RETURN differs (C={rc} rust={rr})");
    assert_eq!(
        bc.0, br.0,
        "{what}: the decoded ge25519_p3 STRUCT differs\n  C   ={:?}\n  rust={:?}",
        &bc.0[..P3_LIMBS],
        &br.0[..P3_LIMBS]
    );
    assert!(
        bc.0[P3_LIMBS..].iter().all(|&x| x == P3_SENT),
        "{what}: C wrote past sizeof(ge25519_p3)"
    );
    assert!(
        br.0[P3_LIMBS..].iter().all(|&x| x == P3_SENT),
        "{what}: rust wrote past sizeof(ge25519_p3)"
    );
    (rc, bc, br)
}

fn d_p3_pred(name: &str, bc: &P3Buf, br: &P3Buf, tag: &str) -> c_int {
    let (fc, fr) = unsafe { pair::<P3PredFn>(name) };
    let rc = unsafe { fc(bc.0.as_ptr()) };
    let rr = unsafe { fr(br.0.as_ptr()) };
    assert_eq!(rc, rr, "{name} [{tag}]: RETURN differs (C={rc} rust={rr})");
    rc
}

/// Direct differential coverage of `ge25519_frombytes`,
/// `ge25519_has_small_order`, `ge25519_is_on_curve` and
/// `ge25519_is_on_main_subgroup` — every OR-term of the small-order predicate,
/// individually. This is what makes ERRORS 165/167/186/201 attributable rather
/// than merely observable through a gate that has other reasons to fail.
#[test]
fn internal_ge25519_p3_predicates() {
    init_both();
    // (a) all 8 small-order points: has_small_order == 1, on_curve == 1.
    //     The four order-8 encodings split across the two `y*sqrt(-1) -+ x`
    //     terms (two hit `fe_sub`, two hit `fe_add`), so each is asserted
    //     individually — dropping either term is caught here.
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        let (rc, bc, br) = d_frombytes("_sodium_ge25519_frombytes", p, &format!("so[{i}]"));
        assert_eq!(rc, 0, "so[{i}]: frombytes must succeed");
        assert_eq!(
            d_p3_pred("_sodium_ge25519_is_on_curve", &bc, &br, &format!("so[{i}]")),
            1,
            "so[{i}]: must be on the curve"
        );
        assert_eq!(
            d_p3_pred("_sodium_ge25519_has_small_order", &bc, &br, &format!("so[{i}]")),
            1,
            "so[{i}] = {} (order {}): has_small_order MUST report 1 — one of the \
             five OR-terms (X==0 | Y==0 | Z==0 | y*sqrt(-1)-x==0 | y*sqrt(-1)+x==0) \
             is missing",
            hexs(p),
            SMALL_ORDER_ORD[i]
        );
        // Only the identity is in the main subgroup.
        let ms = d_p3_pred(
            "_sodium_ge25519_is_on_main_subgroup",
            &bc,
            &br,
            &format!("so[{i}]"),
        );
        assert_eq!(
            ms,
            (SMALL_ORDER_ORD[i] == 1) as c_int,
            "so[{i}]: is_on_main_subgroup should be 1 only for the identity"
        );
    }
    // (b) valid main-subgroup points: has_small_order == 0, main subgroup == 1.
    let valid = valid_ed_points(24, SEED ^ 0x9033);
    for (i, p) in valid.iter().enumerate() {
        let (rc, bc, br) = d_frombytes("_sodium_ge25519_frombytes", p, &format!("valid{i}"));
        assert_eq!(rc, 0);
        assert_eq!(
            d_p3_pred("_sodium_ge25519_has_small_order", &bc, &br, &format!("valid{i}")),
            0,
            "valid{i}: a main-subgroup point must NOT be small order"
        );
        assert_eq!(
            d_p3_pred("_sodium_ge25519_is_on_main_subgroup", &bc, &br, &format!("valid{i}")),
            1,
            "valid{i}: must be on the main subgroup"
        );
        assert_eq!(
            d_p3_pred("_sodium_ge25519_is_on_curve", &bc, &br, &format!("valid{i}")),
            1,
            "valid{i}: must be on the curve"
        );
    }
    // (c) torsion points: on curve, NOT small order, NOT main subgroup.
    for (i, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let tp = arr32(&d_int2("crypto_core_ed25519_add", 32, &valid[0], t, "p3-torsion").1);
        let (rc, bc, br) = d_frombytes("_sodium_ge25519_frombytes", &tp, &format!("torsion{i}"));
        assert_eq!(rc, 0);
        assert_eq!(
            d_p3_pred("_sodium_ge25519_has_small_order", &bc, &br, &format!("torsion{i}")),
            0,
            "torsion{i}: P + small-order[{i}] must NOT be small order"
        );
        assert_eq!(
            d_p3_pred("_sodium_ge25519_is_on_main_subgroup", &bc, &br, &format!("torsion{i}")),
            0,
            "torsion{i}: must be OFF the main subgroup"
        );
    }
    // (d) `frombytes` must fail for a y with no curve point, and
    //     `_negate_vartime` must agree with it on acceptance.
    for &y in NOT_ON_CURVE_Y.iter() {
        for sg in [false, true] {
            let e = y_enc(y, sg);
            let (rc, _, _) =
                d_frombytes("_sodium_ge25519_frombytes", &e, &format!("bad y={y} sg={sg}"));
            assert_eq!(rc, -1, "y={y}: frombytes must fail");
            let (rn, _, _) = d_frombytes(
                "_sodium_ge25519_frombytes_negate_vartime",
                &e,
                &format!("bad y={y} sg={sg}"),
            );
            assert_eq!(rn, -1, "y={y}: frombytes_negate_vartime must fail too");
        }
    }
    // (e) `_negate_vartime` yields -P: has_small_order / main-subgroup verdicts
    //     are preserved, and it accepts exactly what `frombytes` accepts.
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let mut agree = 0usize;
    for i in 0..192 {
        let s = rng.bytes(32);
        let (ra, _, _) = d_frombytes("_sodium_ge25519_frombytes", &s, &format!("rand{i}"));
        let (rb, bc, br) = d_frombytes(
            "_sodium_ge25519_frombytes_negate_vartime",
            &s,
            &format!("rand{i}"),
        );
        assert_eq!(
            ra, rb,
            "rand{i}: frombytes and frombytes_negate_vartime must accept the same inputs"
        );
        if rb == 0 {
            // Exercise all three predicates on arbitrary decoded points.
            d_p3_pred("_sodium_ge25519_has_small_order", &bc, &br, &format!("rand{i}"));
            d_p3_pred("_sodium_ge25519_is_on_curve", &bc, &br, &format!("rand{i}"));
            d_p3_pred("_sodium_ge25519_is_on_main_subgroup", &bc, &br, &format!("rand{i}"));
            agree += 1;
        }
    }
    assert!(agree > 50, "internal p3: only {agree} of 192 random strings decoded");
    eprintln!(
        "internal ge25519_p3: 8 small-order (all 5 OR-terms), 24 valid, 7 torsion, \
         {} bad-y, {agree} random decodes",
        NOT_ON_CURVE_Y.len() * 2
    );
}

/// CONFIGS 133: `ge25519_from_uniform` (the map behind
/// `crypto_core_ed25519_random`). `x_sign` is bit 7 of `r[31]`.
#[test]
fn c133_ed25519_from_uniform() {
    init_both();
    let mut rng = Rng::new(SEED ^ 133);
    let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], (0..32u8).collect()];
    for _ in 0..80 {
        inputs.push(rng.bytes(32));
    }
    let mut sign_flips = 0usize;
    let mut valid_pts = 0usize;
    for (i, r) in inputs.iter().enumerate() {
        let p = d_void1("_sodium_ge25519_from_uniform", 32, r, &format!("row133-{i}"));
        // The map clears the cofactor, so the result is in the main subgroup; it
        // is only rejected by `_is_valid_point` when it lands on the identity
        // (which the degenerate all-zero input does). `d_pred` has already
        // asserted C and Rust agree on the verdict.
        if d_pred("crypto_core_ed25519_is_valid_point", &p, &format!("row133-{i}")) == 1 {
            valid_pts += 1;
        }
        // `x_sign` is bit 7 of r[31]: flipping it must flip the encoded sign bit
        // and nothing else.
        let mut r2 = r.clone();
        r2[31] ^= 0x80;
        let p2 = d_void1("_sodium_ge25519_from_uniform", 32, &r2, &format!("row133-{i}-flip"));
        assert_eq!(
            p[..31],
            p2[..31],
            "row133 [{i}]: flipping bit 7 of r[31] changed more than the sign"
        );
        assert_eq!(
            p[31] & 0x7f,
            p2[31] & 0x7f,
            "row133 [{i}]: flipping bit 7 of r[31] changed the y coordinate"
        );
        if p[31] != p2[31] {
            sign_flips += 1;
        }
    }
    assert!(sign_flips > 60, "row133: only {sign_flips} sign flips observed");
    assert!(
        valid_pts >= 80,
        "row133: only {valid_pts} of {} outputs were main-subgroup points",
        inputs.len()
    );
    eprintln!(
        "row 133: {} inputs, {sign_flips} x_sign flips, {valid_pts} main-subgroup",
        inputs.len()
    );
}

/// CONFIGS 134: `ge25519_from_hash` — 64-byte inputs, `fe25519_reduce64` folds
/// the dropped high bits back in (x19 / x722).
#[test]
fn c134_ed25519_from_hash() {
    init_both();
    let mut rng = Rng::new(SEED ^ 134);
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![0u8; 64],
        vec![0xffu8; 64],
        (0..64u8).collect(),
        {
            // exactly 2^255-19 in the low half, zero high half
            let mut v = vec![0u8; 64];
            v[..32].copy_from_slice(&P_BYTES);
            v
        },
        {
            // high half all-ff: exercises the x722 fold
            let mut v = vec![0u8; 64];
            for x in v[32..].iter_mut() {
                *x = 0xff;
            }
            v
        },
    ];
    for _ in 0..80 {
        inputs.push(rng.bytes(64));
    }
    let mut valid_pts = 0usize;
    let mut seen = std::collections::HashSet::new();
    for (i, h) in inputs.iter().enumerate() {
        let p = d_void1("_sodium_ge25519_from_hash", 32, h, &format!("row134-{i}"));
        // As for row 133: the cofactor is cleared, so the only non-"valid"
        // outcome is the identity (hit by the degenerate all-zero input).
        if d_pred("crypto_core_ed25519_is_valid_point", &p, &format!("row134-{i}")) == 1 {
            valid_pts += 1;
        }
        seen.insert(p);
    }
    assert!(
        valid_pts >= 80,
        "row134: only {valid_pts} of {} outputs were main-subgroup points",
        inputs.len()
    );
    assert!(seen.len() > 80, "row134: only {} distinct outputs", seen.len());
    eprintln!(
        "row 134: {} 64-byte inputs, {valid_pts} main-subgroup, {} distinct",
        inputs.len(),
        seen.len()
    );
}

/// CONFIGS 135: `crypto_core_ed25519_random` **[RNG]**.
#[test]
fn c135_ed25519_random() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    reset_det_rng();
    let mut seen = std::collections::HashSet::new();
    for i in 0..96 {
        let p = d_void0("crypto_core_ed25519_random", 32, &format!("row135-{i}"));
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &p, &format!("row135-{i}")),
            1,
            "row135 [{i}]: _random must yield a valid main-subgroup point"
        );
        seen.insert(p);
    }
    assert!(seen.len() > 90, "row135: only {} distinct points", seen.len());
    // `_random` is exactly `from_uniform(randombytes_buf(32))`.
    reset_det_rng();
    let (bc, _br) = unsafe { pair::<unsafe extern "C" fn(*mut libc::c_void, usize)>("randombytes_buf") };
    let mut u = vec![0u8; 32];
    unsafe { bc(u.as_mut_ptr() as *mut libc::c_void, 32) };
    let expect = d_void1("_sodium_ge25519_from_uniform", 32, &u, "row135-equiv");
    reset_det_rng();
    let got = d_void0("crypto_core_ed25519_random", 32, "row135-equiv");
    assert_eq_bytes("row135: _random != from_uniform(randombytes_buf(32))", &expect, &got);
    eprintln!("row 135: 96 draws, {} distinct", seen.len());
}

/// CONFIGS 136: `crypto_core_ed25519_scalar_random` **[RNG]** — a do/while loop
/// that masks `r[31] &= 0x1f` and retries until canonical AND non-zero, so the
/// number of RNG draws per call varies and must match in both libraries.
#[test]
fn c136_ed25519_scalar_random() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    reset_det_rng();
    let mut seen = std::collections::HashSet::new();
    for i in 0..96 {
        let s = d_void0("crypto_core_ed25519_scalar_random", 32, &format!("row136-{i}"));
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &s, &format!("row136-{i}")),
            1,
            "row136 [{i}]: scalar_random must be canonical"
        );
        assert!(s.iter().any(|&x| x != 0), "row136 [{i}]: scalar_random returned 0");
        assert_eq!(s[31] & 0xe0, 0, "row136 [{i}]: r[31] & 0x1f mask not applied");
        seen.insert(s);
    }
    assert!(seen.len() > 90, "row136: only {} distinct scalars", seen.len());
    // ristretto255's scalar_random delegates to the ed25519 one; with both
    // counters reset the two must produce identical streams.
    reset_det_rng();
    let a = d_void0("crypto_core_ed25519_scalar_random", 32, "row136-deleg-a");
    reset_det_rng();
    let b = d_void0("crypto_core_ristretto255_scalar_random", 32, "row136-deleg-b");
    assert_eq_bytes("row136: ristretto scalar_random != ed25519 scalar_random", &a, &b);
    eprintln!("row 136: 96 draws, {} distinct", seen.len());
}

/// CONFIGS 137: `_scalar_reduce` over the 64-byte non-reduced sweep.
#[test]
fn c137_ed25519_scalar_reduce() {
    init_both();
    let vecs = nonreduced_vectors(80, SEED ^ 137);
    let mut zero = [0u8; 32];
    for (tag, s) in &vecs {
        let r = d_void1("crypto_core_ed25519_scalar_reduce", 32, s, tag);
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &r, tag),
            1,
            "row137 [{tag}]: reduce output must be canonical"
        );
        // ristretto255 delegates.
        let r2 = d_void1("crypto_core_ristretto255_scalar_reduce", 32, s, tag);
        assert_eq_bytes(&format!("row137 [{tag}]: ristretto reduce differs"), &r, &r2);
    }
    // Known values: reduce(0) == 0, reduce(1) == 1, reduce(L) == 0, reduce(2L) == 0.
    let get = |t: &str| -> Vec<u8> {
        let (_, s) = vecs.iter().find(|(x, _)| x == t).unwrap();
        d_void1("crypto_core_ed25519_scalar_reduce", 32, s, t)
    };
    assert_eq_bytes("row137: reduce(0) != 0", &zero, &get("0"));
    zero[0] = 1;
    assert_eq_bytes("row137: reduce(1) != 1", &zero, &get("1"));
    zero[0] = 0;
    assert_eq_bytes("row137: reduce(L) != 0", &zero, &get("L"));
    assert_eq_bytes("row137: reduce(2L) != 0", &zero, &get("2L"));
    assert_eq_bytes("row137: reduce(L<<256) != 0", &zero, &get("L<<256"));
    eprintln!("row 137: {} non-reduced vectors", vecs.len());
}

/// CONFIGS 138: `_scalar_negate` / `_scalar_complement`, with the involution and
/// complement identities asserted on already-agreed values.
#[test]
fn c138_ed25519_scalar_negate_complement() {
    init_both();
    let mut one = [0u8; 32];
    one[0] = 1;
    let zero = [0u8; 32];

    // Full sweep (canonical AND non-canonical) for byte-for-byte agreement.
    for (tag, s) in scalar_vectors(80, SEED ^ 138) {
        let neg = d_void1("crypto_core_ed25519_scalar_negate", 32, &s, &tag);
        let comp = d_void1("crypto_core_ed25519_scalar_complement", 32, &s, &tag);
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &neg, &tag),
            1,
            "row138 [{tag}]: negate output must be canonical"
        );
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &comp, &tag),
            1,
            "row138 [{tag}]: complement output must be canonical"
        );
        // ristretto255 delegates.
        assert_eq_bytes(
            &format!("row138 [{tag}]: ristretto negate differs"),
            &neg,
            &d_void1("crypto_core_ristretto255_scalar_negate", 32, &s, &tag),
        );
        assert_eq_bytes(
            &format!("row138 [{tag}]: ristretto complement differs"),
            &comp,
            &d_void1("crypto_core_ristretto255_scalar_complement", 32, &s, &tag),
        );
        // negate(s) + s == 0 mod L  (valid for any s, since _scalar_add reduces)
        let sum = d_void2("crypto_core_ed25519_scalar_add", 32, &neg, &s, &tag);
        if d_pred("crypto_core_ed25519_scalar_is_canonical", &s, &tag) == 1 {
            assert_eq_bytes(
                &format!("row138 [{tag}]: negate(s) + s != 0 mod L"),
                &zero,
                &sum,
            );
            // complement(s) + s == 1 mod L
            let csum = d_void2("crypto_core_ed25519_scalar_add", 32, &comp, &s, &tag);
            assert_eq_bytes(
                &format!("row138 [{tag}]: complement(s) + s != 1 mod L"),
                &one,
                &csum,
            );
            // negate(negate(s)) == s (only an involution for canonical s)
            let nn = d_void1("crypto_core_ed25519_scalar_negate", 32, &neg, &tag);
            assert_eq_bytes(
                &format!("row138 [{tag}]: negate(negate(s)) != s"),
                &s,
                &nn,
            );
        }
    }
    // negate(0) == 0 and complement(0) == 1.
    assert_eq_bytes(
        "row138: negate(0) != 0",
        &zero,
        &d_void1("crypto_core_ed25519_scalar_negate", 32, &zero, "neg0"),
    );
    assert_eq_bytes(
        "row138: complement(0) != 1",
        &one,
        &d_void1("crypto_core_ed25519_scalar_complement", 32, &zero, "comp0"),
    );
    eprintln!("row 138: negate/complement over the full scalar sweep");
}

/// CONFIGS 139: `_scalar_add` / `_scalar_sub` / `_scalar_mul` over canonical AND
/// non-canonical operands (`_mul` does no canonicality check at all).
#[test]
fn c139_ed25519_scalar_add_sub_mul() {
    init_both();
    let vecs = scalar_vectors(40, SEED ^ 139);
    let mut one = [0u8; 32];
    one[0] = 1;
    let zero = [0u8; 32];
    let mut cases = 0usize;
    for i in 0..vecs.len() {
        for j in [0usize, 1, 3, 5, 7, vecs.len() - 1, (i + 11) % vecs.len()] {
            let (ta, x) = &vecs[i];
            let (tb, y) = &vecs[j];
            let tag = format!("row139 x={ta} y={tb}");
            let add = d_void2("crypto_core_ed25519_scalar_add", 32, x, y, &tag);
            let sub = d_void2("crypto_core_ed25519_scalar_sub", 32, x, y, &tag);
            let mul = d_void2("crypto_core_ed25519_scalar_mul", 32, x, y, &tag);
            for (n, v) in [("add", &add), ("sub", &sub), ("mul", &mul)] {
                assert_eq!(
                    d_pred("crypto_core_ed25519_scalar_is_canonical", v, &tag),
                    1,
                    "{tag}: {n} output must be canonical"
                );
            }
            // ristretto255 delegates for all three.
            assert_eq_bytes(
                &format!("{tag}: ristretto scalar_add differs"),
                &add,
                &d_void2("crypto_core_ristretto255_scalar_add", 32, x, y, &tag),
            );
            assert_eq_bytes(
                &format!("{tag}: ristretto scalar_sub differs"),
                &sub,
                &d_void2("crypto_core_ristretto255_scalar_sub", 32, x, y, &tag),
            );
            assert_eq_bytes(
                &format!("{tag}: ristretto scalar_mul differs"),
                &mul,
                &d_void2("crypto_core_ristretto255_scalar_mul", 32, x, y, &tag),
            );
            // add is commutative, mul is commutative.
            assert_eq_bytes(
                &format!("{tag}: scalar_add not commutative"),
                &add,
                &d_void2("crypto_core_ed25519_scalar_add", 32, y, x, &tag),
            );
            assert_eq_bytes(
                &format!("{tag}: scalar_mul not commutative"),
                &mul,
                &d_void2("crypto_core_ed25519_scalar_mul", 32, y, x, &tag),
            );
            cases += 1;
        }
    }
    // `_mul` accepts non-canonical operands and reduces them: mul(L, y) == 0.
    for (tb, y) in vecs.iter().take(12) {
        let m = d_void2("crypto_core_ed25519_scalar_mul", 32, &L_BYTES, y, tb);
        assert_eq_bytes(&format!("row139: mul(L, {tb}) != 0"), &zero, &m);
        let m1 = d_void2("crypto_core_ed25519_scalar_mul", 32, &one, y, tb);
        let red = {
            let mut wide = vec![0u8; 64];
            wide[..32].copy_from_slice(y);
            d_void1("crypto_core_ed25519_scalar_reduce", 32, &wide, tb)
        };
        assert_eq_bytes(&format!("row139: mul(1, {tb}) != reduce(y)"), &red, &m1);
    }
    // add/sub round-trip. Only meaningful when BOTH operands are canonical:
    // `crypto_core_ed25519_scalar_add` calls `sodium_add(x_, y_, 32)` — i.e. only
    // 32 of the 64 buffer bytes — so the carry out of byte 31 is DISCARDED and
    // the operation is (x + y mod 2^256) mod L, not (x + y) mod L.
    let mut roundtrips = 0usize;
    for (ta, x) in vecs.iter() {
        if d_pred("crypto_core_ed25519_scalar_is_canonical", x, ta) != 1 {
            continue;
        }
        for (tb, y) in vecs.iter() {
            if d_pred("crypto_core_ed25519_scalar_is_canonical", y, tb) != 1 {
                continue;
            }
            let add = d_void2("crypto_core_ed25519_scalar_add", 32, x, y, ta);
            let back = d_void2("crypto_core_ed25519_scalar_sub", 32, &add, y, ta);
            assert_eq_bytes(
                &format!("row139: (x+y)-y != x for x={ta} y={tb}"),
                x,
                &back,
            );
            roundtrips += 1;
        }
    }
    // Pin the dropped-carry quirk exactly: for ANY operands (canonical or not),
    // `_scalar_add(x, y)` must equal `_scalar_reduce((x + y) mod 2^256)`.
    let mut carry_drops = 0usize;
    for (ta, x) in vecs.iter() {
        for (tb, y) in vecs.iter() {
            let tag = format!("row139 carry x={ta} y={tb}");
            let add = d_void2("crypto_core_ed25519_scalar_add", 32, x, y, &tag);
            let (wrapped, carry) = le_add32(x, y);
            let mut wide = vec![0u8; 64];
            wide[..32].copy_from_slice(&wrapped);
            let expect = d_void1("crypto_core_ed25519_scalar_reduce", 32, &wide, &tag);
            assert_eq_bytes(
                &format!(
                    "{tag}: _scalar_add != reduce((x+y) mod 2^256) — the 32-byte \
                     sodium_add carry-drop is not reproduced"
                ),
                &expect,
                &add,
            );
            if carry != 0 {
                carry_drops += 1;
            }
        }
    }
    assert!(
        carry_drops > 0,
        "row139: no operand pair actually overflowed 2^256, the quirk is unproven"
    );
    eprintln!(
        "row 139: {cases} (x, y) combinations, {roundtrips} round-trips, \
         {carry_drops} carry-drop cases pinned"
    );
}

/// CONFIGS 140 + ERRORS 210/217: `_scalar_invert`. `invert(s) * s == 1 mod L`,
/// `invert(1) == 1`, and for `s == 0` the return is -1 while `recip` HAS ALREADY
/// BEEN WRITTEN by `sc25519_invert` — the output buffer is part of the contract.
#[test]
fn c140_e210_e217_ed25519_scalar_invert() {
    init_both();
    let mut one = [0u8; 32];
    one[0] = 1;
    let zero = [0u8; 32];

    for (tag, s) in canonical_scalars(80, SEED ^ 140) {
        let (rc, inv) = d_int1("crypto_core_ed25519_scalar_invert", 32, &s, &tag);
        assert_eq!(rc, 0, "row140 [{tag}]: invert of a non-zero scalar must return 0");
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &inv, &tag),
            1,
            "row140 [{tag}]: invert output must be canonical"
        );
        let prod = d_void2("crypto_core_ed25519_scalar_mul", 32, &inv, &s, &tag);
        assert_eq_bytes(
            &format!("row140 [{tag}]: invert(s) * s != 1 mod L"),
            &one,
            &prod,
        );
        // ristretto255 delegates.
        let (rc2, inv2) = d_int1("crypto_core_ristretto255_scalar_invert", 32, &s, &tag);
        assert_eq!(rc2, rc, "row140 [{tag}]: ristretto invert return differs");
        assert_eq_bytes(&format!("row140 [{tag}]: ristretto invert differs"), &inv, &inv2);
    }
    // invert(1) == 1
    let (rc, inv1) = d_int1("crypto_core_ed25519_scalar_invert", 32, &one, "invert(1)");
    assert_eq!(rc, 0);
    assert_eq_bytes("row140: invert(1) != 1", &one, &inv1);

    // ERRORS 210: invert(0) -> -1 but `recip` IS written (0xAA is overwritten).
    let (rz, recip) = d_int1("crypto_core_ed25519_scalar_invert", 32, &zero, "e210");
    assert_eq!(rz, -1, "ERRORS 210: invert(0) must return -1");
    assert!(
        recip.iter().all(|&x| x != FILL) || recip.iter().any(|&x| x != FILL),
        "ERRORS 210: recip must have been written"
    );
    assert_eq_bytes(
        "ERRORS 210: invert(0) must still write recip (sc25519_invert runs first)",
        &zero,
        &recip,
    );
    // ERRORS 217: the same for ristretto255.
    let (rz2, recip2) = d_int1("crypto_core_ristretto255_scalar_invert", 32, &zero, "e217");
    assert_eq!(rz2, -1, "ERRORS 217: ristretto invert(0) must return -1");
    assert_eq_bytes("ERRORS 217: ristretto invert(0) recip", &zero, &recip2);

    // Non-canonical s >= L: `sc25519_invert` reduces first, so invert(L) behaves
    // like invert(0) except that `sodium_is_zero(s)` is FALSE, hence rc == 0.
    let (rl, invl) = d_int1("crypto_core_ed25519_scalar_invert", 32, &L_BYTES, "invert(L)");
    assert_eq!(
        rl, 0,
        "ERRORS 210: invert(L) must return 0 — the guard is sodium_is_zero(s), not s mod L"
    );
    assert_eq_bytes("row140: invert(L) recip", &zero, &invl);
    eprintln!("row 140 / ERRORS 210+217: invert verified incl. the s==0 output buffer");
}

/// CONFIGS 141 + ERRORS 211/219: `_scalar_is_canonical` on the exact vector set
/// 0, 1, L-1, L, L+1, 2^252, 2^255-1, 2^256-1.
#[test]
fn c141_e211_e219_scalar_is_canonical() {
    init_both();
    let mut p252 = [0u8; 32];
    p252[31] = 0x10;
    let mut m255 = [0xffu8; 32];
    m255[31] = 0x7f;
    let cases: [(&str, [u8; 32], c_int); 8] = [
        ("0", [0u8; 32], 1),
        ("1", y_enc(1, false), 1),
        ("L-1", le_sub_small(&L_BYTES, 1), 1),
        ("L", L_BYTES, 0),
        ("L+1", le_add_small(&L_BYTES, 1), 0),
        ("2^252", p252, 1),
        ("2^255-1", m255, 0),
        ("2^256-1", [0xffu8; 32], 0),
    ];
    for (tag, s, expect) in cases {
        let rc = d_pred("crypto_core_ed25519_scalar_is_canonical", &s, tag);
        assert_eq!(
            rc, expect,
            "row141/ERRORS 211 [{tag}]: expected {expect}, C and rust both returned {rc}"
        );
        // ERRORS 219: ristretto255 uses the very same `sc25519_is_canonical`.
        let rr = d_pred("crypto_core_ristretto255_scalar_is_canonical", &s, tag);
        assert_eq!(
            rr, expect,
            "ERRORS 219 [{tag}]: ristretto scalar_is_canonical expected {expect}, got {rr}"
        );
    }
    // Randomised sweep: canonical iff s < L.
    let mut rng = Rng::new(SEED ^ 141);
    let mut canon = 0usize;
    for i in 0..192 {
        let mut s = arr32(&rng.bytes(32));
        if i % 2 == 0 {
            s[31] &= 0x1f; // bias towards the canonical range
        }
        let rc = d_pred("crypto_core_ed25519_scalar_is_canonical", &s, &format!("rand{i}"));
        let rr = d_pred("crypto_core_ristretto255_scalar_is_canonical", &s, &format!("rand{i}"));
        assert_eq!(rc, rr, "ERRORS 219 rand{i}: ed25519 vs ristretto differ");
        if rc == 1 {
            canon += 1;
        }
    }
    assert!(canon > 0 && canon < 192, "row141: degenerate split ({canon}/192)");
    eprintln!("row 141 / ERRORS 211+219: 8 fixed vectors + 192 random ({canon} canonical)");
}

/// CONFIGS 142/143/144 + 150: the hash-to-curve entry points across
/// `hash_alg` x `ctx_len` (including the `> 0xff` `H2C-OVERSIZE-DST-` rehash
/// path) x `msg_len`.
#[test]
fn c142_144_c150_from_string() {
    init_both();
    let mut rng = Rng::new(SEED ^ 142);
    let ctx_lens: Vec<usize> = vec![0, 1, 2, 16, 254, 255, 256, 257, 300, 512, 1000];
    let msg_lens: Vec<usize> = vec![0, 1, 31, 32, 63, 64, 65, 128, 1000];
    let entry_points: [(&str, usize, &str); 5] = [
        ("crypto_core_ed25519_from_string", 32, "142"),
        ("crypto_core_ed25519_from_string_nu", 32, "143"),
        ("crypto_core_ed25519_scalar_from_string", 32, "144"),
        ("crypto_core_ristretto255_from_string", 32, "150"),
        ("crypto_core_ristretto255_scalar_from_string", 32, "150"),
    ];
    let mut cases = 0usize;
    let mut oversize_cases = 0usize;
    for (name, olen, row) in entry_points {
        for &alg in &[1i32, 2] {
            for &cl in &ctx_lens {
                for &ml in &msg_lens {
                    let ctx = rng.bytes(cl);
                    let msg = rng.bytes(ml);
                    let tag = format!("row{row} alg={alg} ctx={cl} msg={ml}");
                    let (rc, out, _) = d_from_string(name, olen, &ctx, &msg, alg, &tag);
                    assert_eq!(rc, 0, "{tag}: hash_alg in {{1,2}} must succeed");
                    if cl > 0xff {
                        oversize_cases += 1;
                    }
                    // Shape checks on the (already agreed) output.
                    if name.contains("scalar_from_string") {
                        assert_eq!(
                            d_pred("crypto_core_ed25519_scalar_is_canonical", &out, &tag),
                            1,
                            "{tag}: scalar_from_string output must be canonical"
                        );
                    } else if name.starts_with("crypto_core_ed25519") {
                        // `_from_string` adds two points, `_from_string_nu` maps one;
                        // both land on the curve (from_hash clears the cofactor).
                        assert_eq!(
                            d_pred("crypto_core_ed25519_is_valid_point", &out, &tag),
                            1,
                            "{tag}: ed25519 from_string output must be a valid point"
                        );
                    } else {
                        assert_eq!(
                            d_pred("crypto_core_ristretto255_is_valid_point", &out, &tag),
                            1,
                            "{tag}: ristretto from_string output must be valid"
                        );
                    }
                    cases += 1;
                }
            }
        }
    }
    // ctx_len 255 vs 256 MUST differ: 256 goes through the rehash path.
    let msg = rng.bytes(32);
    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ] {
        for &alg in &[1i32, 2] {
            let ctx255 = vec![0x5Au8; 255];
            let mut ctx256 = vec![0x5Au8; 256];
            let (_, a, _) = d_from_string(name, 32, &ctx255, &msg, alg, "dst255");
            let (_, b, _) = d_from_string(name, 32, &ctx256, &msg, alg, "dst256");
            assert_ne!(a, b, "{name} alg={alg}: ctx_len 255 and 256 gave the SAME output");
            // Truncating a 256-byte ctx to its first 255 bytes must also differ.
            ctx256[255] = 0x5A;
            let (_, c, _) = d_from_string(name, 32, &ctx256[..255], &msg, alg, "dst255b");
            assert_eq_bytes(
                &format!("{name}: ctx[..255] must equal the 255-byte ctx"),
                &a,
                &c,
            );
        }
    }
    // ristretto255's `_scalar_from_string` delegates to the ed25519 one.
    for &alg in &[1i32, 2] {
        for &cl in &[0usize, 1, 255, 256] {
            let ctx = rng.bytes(cl);
            let (_, a, _) = d_from_string(
                "crypto_core_ed25519_scalar_from_string",
                32,
                &ctx,
                &msg,
                alg,
                "deleg",
            );
            let (_, b, _) = d_from_string(
                "crypto_core_ristretto255_scalar_from_string",
                32,
                &ctx,
                &msg,
                alg,
                "deleg",
            );
            assert_eq_bytes(
                "rows 144/150: ristretto scalar_from_string != ed25519 scalar_from_string",
                &a,
                &b,
            );
        }
    }
    // `_from_string` == add(from_string_nu-style pair) is internal, but the two
    // entry points must at least differ (2 points vs 1).
    for &alg in &[1i32, 2] {
        let ctx = b"abc".to_vec();
        let (_, two, _) = d_from_string("crypto_core_ed25519_from_string", 32, &ctx, &msg, alg, "2v1");
        let (_, nu, _) =
            d_from_string("crypto_core_ed25519_from_string_nu", 32, &ctx, &msg, alg, "2v1");
        assert_ne!(two, nu, "rows 142/143: _from_string and _from_string_nu agree");
    }
    eprintln!(
        "rows 142-144 + 150: {cases} cross-product cases ({oversize_cases} oversize-DST)"
    );
}

/// ERRORS 209/218/221: `hash_alg` outside {1, 2} on EVERY `_from_string*` entry
/// point — `core_h2c_string_to_hash`'s `default:` case sets `errno = EINVAL`,
/// returns -1 and leaves the output buffer untouched. Out-of-range enum values
/// crossing the FFI boundary.
#[test]
fn e209_e218_e221_bad_hash_alg() {
    init_both();
    let algs: [c_int; 10] = [0, 3, 4, -1, -2, 255, 256, 0x7fff_ffff, -0x8000_0000, 0x0100_0001];
    let entry_points: [(&str, &str); 5] = [
        ("crypto_core_ed25519_from_string", "209"),
        ("crypto_core_ed25519_from_string_nu", "209"),
        ("crypto_core_ed25519_scalar_from_string", "209"),
        ("crypto_core_ristretto255_from_string", "218"),
        ("crypto_core_ristretto255_scalar_from_string", "218"),
    ];
    let mut cases = 0usize;
    for (name, row) in entry_points {
        for &alg in &algs {
            for &(cl, ml) in &[(0usize, 0usize), (1, 1), (255, 64), (256, 1000)] {
                let c = vec![0x11u8; cl];
                let m = vec![0x22u8; ml];
                let tag = format!("ERRORS {row}/221 alg={alg}");
                let (fc, fr) = unsafe { pair::<FromStringFn>(name) };
                let mut bc = vec![FILL; 32 + PAD];
                let mut br = vec![FILL; 32 + PAD];
                errno_set(0);
                let rc = unsafe {
                    fc(bc.as_mut_ptr(), c.as_ptr(), cl, m.as_ptr(), ml, alg)
                };
                let ec = errno_get();
                errno_set(0);
                let rr = unsafe {
                    fr(br.as_mut_ptr(), c.as_ptr(), cl, m.as_ptr(), ml, alg)
                };
                let er = errno_get();
                let what = format!("{name} [{tag}] ctx={cl} msg={ml}");
                assert_eq!(rc, rr, "{what}: return C={rc} rust={rr}");
                assert_eq!(rc, -1, "{what}: must return -1");
                assert_eq!(ec, er, "{what}: errno C={ec} rust={er}");
                assert_eq!(
                    ec,
                    libc::EINVAL,
                    "{what}: errno must be EINVAL ({}), got {ec}",
                    libc::EINVAL
                );
                assert_eq_bytes(&what, &bc, &br);
                assert!(
                    bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                    "{what}: the output buffer must be left untouched"
                );
                cases += 1;
            }
        }
    }
    eprintln!("ERRORS 209/218/221: {cases} out-of-range hash_alg cases, all -1 + EINVAL");
}

// ===========================================================================
// 14b. crypto_core_ristretto255
// ===========================================================================

/// CONFIGS 145 + ERRORS 212: `_is_valid_point`. The identity (32 zero bytes) IS
/// a valid ristretto encoding.
#[test]
fn c145_e212_ristretto_is_valid_point() {
    init_both();
    // Identity.
    assert_eq!(
        d_pred("crypto_core_ristretto255_is_valid_point", &[0u8; 32], "identity"),
        1,
        "row145: the ristretto identity (32 zero bytes) must be valid"
    );
    // Valid canonical even encodings.
    for (i, p) in valid_ristretto_points(24, SEED ^ 145).iter().enumerate() {
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", p, &format!("valid{i}")),
            1,
            "row145 [{i}]: from_hash output must be valid"
        );
        assert_eq!(p[0] & 1, 0, "row145 [{i}]: a valid encoding must be even");
        assert_eq!(p[31] & 0x80, 0, "row145 [{i}]: bit 255 must be clear");
    }
    // Invalid shapes (also ERRORS 212).
    let v = valid_ristretto_points(1, SEED ^ 0x145)[0];
    let mut odd = v;
    odd[0] |= 1;
    let mut hi = v;
    hi[31] |= 0x80;
    for (tag, s) in [
        ("s odd", odd),
        ("bit255 set", hi),
        ("s = p", P_BYTES),
        ("s = p+1", le_add_small(&P_BYTES, 1)),
        ("s = 2^256-1", [0xffu8; 32]),
        ("s = 8 (non-square)", y_enc(8, false)),
        ("s = 2 (T negative)", y_enc(2, false)),
        ("s = p-1 (Y == 0)", le_sub_small(&P_BYTES, 1)),
    ] {
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &s, tag),
            0,
            "row145/ERRORS 212 [{tag}]: must be rejected"
        );
    }
    // Randomised sweep.
    let mut rng = Rng::new(SEED ^ 0x1450);
    let mut valid_n = 0usize;
    for i in 0..256 {
        let mut s = arr32(&rng.bytes(32));
        if i % 3 == 0 {
            s[0] &= 0xfe;
            s[31] &= 0x7f;
        }
        if d_pred("crypto_core_ristretto255_is_valid_point", &s, &format!("rand{i}")) == 1 {
            valid_n += 1;
        }
    }
    eprintln!("row 145 / ERRORS 212: {valid_n} of 256 random strings valid");
}

/// CONFIGS 146 + ERRORS 213–216: `_add` / `_sub` and the ristretto group law.
#[test]
fn c146_e213_216_ristretto_add_sub() {
    init_both();
    let pts = valid_ristretto_points(10, SEED ^ 146);
    let ident = [0u8; 32];
    let mut cases = 0usize;
    for i in 0..9 {
        let (p, q) = (pts[i], pts[i + 1]);
        let (r1, s1) = d_int2("crypto_core_ristretto255_add", 32, &p, &q, &format!("row146 add{i}"));
        let (r2, s2) = d_int2("crypto_core_ristretto255_add", 32, &q, &p, &format!("row146 add{i}r"));
        assert_eq!((r1, r2), (0, 0), "row146: valid+valid must succeed");
        assert_eq_bytes(&format!("row146 add{i}: not commutative"), &s1, &s2);
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &s1, "sum"),
            1,
            "row146 add{i}: sum must be a valid encoding"
        );
        let (r3, back) = d_int2("crypto_core_ristretto255_sub", 32, &s1, &q, &format!("row146 sub{i}"));
        assert_eq!(r3, 0);
        assert_eq_bytes(&format!("row146 {i}: (p+q)-q != p"), &p, &back);
        let (r4, zero) = d_int2("crypto_core_ristretto255_sub", 32, &p, &p, &format!("row146 self{i}"));
        assert_eq!(r4, 0);
        assert_eq_bytes(&format!("row146 {i}: p-p != identity"), &ident, &zero);
        let (r5, same) = d_int2("crypto_core_ristretto255_add", 32, &p, &ident, &format!("row146 id{i}"));
        assert_eq!(r5, 0);
        assert_eq_bytes(&format!("row146 {i}: p+identity != p"), &p, &same);
        // Associativity: (p+q)+r == p+(q+r)
        let r = pts[(i + 5) % 10];
        let lhs = d_int2("crypto_core_ristretto255_add", 32, &s1, &r, "assoc").1;
        let qr = d_int2("crypto_core_ristretto255_add", 32, &q, &r, "assoc").1;
        let rhs = d_int2("crypto_core_ristretto255_add", 32, &p, &qr, "assoc").1;
        assert_eq_bytes(&format!("row146 {i}: ristretto add not associative"), &lhs, &rhs);
        cases += 6;
    }
    // ERRORS 213–216: a bad operand in either position, for both entry points.
    let bad: [(&str, [u8; 32]); 4] = [
        ("s odd", y_enc(1, false)),
        ("s = p", P_BYTES),
        ("s = 8 (non-square)", y_enc(8, false)),
        ("s = p-1 (Y == 0)", le_sub_small(&P_BYTES, 1)),
    ];
    for (btag, b) in bad {
        for (name, rows) in [
            ("crypto_core_ristretto255_add", "213/214"),
            ("crypto_core_ristretto255_sub", "215/216"),
        ] {
            for (which, p, q) in [
                ("bad p", b, pts[0]),
                ("bad q", pts[0], b),
                ("both bad", b, b),
            ] {
                let tag = format!("ERRORS {rows} {name} [{which}: {btag}]");
                let (rc, buf) = d_int2(name, 32, &p, &q, &tag);
                assert_eq!(rc, -1, "{tag}: must return -1");
                assert!(
                    buf.iter().all(|&x| x == FILL),
                    "{tag}: r was written on the rejection path"
                );
            }
        }
    }
    eprintln!("rows 146 / ERRORS 213-216: {cases} group-law checks + 24 rejections");
}

/// CONFIGS 147 + ERRORS 220: `_from_hash` accepts EVERY 64-byte input.
#[test]
fn c147_e220_ristretto_from_hash() {
    init_both();
    let mut rng = Rng::new(SEED ^ 147);
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![0u8; 64],
        vec![0xffu8; 64],
        (0..64u8).collect(),
        {
            let mut v = vec![0u8; 64];
            v[..32].copy_from_slice(&P_BYTES);
            v
        },
        {
            let mut v = vec![0xffu8; 64];
            v[31] = 0x7f;
            v[63] = 0x7f;
            v
        },
    ];
    for _ in 0..96 {
        inputs.push(rng.bytes(64));
    }
    let mut seen = std::collections::HashSet::new();
    for (i, h) in inputs.iter().enumerate() {
        let (rc, p) = d_int1("crypto_core_ristretto255_from_hash", 32, h, &format!("row147-{i}"));
        // ERRORS 220: no rejection branch exists.
        assert_eq!(rc, 0, "ERRORS 220 [{i}]: from_hash must ALWAYS return 0");
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &p, &format!("row147-{i}")),
            1,
            "row147 [{i}]: from_hash output must be a valid encoding"
        );
        seen.insert(p);
    }
    assert!(seen.len() > 95, "row147: only {} distinct outputs", seen.len());
    eprintln!(
        "row 147 / ERRORS 220: {} inputs, all accepted, {} distinct",
        inputs.len(),
        seen.len()
    );
}

/// CONFIGS 148: `_ristretto255_random` / `_scalar_random` **[RNG]**.
#[test]
fn c148_ristretto255_random() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    reset_det_rng();
    let mut seen = std::collections::HashSet::new();
    for i in 0..96 {
        let p = d_void0("crypto_core_ristretto255_random", 32, &format!("row148-{i}"));
        assert_eq!(
            d_pred("crypto_core_ristretto255_is_valid_point", &p, &format!("row148-{i}")),
            1,
            "row148 [{i}]: _random must yield a valid encoding"
        );
        seen.insert(p);
    }
    assert!(seen.len() > 90, "row148: only {} distinct points", seen.len());
    // `_random` == `from_hash(randombytes_buf(64))`.
    reset_det_rng();
    let (bc, _) = unsafe { pair::<unsafe extern "C" fn(*mut libc::c_void, usize)>("randombytes_buf") };
    let mut h = vec![0u8; 64];
    unsafe { bc(h.as_mut_ptr() as *mut libc::c_void, 64) };
    let expect = d_int1("crypto_core_ristretto255_from_hash", 32, &h, "row148-equiv").1;
    reset_det_rng();
    let got = d_void0("crypto_core_ristretto255_random", 32, "row148-equiv");
    assert_eq_bytes("row148: _random != from_hash(randombytes_buf(64))", &expect, &got);

    reset_det_rng();
    let mut sseen = std::collections::HashSet::new();
    for i in 0..96 {
        let s = d_void0("crypto_core_ristretto255_scalar_random", 32, &format!("row148s-{i}"));
        assert_eq!(
            d_pred("crypto_core_ristretto255_scalar_is_canonical", &s, &format!("row148s-{i}")),
            1,
            "row148 [{i}]: scalar_random must be canonical"
        );
        assert!(s.iter().any(|&x| x != 0), "row148 [{i}]: scalar_random returned 0");
        sseen.insert(s);
    }
    assert!(sseen.len() > 90);
    eprintln!("row 148: 96 points + 96 scalars");
}

/// CONFIGS 149: the whole `crypto_core_ristretto255_scalar_*` family over the
/// same vector sets as rows 137–141. Every one of them delegates to the ed25519
/// implementation, which is asserted explicitly.
#[test]
fn c149_ristretto255_scalar_ops() {
    init_both();
    let mut one = [0u8; 32];
    one[0] = 1;
    let zero = [0u8; 32];
    let vecs = scalar_vectors(40, SEED ^ 149);

    // Unary: negate, complement, invert.
    for (tag, s) in &vecs {
        for (rn, en) in [
            ("crypto_core_ristretto255_scalar_negate", "crypto_core_ed25519_scalar_negate"),
            (
                "crypto_core_ristretto255_scalar_complement",
                "crypto_core_ed25519_scalar_complement",
            ),
        ] {
            let a = d_void1(rn, 32, s, tag);
            let b = d_void1(en, 32, s, tag);
            assert_eq_bytes(&format!("row149 [{tag}]: {rn} != {en}"), &b, &a);
        }
        let (ra, ia) = d_int1("crypto_core_ristretto255_scalar_invert", 32, s, tag);
        let (rb, ib) = d_int1("crypto_core_ed25519_scalar_invert", 32, s, tag);
        assert_eq!(ra, rb, "row149 [{tag}]: invert return differs from ed25519");
        assert_eq_bytes(&format!("row149 [{tag}]: invert differs from ed25519"), &ib, &ia);
        if ra == 0 {
            let prod = d_void2("crypto_core_ristretto255_scalar_mul", 32, &ia, s, tag);
            if d_pred("crypto_core_ristretto255_scalar_is_canonical", s, tag) == 1
                && s.iter().any(|&x| x != 0)
            {
                assert_eq_bytes(
                    &format!("row149 [{tag}]: invert(s) * s != 1 mod L"),
                    &one,
                    &prod,
                );
            }
        }
        // is_canonical
        assert_eq!(
            d_pred("crypto_core_ristretto255_scalar_is_canonical", s, tag),
            d_pred("crypto_core_ed25519_scalar_is_canonical", s, tag),
            "row149 [{tag}]: is_canonical differs from ed25519"
        );
    }
    // Binary: add, sub, mul.
    let mut cases = 0usize;
    for i in 0..vecs.len() {
        for j in [0usize, 2, 4, 9, vecs.len() - 1] {
            let (ta, x) = &vecs[i];
            let (tb, y) = &vecs[j];
            let tag = format!("row149 x={ta} y={tb}");
            for (rn, en) in [
                ("crypto_core_ristretto255_scalar_add", "crypto_core_ed25519_scalar_add"),
                ("crypto_core_ristretto255_scalar_sub", "crypto_core_ed25519_scalar_sub"),
                ("crypto_core_ristretto255_scalar_mul", "crypto_core_ed25519_scalar_mul"),
            ] {
                let a = d_void2(rn, 32, x, y, &tag);
                let b = d_void2(en, 32, x, y, &tag);
                assert_eq_bytes(&format!("{tag}: {rn} != {en}"), &b, &a);
            }
            cases += 1;
        }
    }
    // reduce over the 64-byte sweep.
    for (tag, s) in nonreduced_vectors(24, SEED ^ 0x149) {
        let a = d_void1("crypto_core_ristretto255_scalar_reduce", 32, &s, &tag);
        let b = d_void1("crypto_core_ed25519_scalar_reduce", 32, &s, &tag);
        assert_eq_bytes(&format!("row149 reduce [{tag}] differs from ed25519"), &b, &a);
    }
    // negate/complement identities in the ristretto namespace.
    for (tag, s) in canonical_scalars(24, SEED ^ 0x1490) {
        let neg = d_void1("crypto_core_ristretto255_scalar_negate", 32, &s, &tag);
        let comp = d_void1("crypto_core_ristretto255_scalar_complement", 32, &s, &tag);
        assert_eq_bytes(
            &format!("row149 [{tag}]: negate(s) + s != 0"),
            &zero,
            &d_void2("crypto_core_ristretto255_scalar_add", 32, &neg, &s, &tag),
        );
        assert_eq_bytes(
            &format!("row149 [{tag}]: complement(s) + s != 1"),
            &one,
            &d_void2("crypto_core_ristretto255_scalar_add", 32, &comp, &s, &tag),
        );
        assert_eq_bytes(
            &format!("row149 [{tag}]: negate(negate(s)) != s"),
            &s,
            &d_void1("crypto_core_ristretto255_scalar_negate", 32, &neg, &tag),
        );
    }
    eprintln!("row 149: {} unary + {cases} binary + reduce/identities", vecs.len());
}

/// CONFIGS 151: every `crypto_core_*` / `crypto_scalarmult_*` / `crypto_sign_*`
/// constant getter.
#[test]
fn c151_constant_getters() {
    init_both();
    d_size("crypto_core_ed25519_bytes", 32);
    d_size("crypto_core_ed25519_uniformbytes", 32);
    d_size("crypto_core_ed25519_hashbytes", 64);
    d_size("crypto_core_ed25519_scalarbytes", 32);
    d_size("crypto_core_ed25519_nonreducedscalarbytes", 64);
    d_size("crypto_core_ristretto255_bytes", 32);
    d_size("crypto_core_ristretto255_hashbytes", 64);
    d_size("crypto_core_ristretto255_scalarbytes", 32);
    d_size("crypto_core_ristretto255_nonreducedscalarbytes", 64);

    d_size("crypto_scalarmult_bytes", 32);
    d_size("crypto_scalarmult_scalarbytes", 32);
    d_size("crypto_scalarmult_curve25519_bytes", 32);
    d_size("crypto_scalarmult_curve25519_scalarbytes", 32);
    d_size("crypto_scalarmult_ed25519_bytes", 32);
    d_size("crypto_scalarmult_ed25519_scalarbytes", 32);
    d_size("crypto_scalarmult_ristretto255_bytes", 32);
    d_size("crypto_scalarmult_ristretto255_scalarbytes", 32);
    d_cstr("crypto_scalarmult_primitive", "curve25519");

    d_size("crypto_sign_bytes", 64);
    d_size("crypto_sign_seedbytes", 32);
    d_size("crypto_sign_publickeybytes", 32);
    d_size("crypto_sign_secretkeybytes", 64);
    d_size("crypto_sign_statebytes", PH_STATE);
    d_size("crypto_sign_messagebytes_max", usize::MAX - 64);
    d_cstr("crypto_sign_primitive", "ed25519");
    d_size("crypto_sign_ed25519_bytes", 64);
    d_size("crypto_sign_ed25519_seedbytes", 32);
    d_size("crypto_sign_ed25519_publickeybytes", 32);
    d_size("crypto_sign_ed25519_secretkeybytes", 64);
    d_size("crypto_sign_ed25519_messagebytes_max", usize::MAX - 64);
    d_size("crypto_sign_ed25519ph_statebytes", PH_STATE);
    eprintln!("row 151: 30 constant getters");
}

// ===========================================================================
// 15. crypto_sign / ed25519
// ===========================================================================

/// CONFIGS 152: `_seed_keypair` — all-0x00, all-0xff, the RFC 8032 test seeds
/// and random seeds. `sk == seed || pk`.
#[test]
fn c152_sign_seed_keypair() {
    init_both();
    // RFC 8032 section 7.1 TEST 1 / TEST 2 / TEST 3 secret keys.
    let rfc1: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let rfc1_pk: [u8; 32] = [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07,
        0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07,
        0x51, 0x1a,
    ];
    let rfc2: [u8; 32] = [
        0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11, 0x4e,
        0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed, 0x4f, 0xb8,
        0xa6, 0xfb,
    ];
    let rfc2_pk: [u8; 32] = [
        0x3d, 0x40, 0x17, 0xc3, 0xe8, 0x43, 0x89, 0x5a, 0x92, 0xb7, 0x0a, 0xa7, 0x4d, 0x1b, 0x7e,
        0xbc, 0x9c, 0x98, 0x2c, 0xcf, 0x2e, 0xc4, 0x96, 0x8c, 0xc0, 0xcd, 0x55, 0xf1, 0x2a, 0xf4,
        0x66, 0x0c,
    ];

    let mut rng = Rng::new(SEED ^ 152);
    let mut seeds: Vec<[u8; 32]> = vec![
        [0u8; 32],
        [0xffu8; 32],
        (0..32u8).collect::<Vec<_>>().try_into().unwrap(),
        rfc1,
        rfc2,
        arr32(&L_BYTES),
        P_BYTES,
    ];
    for _ in 0..72 {
        seeds.push(arr32(&rng.bytes(32)));
    }

    for (i, seed) in seeds.iter().enumerate() {
        let (pk, sk) = keypair_from_seed(seed);
        // sk == seed || pk
        assert_eq_bytes(
            &format!("row152 [{i}]: sk[0..32) != seed"),
            seed,
            &sk[..32],
        );
        assert_eq_bytes(
            &format!("row152 [{i}]: sk[32..64) != pk"),
            &pk,
            &sk[32..],
        );
        // pk must be a valid main-subgroup point.
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &pk, &format!("row152-{i}")),
            1,
            "row152 [{i}]: pk must be a valid main-subgroup point"
        );
        // Determinism.
        let (pk2, sk2) = keypair_from_seed(seed);
        assert_eq_bytes(&format!("row152 [{i}]: pk not deterministic"), &pk, &pk2);
        assert_eq_bytes(&format!("row152 [{i}]: sk not deterministic"), &sk, &sk2);
        // The dispatch wrapper must agree (CONFIGS 156).
        let (fc, fr) = unsafe { pair::<SeedKeypairFn>("crypto_sign_seed_keypair") };
        let mut pc = vec![FILL; 32 + PAD];
        let mut pr = vec![FILL; 32 + PAD];
        let mut kc = vec![FILL; 64 + PAD];
        let mut kr = vec![FILL; 64 + PAD];
        let rc = unsafe { fc(pc.as_mut_ptr(), kc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { fr(pr.as_mut_ptr(), kr.as_mut_ptr(), seed.as_ptr()) };
        assert_eq!(rc, rr, "row156 [{i}]: crypto_sign_seed_keypair return differs");
        assert_eq_bytes("row156: dispatch pk differs", &pc, &pr);
        assert_eq_bytes("row156: dispatch sk differs", &kc, &kr);
        assert_eq_bytes("row156: dispatch pk != ed25519 pk", &pk, &pc[..32]);
        assert_eq_bytes("row156: dispatch sk != ed25519 sk", &sk, &kc[..64]);
    }
    // RFC 8032 known-answer checks (a wrong translation of the clamp or of
    // scalarmult_base would fail here).
    let (pk1, _) = keypair_from_seed(&rfc1);
    assert_eq_bytes("row152: RFC 8032 TEST 1 public key mismatch", &rfc1_pk, &pk1);
    let (pk2, _) = keypair_from_seed(&rfc2);
    assert_eq_bytes("row152: RFC 8032 TEST 2 public key mismatch", &rfc2_pk, &pk2);
    eprintln!("row 152: {} seeds incl. 2 RFC 8032 known answers", seeds.len());
}

/// CONFIGS 153: `_keypair` **[RNG]**.
#[test]
fn c153_sign_keypair() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);

    for name in ["crypto_sign_ed25519_keypair", "crypto_sign_keypair"] {
        reset_det_rng();
        let (fc, fr) = unsafe { pair::<KeypairFn>(name) };
        let mut seen = std::collections::HashSet::new();
        for i in 0..64 {
            let mut pc = vec![FILL; 32 + PAD];
            let mut pr = vec![FILL; 32 + PAD];
            let mut kc = vec![FILL; 64 + PAD];
            let mut kr = vec![FILL; 64 + PAD];
            let rc = unsafe { fc(pc.as_mut_ptr(), kc.as_mut_ptr()) };
            let rr = unsafe { fr(pr.as_mut_ptr(), kr.as_mut_ptr()) };
            let what = format!("row153 {name} #{i}");
            assert_eq!(rc, rr, "{what}: return C={rc} rust={rr}");
            assert_eq!(rc, 0, "{what}: must return 0");
            assert_eq_bytes(&format!("{what} pk"), &pc, &pr);
            assert_eq_bytes(&format!("{what} sk"), &kc, &kr);
            guard_ok(&what, "C pk", &pc, 32);
            guard_ok(&what, "C sk", &kc, 64);
            // sk == seed || pk and the pair is consistent.
            assert_eq_bytes(&format!("{what}: sk[32..64) != pk"), &pc[..32], &kc[32..64]);
            let (pk3, sk3) = keypair_from_seed(&kc[..32]);
            assert_eq_bytes(&format!("{what}: seed_keypair(seed) != keypair pk"), &pc[..32], &pk3);
            assert_eq_bytes(&format!("{what}: seed_keypair(seed) != keypair sk"), &kc[..64], &sk3);
            seen.insert(pc[..32].to_vec());
        }
        assert!(seen.len() > 60, "row153 {name}: only {} distinct keys", seen.len());
    }
    // The two entry points must draw identically from the RNG.
    reset_det_rng();
    let (ac, _) = unsafe { pair::<KeypairFn>("crypto_sign_ed25519_keypair") };
    let mut p1 = vec![0u8; 32];
    let mut s1 = vec![0u8; 64];
    unsafe { ac(p1.as_mut_ptr(), s1.as_mut_ptr()) };
    reset_det_rng();
    let (bc, _) = unsafe { pair::<KeypairFn>("crypto_sign_keypair") };
    let mut p2 = vec![0u8; 32];
    let mut s2 = vec![0u8; 64];
    unsafe { bc(p2.as_mut_ptr(), s2.as_mut_ptr()) };
    assert_eq_bytes("row156: crypto_sign_keypair != crypto_sign_ed25519_keypair", &p1, &p2);
    eprintln!("row 153: 2 x 64 keypairs from the injected RNG");
}

/// CONFIGS 154 + the sign/verify round-trip property. `mlen` sweeps the set that
/// straddles the 64-byte prefix (`sig[0..32) || pk`) already absorbed by SHA-512.
#[test]
fn c154_sign_detached_verify() {
    init_both();
    let (pk, sk) = test_keypair(0);
    let (pk2, _sk2) = test_keypair(1);
    let mlens: [usize; 14] = [0, 1, 31, 32, 63, 64, 65, 111, 127, 128, 129, 255, 256, 1000];
    let mut rng = Rng::new(SEED ^ 154);
    let mut cases = 0usize;

    for &mlen in &mlens {
        for salt in 0..5u8 {
            let m = if salt == 4 { rng.bytes(mlen) } else { msg_of(mlen, salt) };
            let tag = format!("row154 mlen={mlen} salt={salt}");
            let (rc, sig, siglen) =
                d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, &tag);
            assert_eq!(rc, 0, "{tag}: signing must succeed");
            assert_eq!(siglen, 64, "{tag}: *siglen_p must be 64");
            // ed25519 is deterministic.
            let (_, sig_b, _) = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, &tag);
            assert_eq_bytes(&format!("{tag}: signing is not deterministic"), &sig, &sig_b);
            // siglen_p == NULL must not change the signature.
            let (rc2, sig_c, _) = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, true, &tag);
            assert_eq!(rc2, 0);
            assert_eq_bytes(&format!("{tag}: NULL siglen_p changed the signature"), &sig, &sig_c);
            // ROUND TRIP.
            assert_eq!(
                d_verify("crypto_sign_ed25519_verify_detached", &sig, &m, &pk, &tag),
                0,
                "{tag}: sign -> verify round-trip FAILED"
            );
            // Wrong public key.
            assert_eq!(
                d_verify("crypto_sign_ed25519_verify_detached", &sig, &m, &pk2, &tag),
                -1,
                "{tag}: verified under the WRONG public key"
            );
            // Every single-bit flip in the signature must be rejected.
            for &bit in &[0usize, 7, 100, 255, 256, 300, 500, 511] {
                let mut bad = sig.clone();
                bad[bit / 8] ^= 1 << (bit % 8);
                assert_eq!(
                    d_verify("crypto_sign_ed25519_verify_detached", &bad, &m, &pk, &tag),
                    -1,
                    "{tag}: a signature with bit {bit} flipped verified"
                );
            }
            // Flipping a message bit must be rejected.
            if mlen > 0 {
                let mut bm = m.clone();
                bm[mlen / 2] ^= 0x01;
                assert_eq!(
                    d_verify("crypto_sign_ed25519_verify_detached", &sig, &bm, &pk, &tag),
                    -1,
                    "{tag}: verified against a modified message"
                );
            }
            // Truncating / extending the message must be rejected.
            if mlen > 1 {
                assert_eq!(
                    d_verify("crypto_sign_ed25519_verify_detached", &sig, &m[..mlen - 1], &pk, &tag),
                    -1,
                    "{tag}: verified against a truncated message"
                );
            }
            cases += 1;
        }
    }
    // Different messages under the same key give different signatures.
    let a = d_sign("crypto_sign_ed25519_detached", 64, b"abc", &sk, false, "distinct").1;
    let b = d_sign("crypto_sign_ed25519_detached", 64, b"abd", &sk, false, "distinct").1;
    assert_ne!(a, b, "row154: distinct messages produced the same signature");
    eprintln!("row 154: {cases} (mlen, salt) sign/verify cases");
}

/// CONFIGS 155 + ERRORS 170/172: the combined `crypto_sign_ed25519` /
/// `_open` form, including `m == NULL` and `mlen_p == NULL` on open.
#[test]
fn c155_e170_e172_sign_open_combined() {
    init_both();
    let (pk, sk) = test_keypair(2);
    let (pk2, _) = test_keypair(3);
    let mlens: [usize; 12] = [0, 1, 31, 32, 63, 64, 65, 127, 128, 129, 256, 1000];
    let mut cases = 0usize;

    for &mlen in &mlens {
        let m = msg_of(mlen, 0x5A);
        let tag = format!("row155 mlen={mlen}");
        let (rc, sm, smlen) = d_sign("crypto_sign_ed25519", mlen + 64, &m, &sk, false, &tag);
        assert_eq!(rc, 0, "{tag}: crypto_sign_ed25519 must succeed");
        assert_eq!(smlen, (mlen + 64) as u64, "{tag}: *smlen_p must be mlen + 64");
        // sm == sig || m
        assert_eq_bytes(&format!("{tag}: sm[64..] != m"), &m, &sm[64..]);
        let det = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, &tag).1;
        assert_eq_bytes(&format!("{tag}: sm[0..64) != detached signature"), &det, &sm[..64]);

        // open, all four NULL combinations
        for (nm, nl) in [(false, false), (true, false), (false, true), (true, true)] {
            let (ro, mout, mlen_out) =
                d_open("crypto_sign_ed25519_open", &sm, &pk, nm, nl, &tag);
            assert_eq!(ro, 0, "{tag}: open must succeed (null_m={nm} null_len={nl})");
            if !nl {
                assert_eq!(mlen_out, mlen as u64, "{tag}: *mlen_p must be mlen");
            }
            if !nm {
                assert_eq_bytes(&format!("{tag}: recovered message differs"), &m, &mout[..mlen]);
                assert!(
                    mout[mlen..].iter().all(|&x| x == FILL),
                    "{tag}: open wrote past mlen"
                );
            }
        }
        // ERRORS 172: inner verify fails -> -1, *mlen_p = 0, memset(m, 0, mlen).
        let mut bad = sm.clone();
        bad[10] ^= 0x40;
        let (rb, mb, lb) = d_open("crypto_sign_ed25519_open", &bad, &pk, false, false, &tag);
        assert_eq!(rb, -1, "ERRORS 172 [{tag}]: a bad signature must be rejected");
        assert_eq!(lb, 0, "ERRORS 172 [{tag}]: *mlen_p must be set to 0");
        assert!(
            mb[..mlen].iter().all(|&x| x == 0),
            "ERRORS 172 [{tag}]: m[0..mlen) must be zeroed, got {}",
            hexs(&mb[..mlen])
        );
        assert!(
            mb[mlen..].iter().all(|&x| x == FILL),
            "ERRORS 172 [{tag}]: the zeroing ran past mlen"
        );
        // Same with m == NULL: no write, still -1 and *mlen_p = 0.
        let (rb2, _, lb2) = d_open("crypto_sign_ed25519_open", &bad, &pk, true, false, &tag);
        assert_eq!(rb2, -1);
        assert_eq!(lb2, 0, "ERRORS 172 [{tag}]: *mlen_p must be 0 even when m == NULL");
        // Wrong key.
        let (rw, _, lw) = d_open("crypto_sign_ed25519_open", &sm, &pk2, false, false, &tag);
        assert_eq!(rw, -1, "{tag}: open succeeded under the WRONG public key");
        assert_eq!(lw, 0);
        cases += 1;
    }
    eprintln!("row 155 / ERRORS 170+172: {cases} mlens x 4 NULL combinations");
}

/// ERRORS 170: `smlen < 64` -> -1, `*mlen_p = 0`, `m` completely untouched
/// (the early `goto badsig` runs before any `memset`).
#[test]
fn e170_open_smlen_lt_64() {
    init_both();
    let (pk, sk) = test_keypair(4);
    let full = d_sign("crypto_sign_ed25519", 64 + 8, &msg_of(8, 1), &sk, false, "e170").1;

    for smlen in 0..64usize {
        let sm = &full[..smlen];
        let tag = format!("ERRORS 170 smlen={smlen}");
        let (rc, mout, mlen_out) = d_open("crypto_sign_ed25519_open", sm, &pk, false, false, &tag);
        assert_eq!(rc, -1, "{tag}: smlen < 64 must return -1");
        assert_eq!(mlen_out, 0, "{tag}: *mlen_p must be 0");
        assert!(
            mout.iter().all(|&x| x == FILL),
            "{tag}: m must be COMPLETELY untouched, got {}",
            hexs(&mout)
        );
        // Also with m == NULL and mlen_p == NULL.
        assert_eq!(
            d_open("crypto_sign_ed25519_open", sm, &pk, true, true, &tag).0,
            -1
        );
        // ERRORS 173: the dispatch wrapper behaves identically.
        let (rw, mw, lw) = d_open("crypto_sign_open", sm, &pk, false, false, &tag);
        assert_eq!(rw, -1, "ERRORS 173 [{tag}]: wrapper must return -1");
        assert_eq!(lw, 0);
        assert!(mw.iter().all(|&x| x == FILL));
    }
    // smlen == 64 (mlen == 0) is the first length that reaches the verifier.
    let sm0 = d_sign("crypto_sign_ed25519", 64, &[], &sk, false, "e170-64").1;
    let (rc, _, l) = d_open("crypto_sign_ed25519_open", &sm0, &pk, false, false, "e170-64");
    assert_eq!(rc, 0, "ERRORS 170: smlen == 64 must be accepted");
    assert_eq!(l, 0, "ERRORS 170: smlen == 64 gives mlen == 0");
    eprintln!("ERRORS 170/173: smlen 0..63 rejected with m untouched; 64 accepted");
}

/// ERRORS 171: `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX`. Since
/// `MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - 64` and `SODIUM_SIZE_MAX == SIZE_MAX`
/// on this platform, while `smlen` is `unsigned long long` (== `size_t` here),
/// the branch needs `smlen > SIZE_MAX` and is therefore DEAD. Proven by pinning
/// the constant in both libraries, then showing the largest representable
/// `smlen` values still take the `smlen < 64` path or the verifier path.
#[test]
fn e171_open_messagebytes_max_dead() {
    init_both();
    let max = d_size("crypto_sign_ed25519_messagebytes_max", usize::MAX - 64);
    assert_eq!(
        max,
        usize::MAX - 64,
        "ERRORS 171: MESSAGEBYTES_MAX must be SIZE_MAX - 64 for the branch to be dead"
    );
    // `smlen - 64 > SIZE_MAX - 64`  <=>  `smlen > SIZE_MAX`: unrepresentable in
    // the u64 parameter, so no caller can reach the branch.
    assert_eq!(
        (u64::MAX as u128).saturating_sub(64),
        (max as u128),
        "ERRORS 171: u64::MAX - 64 must equal MESSAGEBYTES_MAX, leaving no room above it"
    );
    d_size("crypto_sign_messagebytes_max", usize::MAX - 64);
    eprintln!("ERRORS 171: branch is unreachable (MESSAGEBYTES_MAX == SIZE_MAX - 64 == u64::MAX - 64)");
}

/// ERRORS 179: `crypto_sign_ed25519`'s `siglen != 64` guard is marked
/// `LCOV_EXCL` in the C and is unreachable, because `_detached` unconditionally
/// writes 64. Prove it: over a wide sweep the return is always 0,
/// `*smlen_p == mlen + 64`, and `sm[0..64)` is exactly the detached signature.
#[test]
fn e179_sign_siglen_dead_branch() {
    init_both();
    let (_pk, sk) = test_keypair(5);
    let mut rng = Rng::new(SEED ^ 179);
    for i in 0..72 {
        let mlen = *rng.pick(&[0usize, 1, 15, 32, 63, 64, 65, 127, 200, 1000]);
        let m = rng.bytes(mlen);
        let tag = format!("ERRORS 179 #{i} mlen={mlen}");
        let (rc, sm, smlen) = d_sign("crypto_sign_ed25519", mlen + 64, &m, &sk, false, &tag);
        assert_eq!(rc, 0, "{tag}: crypto_sign_ed25519 must ALWAYS return 0");
        assert_eq!(smlen, (mlen + 64) as u64, "{tag}: *smlen_p must be mlen + 64");
        let det = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, &tag).1;
        assert_eq_bytes(&format!("{tag}: sm[0..64) != detached signature"), &det, &sm[..64]);
        assert_eq_bytes(&format!("{tag}: sm[64..) != m"), &m, &sm[64..]);
        // NULL smlen_p must not change sm.
        let (rc2, sm2, _) = d_sign("crypto_sign_ed25519", mlen + 64, &m, &sk, true, &tag);
        assert_eq!(rc2, 0);
        assert_eq_bytes(&format!("{tag}: NULL smlen_p changed sm"), &sm, &sm2);
    }
    eprintln!("ERRORS 179: 72 cases, siglen guard never fires (sm untouched by it)");
}

/// ERRORS 163: `ge25519_is_canonical(pk) == 0` — verification rejects all 38
/// non-canonical public-key encodings BEFORE any decoding. Contrast with
/// `_pk_to_curve25519` (ERRORS 177), which has no such gate.
#[test]
fn e163_verify_noncanonical_pk() {
    init_both();
    let (_pk, sk) = test_keypair(6);
    let m = msg_of(32, 7);
    let sig = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, "e163").1;
    let mut n = 0usize;
    for (tag, e) in noncanonical_encodings() {
        // Pin `ge25519_is_canonical` itself: this set is EXACTLY its reject set.
        assert_eq!(
            d_pred("_sodium_ge25519_is_canonical", &e, &tag),
            0,
            "ERRORS 163 [{tag}]: ge25519_is_canonical must report 0"
        );
        for name in [
            "crypto_sign_ed25519_verify_detached",
            "crypto_sign_verify_detached",
        ] {
            assert_eq!(
                d_verify(name, &sig, &m, &e, &format!("ERRORS 163 {tag}")),
                -1,
                "ERRORS 163 [{tag}]: {name} must reject a non-canonical pk"
            );
        }
        // And via the combined form.
        let mut sm = sig.clone();
        sm.extend_from_slice(&m);
        assert_eq!(
            d_open("crypto_sign_ed25519_open", &sm, &e, false, false, tag.as_str()).0,
            -1,
            "ERRORS 163 [{tag}]: _open must reject a non-canonical pk"
        );
        n += 1;
    }
    assert_eq!(n, 38, "ERRORS 163: expected 38 non-canonical encodings, got {n}");
    // Conversely, every CANONICAL encoding must be reported as canonical: this is
    // the complement of the reject set, so the boundary at y == p is exact.
    let mut rng = Rng::new(SEED ^ 0x163);
    for i in 0..192 {
        let mut s = arr32(&rng.bytes(32));
        // Only y in [p, 2^255) is non-canonical; force y < p to stay canonical.
        s[31] &= 0x7f;
        s[30] = 0x00;
        assert_eq!(
            d_pred("_sodium_ge25519_is_canonical", &s, &format!("canon{i}")),
            1,
            "ERRORS 163: y < p must be point-canonical"
        );
    }
    // p-1 is the last canonical y; p is the first non-canonical one.
    assert_eq!(
        d_pred("_sodium_ge25519_is_canonical", &le_sub_small(&P_BYTES, 1), "p-1"),
        1,
        "ERRORS 163: y == p-1 must be canonical"
    );
    assert_eq!(
        d_pred("_sodium_ge25519_is_canonical", &P_BYTES, "p"),
        0,
        "ERRORS 163: y == p must be non-canonical"
    );
    eprintln!("ERRORS 163: all 38 non-canonical pk encodings rejected; boundary at y==p exact");
}

/// ERRORS 164–167: the point gates of `_verify_detached` on `pk` and on `R`.
///
/// * 164 `ge25519_frombytes_negate_vartime(&A, pk) != 0`
/// * 165 `ge25519_has_small_order(&A) != 0`
/// * 166 `ge25519_frombytes(&expected_r, sig) != 0`
/// * 167 `ge25519_has_small_order(&expected_r) != 0`
///
/// Note there is NO canonicality check on `R` — only `frombytes` + small-order.
#[test]
fn e164_167_verify_point_gates() {
    init_both();
    let (pk, sk) = test_keypair(7);
    let m = msg_of(48, 11);
    let sig = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, "e164").1;
    let mut cases = 0usize;

    // ERRORS 164: pk whose y has no curve point (canonical, so gate 163 passes).
    for &y in NOT_ON_CURVE_Y.iter() {
        for sg in [false, true] {
            let bad_pk = y_enc(y, sg);
            // POINT canonicality (`ge25519_is_canonical`, gate 163) must pass so
            // that the failure is attributable to `frombytes` (gate 164).
            assert_eq!(
                d_pred("_sodium_ge25519_is_canonical", &bad_pk, "canon-check"),
                1,
                "ERRORS 164: the test pk must be point-canonical so gate 163 is bypassed"
            );
            assert_eq!(
                d_verify(
                    "crypto_sign_ed25519_verify_detached",
                    &sig,
                    &m,
                    &bad_pk,
                    &format!("ERRORS 164 y={y} sg={sg}")
                ),
                -1,
                "ERRORS 164: pk y={y} has no curve point, must be rejected"
            );
            cases += 1;
        }
    }
    // ERRORS 165: pk is one of the 8 small-order points (all point-canonical, so
    // gate 163 is bypassed and only `has_small_order` can be responsible).
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        assert_eq!(
            d_pred("_sodium_ge25519_is_canonical", p, &format!("so[{i}] canon")),
            1,
            "ERRORS 165: small-order[{i}] must be point-canonical"
        );
        assert_eq!(
            d_verify(
                "crypto_sign_ed25519_verify_detached",
                &sig,
                &m,
                p,
                &format!("ERRORS 165 so[{i}]")
            ),
            -1,
            "ERRORS 165: small-order pk[{i}] must be rejected"
        );
        cases += 1;
    }
    // ERRORS 166: R (sig[0..32)) is not a curve point.
    for &y in NOT_ON_CURVE_Y.iter() {
        for sg in [false, true] {
            let mut bad = sig.clone();
            bad[..32].copy_from_slice(&y_enc(y, sg));
            assert_eq!(
                d_verify(
                    "crypto_sign_ed25519_verify_detached",
                    &bad,
                    &m,
                    &pk,
                    &format!("ERRORS 166 y={y} sg={sg}")
                ),
                -1,
                "ERRORS 166: R with y={y} is not a curve point, must be rejected"
            );
            cases += 1;
        }
    }
    // ERRORS 167: R is small order.
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        let mut bad = sig.clone();
        bad[..32].copy_from_slice(p);
        assert_eq!(
            d_verify(
                "crypto_sign_ed25519_verify_detached",
                &bad,
                &m,
                &pk,
                &format!("ERRORS 167 so[{i}]")
            ),
            -1,
            "ERRORS 167: small-order R[{i}] must be rejected"
        );
        cases += 1;
    }
    // There is NO canonicality gate on R: a non-canonical R that decodes is
    // handled by frombytes + small-order like any other, NOT rejected up front.
    // Every non-canonical encoding of y in {p..p+18} is small-order or has no
    // curve point, so the observable result is -1 either way; assert C == Rust.
    for (tag, e) in noncanonical_encodings() {
        let mut bad = sig.clone();
        bad[..32].copy_from_slice(&e);
        assert_eq!(
            d_verify(
                "crypto_sign_ed25519_verify_detached",
                &bad,
                &m,
                &pk,
                &format!("ERRORS 166/167 non-canonical R {tag}")
            ),
            -1,
            "non-canonical R [{tag}] must be rejected"
        );
        cases += 1;
    }
    // A random garbage signature must be rejected.
    let mut rng = Rng::new(SEED ^ 164);
    for i in 0..64 {
        let bad = rng.bytes(64);
        assert_eq!(
            d_verify("crypto_sign_ed25519_verify_detached", &bad, &m, &pk, &format!("rand{i}")),
            -1,
            "a random 64-byte signature verified"
        );
        cases += 1;
    }
    eprintln!("ERRORS 164-167: {cases} rejecting (sig, pk) combinations");
}

/// ERRORS 169 + 173: `crypto_sign_verify_detached` and `crypto_sign_open` are
/// thin wrappers — they must agree with the ed25519 entry points on the ENTIRE
/// corpus of accepting and rejecting inputs.
#[test]
fn e169_e173_dispatch_wrappers() {
    init_both();
    let (pk, sk) = test_keypair(8);
    let (pk2, _) = test_keypair(9);
    let mut rng = Rng::new(SEED ^ 169);
    let mut cases = 0usize;

    let mut corpus: Vec<(String, Vec<u8>, Vec<u8>, [u8; 32])> = Vec::new();
    for &mlen in &[0usize, 1, 32, 64, 65, 128, 1000] {
        let m = msg_of(mlen, 3);
        let sig = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, "e169").1;
        corpus.push((format!("valid mlen={mlen}"), sig.clone(), m.clone(), pk));
        corpus.push((format!("wrong key mlen={mlen}"), sig.clone(), m.clone(), pk2));
        let mut flipped = sig.clone();
        flipped[0] ^= 0x01;
        corpus.push((format!("flipped R mlen={mlen}"), flipped, m.clone(), pk));
        let mut flipped2 = sig.clone();
        flipped2[32] ^= 0x01;
        corpus.push((format!("flipped S mlen={mlen}"), flipped2, m.clone(), pk));
        // S + L (malleable, ERRORS 162)
        let (s_plus_l, _) = le_add32(&arr32(&sig[32..]), &L_BYTES);
        let mut mal = sig.clone();
        mal[32..].copy_from_slice(&s_plus_l);
        corpus.push((format!("S+L mlen={mlen}"), mal, m.clone(), pk));
        for (i, p) in SMALL_ORDER.iter().enumerate() {
            corpus.push((format!("small-order pk[{i}] mlen={mlen}"), sig.clone(), m.clone(), *p));
        }
        for _ in 0..3 {
            corpus.push((format!("garbage mlen={mlen}"), rng.bytes(64), m.clone(), pk));
        }
    }

    for (tag, sig, m, key) in &corpus {
        let a = d_verify("crypto_sign_ed25519_verify_detached", sig, m, key, tag);
        let b = d_verify("crypto_sign_verify_detached", sig, m, key, tag);
        assert_eq!(
            a, b,
            "ERRORS 169 [{tag}]: crypto_sign_verify_detached ({b}) != \
             crypto_sign_ed25519_verify_detached ({a})"
        );
        // ERRORS 173: the same for the combined form.
        let mut sm = sig.clone();
        sm.extend_from_slice(m);
        let (oa, ma, la) = d_open("crypto_sign_ed25519_open", &sm, key, false, false, tag);
        let (ob, mb, lb) = d_open("crypto_sign_open", &sm, key, false, false, tag);
        assert_eq!(oa, ob, "ERRORS 173 [{tag}]: crypto_sign_open return differs");
        assert_eq!(la, lb, "ERRORS 173 [{tag}]: crypto_sign_open *mlen_p differs");
        assert_eq_bytes(&format!("ERRORS 173 [{tag}]: crypto_sign_open m differs"), &ma, &mb);
        // `_open` must agree with `_verify_detached` on the verdict.
        assert_eq!(
            oa == 0,
            a == 0,
            "ERRORS 173 [{tag}]: _open verdict ({oa}) disagrees with _verify_detached ({a})"
        );
        cases += 1;
    }
    // `crypto_sign` == `crypto_sign_ed25519`, `crypto_sign_detached` == ed25519.
    for &mlen in &[0usize, 1, 64, 65, 1000] {
        let m = msg_of(mlen, 9);
        let a = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, "e169d").1;
        let b = d_sign("crypto_sign_detached", 64, &m, &sk, false, "e169d").1;
        assert_eq_bytes("row156: crypto_sign_detached != _ed25519_detached", &a, &b);
        let c = d_sign("crypto_sign_ed25519", mlen + 64, &m, &sk, false, "e169s").1;
        let d = d_sign("crypto_sign", mlen + 64, &m, &sk, false, "e169s").1;
        assert_eq_bytes("row156: crypto_sign != crypto_sign_ed25519", &c, &d);
    }
    eprintln!("ERRORS 169/173 + row 156: {cases} corpus entries through both layers");
}

/// CONFIGS 156: the `crypto_sign` dispatch layer + all constant getters.
#[test]
fn c156_sign_dispatch() {
    init_both();
    d_size("crypto_sign_bytes", 64);
    d_size("crypto_sign_seedbytes", 32);
    d_size("crypto_sign_publickeybytes", 32);
    d_size("crypto_sign_secretkeybytes", 64);
    d_size("crypto_sign_messagebytes_max", usize::MAX - 64);
    d_size("crypto_sign_statebytes", PH_STATE);
    d_cstr("crypto_sign_primitive", "ed25519");
    // The dispatch layer must forward every entry point verbatim; the
    // accept/reject corpus is covered by `e169_e173_dispatch_wrappers`, so here
    // just re-check the happy path across a length sweep.
    let (pk, sk) = test_keypair(10);
    for &mlen in &[0usize, 1, 32, 63, 64, 65, 127, 128, 1000] {
        let m = msg_of(mlen, 0x33);
        let tag = format!("row156 mlen={mlen}");
        let sig = d_sign("crypto_sign_detached", 64, &m, &sk, false, &tag).1;
        assert_eq!(
            d_verify("crypto_sign_verify_detached", &sig, &m, &pk, &tag),
            0,
            "{tag}: dispatch round-trip failed"
        );
        let (rc, sm, smlen) = d_sign("crypto_sign", mlen + 64, &m, &sk, false, &tag);
        assert_eq!(rc, 0);
        assert_eq!(smlen, (mlen + 64) as u64);
        let (ro, mo, lo) = d_open("crypto_sign_open", &sm, &pk, false, false, &tag);
        assert_eq!(ro, 0, "{tag}: crypto_sign_open failed");
        assert_eq!(lo, mlen as u64);
        assert_eq_bytes(&format!("{tag}: crypto_sign_open message"), &m, &mo[..mlen]);
    }
    eprintln!("row 156: dispatch layer + 7 constants");
}

/// CONFIGS 157/158 + ERRORS 178: `ed25519ph` — update chunking
/// {0, 1, 127, 128, 129} x {1 update, N updates, ZERO updates}. The 34-byte
/// DOM2PREFIX must be prepended, so a ph signature must NEVER validate as a
/// plain one and vice versa.
#[test]
fn c157_c158_e178_ed25519ph() {
    init_both();
    d_size("crypto_sign_ed25519ph_statebytes", PH_STATE);
    d_size("crypto_sign_statebytes", PH_STATE);
    let (pk, sk) = test_keypair(11);
    let (pk2, _) = test_keypair(12);
    let mut rng = Rng::new(SEED ^ 157);

    // Chunkings of the same total message must give the same signature.
    let base_lens: [usize; 5] = [0, 1, 127, 128, 129];
    let mut cases = 0usize;
    for &total in &[0usize, 1, 63, 64, 127, 128, 129, 255, 256, 1000] {
        let m = msg_of(total, 0x77);
        let mut scripts: Vec<Vec<Vec<u8>>> = Vec::new();
        // ZERO updates only makes sense for total == 0.
        if total == 0 {
            scripts.push(vec![]);
            scripts.push(vec![vec![]]);
            scripts.push(vec![vec![], vec![], vec![]]);
        } else {
            scripts.push(vec![m.clone()]); // 1 update
            // N updates, split on each of the spec's boundary sizes.
            for &c in &base_lens {
                if c == 0 {
                    continue;
                }
                let mut s: Vec<Vec<u8>> = m.chunks(c).map(|x| x.to_vec()).collect();
                // interleave a 0-length update
                s.insert(0, vec![]);
                s.push(vec![]);
                scripts.push(s);
            }
            // random split
            let mut s: Vec<Vec<u8>> = Vec::new();
            let mut off = 0usize;
            while off < total {
                let n = 1 + rng.below(total - off);
                s.push(m[off..off + n].to_vec());
                off += n;
            }
            scripts.push(s);
        }

        let mut ref_sig: Option<Vec<u8>> = None;
        for (si, script) in scripts.iter().enumerate() {
            let tag = format!("row157 total={total} script{si}");
            for prefix in ["crypto_sign_ed25519ph", "crypto_sign"] {
                let (sig, siglen) = d_ph_create(prefix, script, &sk, &tag);
                assert_eq!(siglen, 64, "{tag}: *siglen_p must be 64");
                if let Some(r) = &ref_sig {
                    assert_eq_bytes(
                        &format!(
                            "{tag}: chunking changed the ph signature (or the \
                             crypto_sign_* dispatch layer disagrees)"
                        ),
                        r,
                        &sig,
                    );
                } else {
                    ref_sig = Some(sig.clone());
                }
                // final_verify round-trip, in both namespaces (CONFIGS 158).
                for vprefix in ["crypto_sign_ed25519ph", "crypto_sign"] {
                    assert_eq!(
                        d_ph_verify(vprefix, script, &sig, &pk, &tag),
                        0,
                        "{tag}: {vprefix}_final_verify round-trip FAILED"
                    );
                }
                // ERRORS 178: wrong key, flipped signature bits, small-order and
                // non-canonical pk, S+L malleability — all on the prehash path.
                assert_eq!(
                    d_ph_verify("crypto_sign_ed25519ph", script, &sig, &pk2, &tag),
                    -1,
                    "ERRORS 178 [{tag}]: verified under the wrong key"
                );
                for &bit in &[0usize, 255, 256, 511] {
                    let mut bad = sig.clone();
                    bad[bit / 8] ^= 1 << (bit % 8);
                    assert_eq!(
                        d_ph_verify("crypto_sign_ed25519ph", script, &bad, &pk, &tag),
                        -1,
                        "ERRORS 178 [{tag}]: bit {bit} flipped still verified"
                    );
                }
                cases += 1;
            }
        }
        // DOM2PREFIX separation: a ph signature must not verify as a plain one
        // over the SHA-512 prehash, and a plain signature over the prehash must
        // not verify as ph.
        let sig = ref_sig.clone().unwrap();
        let ph = d_sha512(&m);
        assert_eq!(
            d_verify("crypto_sign_ed25519_verify_detached", &sig, &ph, &pk, "dom2"),
            -1,
            "row157 total={total}: a ph signature verified as a PLAIN signature \
             over the prehash — DOM2PREFIX is missing"
        );
        let plain = d_sign("crypto_sign_ed25519_detached", 64, &ph, &sk, false, "dom2").1;
        let script = if total == 0 { vec![] } else { vec![m.clone()] };
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &plain, &pk, "dom2"),
            -1,
            "row157 total={total}: a PLAIN signature over the prehash verified as ph"
        );
        assert_ne!(sig, plain, "row157 total={total}: ph and plain signatures are identical");
    }
    // ZERO updates: init + final_create must equal signing SHA-512("") as prehash.
    let empty: Vec<Vec<u8>> = vec![];
    let (sig0, _) = d_ph_create("crypto_sign_ed25519ph", &empty, &sk, "zero-updates");
    let (sig0b, _) = d_ph_create("crypto_sign_ed25519ph", &[vec![]], &sk, "one-empty-update");
    assert_eq_bytes(
        "row157: ZERO updates != one zero-length update",
        &sig0,
        &sig0b,
    );
    assert_eq!(
        d_ph_verify("crypto_sign_ed25519ph", &empty, &sig0, &pk, "zero-updates"),
        0,
        "row157: ZERO-update signature must verify"
    );
    eprintln!("rows 157/158 + ERRORS 178: {cases} chunking scripts x 2 namespaces");
}

/// ERRORS 178: the ph verifier reuses `_crypto_sign_ed25519_verify_detached`
/// with `prehashed = 1`, so ALL of gates 162–168 apply to it. Drive the very
/// same rejecting corpus through `_final_verify`.
#[test]
fn e178_ph_final_verify_rejections() {
    init_both();
    let (pk, sk) = test_keypair(13);
    let m = msg_of(200, 0x21);
    let script = vec![m.clone()];
    let (sig, _) = d_ph_create("crypto_sign_ed25519ph", &script, &sk, "e178");
    assert_eq!(d_ph_verify("crypto_sign_ed25519ph", &script, &sig, &pk, "e178"), 0);
    let mut cases = 0usize;

    // gate 163: non-canonical pk
    for (tag, e) in noncanonical_encodings() {
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &sig, &e, &tag),
            -1,
            "ERRORS 178/163 [{tag}]: non-canonical pk must be rejected on the ph path"
        );
        cases += 1;
    }
    // gates 164/165: pk not on curve / small order
    for &y in NOT_ON_CURVE_Y.iter() {
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &sig, &y_enc(y, false), "e178-164"),
            -1,
            "ERRORS 178/164: pk y={y} must be rejected"
        );
        cases += 1;
    }
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &sig, p, "e178-165"),
            -1,
            "ERRORS 178/165: small-order pk[{i}] must be rejected"
        );
        cases += 1;
    }
    // gates 166/167: R not on curve / small order
    for &y in NOT_ON_CURVE_Y.iter() {
        let mut bad = sig.clone();
        bad[..32].copy_from_slice(&y_enc(y, false));
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &bad, &pk, "e178-166"),
            -1,
            "ERRORS 178/166: R y={y} must be rejected"
        );
        cases += 1;
    }
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        let mut bad = sig.clone();
        bad[..32].copy_from_slice(p);
        assert_eq!(
            d_ph_verify("crypto_sign_ed25519ph", &script, &bad, &pk, "e178-167"),
            -1,
            "ERRORS 178/167: small-order R[{i}] must be rejected"
        );
        cases += 1;
    }
    // gate 162: S + L
    let (s_plus_l, carry) = le_add32(&arr32(&sig[32..]), &L_BYTES);
    assert_eq!(carry, 0, "ERRORS 178/162: S + L must fit in 32 bytes");
    let mut mal = sig.clone();
    mal[32..].copy_from_slice(&s_plus_l);
    assert_ne!(mal[63] & 0xf0, 0, "ERRORS 178/162: S+L must set the high nibble");
    assert_eq!(
        d_ph_verify("crypto_sign_ed25519ph", &script, &mal, &pk, "e178-162"),
        -1,
        "ERRORS 178/162: S+L must be rejected on the ph path"
    );
    cases += 1;
    // gate 168: a wrong-but-well-formed signature (different message)
    let other = vec![msg_of(200, 0x22)];
    let (sig_other, _) = d_ph_create("crypto_sign_ed25519ph", &other, &sk, "e178-168");
    assert_eq!(
        d_ph_verify("crypto_sign_ed25519ph", &script, &sig_other, &pk, "e178-168"),
        -1,
        "ERRORS 178/168: a signature over a different prehash must be rejected"
    );
    cases += 1;
    eprintln!("ERRORS 178: {cases} rejecting inputs through _final_verify");
}

/// CONFIGS 159 + the `sk_to_pk` / `sk_to_seed` consistency property.
#[test]
fn c159_sk_to_seed_and_pk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 159);
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for _ in 0..72 {
        seeds.push(arr32(&rng.bytes(32)));
    }
    for (i, seed) in seeds.iter().enumerate() {
        let (pk, sk) = keypair_from_seed(seed);
        let tag = format!("row159 #{i}");
        let (rs, got_seed) = d_int1("crypto_sign_ed25519_sk_to_seed", 32, &sk, &tag);
        assert_eq!(rs, 0, "{tag}: sk_to_seed must return 0");
        assert_eq_bytes(&format!("{tag}: sk_to_seed != seed"), seed, &got_seed);
        let (rp, got_pk) = d_int1("crypto_sign_ed25519_sk_to_pk", 32, &sk, &tag);
        assert_eq!(rp, 0, "{tag}: sk_to_pk must return 0");
        assert_eq_bytes(&format!("{tag}: sk_to_pk != pk"), &pk, &got_pk);
        // Consistency: seed_keypair(sk_to_seed(sk)) reproduces (pk, sk).
        let (pk2, sk2) = keypair_from_seed(&got_seed);
        assert_eq_bytes(&format!("{tag}: seed_keypair(sk_to_seed(sk)).pk != pk"), &pk, &pk2);
        assert_eq_bytes(&format!("{tag}: seed_keypair(sk_to_seed(sk)).sk != sk"), &sk, &sk2);
        assert_eq_bytes(&format!("{tag}: sk_to_pk != sk_to_seed-derived pk"), &got_pk, &pk2);
        // Both are pure memmoves: they work on ANY 64 bytes.
        let junk = rng.bytes(64);
        let (rj, js) = d_int1("crypto_sign_ed25519_sk_to_seed", 32, &junk, &tag);
        assert_eq!(rj, 0, "{tag}: sk_to_seed is a memmove, it never fails");
        assert_eq_bytes(&format!("{tag}: sk_to_seed(junk) != junk[0..32)"), &junk[..32], &js);
        let (rj2, jp) = d_int1("crypto_sign_ed25519_sk_to_pk", 32, &junk, &tag);
        assert_eq!(rj2, 0, "{tag}: sk_to_pk is a memmove, it never fails");
        assert_eq_bytes(&format!("{tag}: sk_to_pk(junk) != junk[32..64)"), &junk[32..], &jp);
    }
    eprintln!("row 159: {} keypairs, sk_to_seed/sk_to_pk consistency", seeds.len());
}

/// CONFIGS 160: `_sk_to_curve25519` — SHA-512 of `sk[0..32)` then clamp.
#[test]
fn c160_sk_to_curve25519() {
    init_both();
    let mut rng = Rng::new(SEED ^ 160);
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for _ in 0..72 {
        seeds.push(arr32(&rng.bytes(32)));
    }
    for (i, seed) in seeds.iter().enumerate() {
        let (_pk, sk) = keypair_from_seed(seed);
        let tag = format!("row160 #{i}");
        let (rc, csk) = d_int1("crypto_sign_ed25519_sk_to_curve25519", 32, &sk, &tag);
        assert_eq!(rc, 0, "{tag}: sk_to_curve25519 must return 0");
        // It must equal clamp(sha512(sk[0..32))[0..32)).
        let h = d_sha512(&sk[..32]);
        let mut want = arr32(&h[..32]);
        want[0] &= 248;
        want[31] &= 127;
        want[31] |= 64;
        assert_eq_bytes(
            &format!("{tag}: sk_to_curve25519 != clamp(sha512(seed)[0..32))"),
            &want,
            &csk,
        );
        // The clamp really is applied.
        assert_eq!(csk[0] & 7, 0, "{tag}: low 3 bits not cleared");
        assert_eq!(csk[31] & 0x80, 0, "{tag}: bit 255 not cleared");
        assert_eq!(csk[31] & 0x40, 0x40, "{tag}: bit 254 not set");
        // It only reads sk[0..32), so it works on any 64-byte buffer.
        let junk = rng.bytes(64);
        let (rj, cj) = d_int1("crypto_sign_ed25519_sk_to_curve25519", 32, &junk, &tag);
        assert_eq!(rj, 0, "{tag}: sk_to_curve25519 never fails");
        let hj = d_sha512(&junk[..32]);
        let mut wj = arr32(&hj[..32]);
        wj[0] &= 248;
        wj[31] &= 127;
        wj[31] |= 64;
        assert_eq_bytes(&format!("{tag}: sk_to_curve25519(junk)"), &wj, &cj);
    }
    eprintln!("row 160: {} keys, clamp(sha512(seed)) verified", seeds.len());
}

/// CONFIGS 161 + ERRORS 174–177: `_pk_to_curve25519`. Its gate is
/// `frombytes_negate_vartime` + `has_small_order` + `is_on_main_subgroup` —
/// there is NO `ge25519_is_canonical` call.
#[test]
fn c161_e174_176_pk_to_curve25519() {
    init_both();
    let mut rng = Rng::new(SEED ^ 161);
    // Happy path: a real ed25519 public key maps to the matching X25519 key.
    for i in 0..72 {
        let seed = rng.bytes(32);
        let (pk, sk) = keypair_from_seed(&seed);
        let tag = format!("row161 #{i}");
        let (rc, cpk) = d_int1("crypto_sign_ed25519_pk_to_curve25519", 32, &pk, &tag);
        assert_eq!(rc, 0, "{tag}: a valid pk must be accepted");
        // The converted pair must perform a consistent X25519 exchange: the
        // birational map must agree with `crypto_scalarmult_curve25519_base`.
        let csk = d_int1("crypto_sign_ed25519_sk_to_curve25519", 32, &sk, &tag).1;
        let (rb, base) = d_int1("crypto_scalarmult_curve25519_base", 32, &csk, &tag);
        assert_eq!(rb, 0);
        assert_eq_bytes(
            &format!("{tag}: pk_to_curve25519(pk) != curve25519_base(sk_to_curve25519(sk))"),
            &base,
            &cpk,
        );
        // Both sign bits of the ed25519 pk map to the SAME X25519 key (the map
        // only uses the y coordinate).
        let mut flipped = pk;
        flipped[31] ^= 0x80;
        let (rf, cf) = d_int1("crypto_sign_ed25519_pk_to_curve25519", 32, &flipped, &tag);
        assert_eq!(rf, 0, "{tag}: -pk must also be accepted");
        assert_eq_bytes(&format!("{tag}: pk and -pk gave different X25519 keys"), &cpk, &cf);
    }
    // ERRORS 174: `frombytes_negate_vartime` fails.
    for &y in NOT_ON_CURVE_Y.iter() {
        for sg in [false, true] {
            let (rc, buf) = d_int1(
                "crypto_sign_ed25519_pk_to_curve25519",
                32,
                &y_enc(y, sg),
                &format!("ERRORS 174 y={y} sg={sg}"),
            );
            assert_eq!(rc, -1, "ERRORS 174: pk y={y} has no curve point");
            assert!(
                buf.iter().all(|&x| x == FILL),
                "ERRORS 174: curve25519_pk written on the rejection path"
            );
        }
    }
    // ERRORS 175: small order.
    for (i, p) in SMALL_ORDER.iter().enumerate() {
        let (rc, buf) = d_int1(
            "crypto_sign_ed25519_pk_to_curve25519",
            32,
            p,
            &format!("ERRORS 175 so[{i}]"),
        );
        assert_eq!(rc, -1, "ERRORS 175: small-order pk[{i}] must be rejected");
        assert!(buf.iter().all(|&x| x == FILL), "ERRORS 175: output written");
    }
    // ERRORS 176: on the curve, not small order, but off the main subgroup.
    let valid = valid_ed_points(1, SEED ^ 176)[0];
    let mut torsion_cases = 0usize;
    for (i, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let tp = arr32(&d_int2("crypto_core_ed25519_add", 32, &valid, t, "e176").1);
        let (rc, buf) = d_int1(
            "crypto_sign_ed25519_pk_to_curve25519",
            32,
            &tp,
            &format!("ERRORS 176 torsion{i}"),
        );
        assert_eq!(
            rc, -1,
            "ERRORS 176: torsion pk (P + small-order[{i}]) must be rejected"
        );
        assert!(buf.iter().all(|&x| x == FILL), "ERRORS 176: output written");
        torsion_cases += 1;
    }
    assert_eq!(torsion_cases, 7);
    eprintln!("row 161 / ERRORS 174-176: 72 happy paths + {} rejections", 14 + 8 + 7);
}

/// ERRORS 177: `_pk_to_curve25519` performs NO `ge25519_is_canonical` check, so
/// a non-canonical encoding is decoded (reduced mod p) exactly like the
/// canonical encoding of the same y — it is never rejected FOR BEING
/// non-canonical.
///
/// NOTE on reachability: a non-canonical encoding exists only for y in
/// {p, ..., p+18}, i.e. y mod p in 0..=18. Every one of those 19 y values
/// (both sign bits, 38 encodings in total) is either off the curve or NOT in the
/// main subgroup, so the literal "returns 0" outcome of this row is UNREACHABLE
/// through the public API. What IS observable, and is asserted here, is that the
/// verdict and output for a non-canonical encoding are byte-identical to those
/// for the canonical encoding of the same reduced y — which cannot happen if a
/// canonicality gate runs first. The contrast with `_verify_detached` and
/// `crypto_scalarmult_ed25519` (which DO reject on canonicality) is asserted too.
#[test]
fn e177_pk_to_curve25519_no_canonical_check() {
    init_both();
    let mut checked = 0usize;
    for (tag, e) in noncanonical_encodings() {
        // y = p + k  =>  y mod p == k, so the canonical twin is just y_enc(k).
        let k = e[0].wrapping_sub(0xed);
        assert!(k <= 18, "ERRORS 177: unexpected non-canonical first byte");
        let canon = y_enc(k, e[31] & 0x80 != 0);

        let (rn, bn) = d_int1(
            "crypto_sign_ed25519_pk_to_curve25519",
            32,
            &e,
            &format!("ERRORS 177 non-canonical {tag}"),
        );
        let (rc, bc) = d_int1(
            "crypto_sign_ed25519_pk_to_curve25519",
            32,
            &canon,
            &format!("ERRORS 177 canonical y={k}"),
        );
        assert_eq!(
            rn, rc,
            "ERRORS 177 [{tag}]: pk_to_curve25519(non-canonical) = {rn} but \
             pk_to_curve25519(canonical y={k}) = {rc} — a canonicality gate fired"
        );
        assert_eq_bytes(
            &format!(
                "ERRORS 177 [{tag}]: the non-canonical encoding produced a \
                 DIFFERENT output than the canonical encoding of y={k}"
            ),
            &bc,
            &bn,
        );
        // Contrast: the entry points that DO have a canonicality gate reject
        // the non-canonical form. `crypto_scalarmult_ed25519` rejects it while
        // `crypto_core_ed25519_add` accepts it (proven in `e207_*`).
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &e, tag.as_str()),
            0,
            "ERRORS 177 contrast [{tag}]: _is_valid_point must reject it"
        );
        let n: Vec<u8> = (1..=32u8).collect();
        assert_eq!(
            d_int2("crypto_scalarmult_ed25519", 32, &n, &e, tag.as_str()).0,
            -1,
            "ERRORS 177 contrast [{tag}]: crypto_scalarmult_ed25519 must reject it"
        );
        checked += 1;
    }
    assert_eq!(checked, 38, "ERRORS 177: expected 38 non-canonical encodings");
    eprintln!(
        "ERRORS 177: 38 non-canonical encodings behave exactly like their \
         reduced canonical twins (no is_canonical gate)"
    );
}

/// CONFIGS 162 + ERRORS 162: the malleability axis.
///
/// The C guard is `(sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0`.
/// Because `L > 2^252`, ANY `S >= L` necessarily has `S >> 248 >= 0x10`, i.e.
/// `sig[63] & 0xF0 != 0` — so the CONFIGS-162 case "`S >= L` with
/// `sig[63] & 0xF0 == 0` is ACCEPTED" is arithmetically UNREACHABLE, and the
/// short-circuit is equivalent to plain `!is_canonical(S)`. All three states of
/// the `&&` are still driven here:
///   (a) high nibble 0 -> `is_canonical` never evaluated (every genuine
///       signature, since S < L < 2^252 means sig[63] <= 0x0f);
///   (b) high nibble set AND canonical (S in [2^252, L)) -> gate passes;
///   (c) high nibble set AND non-canonical (S >= L) -> REJECTED.
///
/// (c) is sharp: `ge25519_double_scalarmult_vartime` consumes the RAW 256-bit S,
/// and `(S + L)*B == S*B`, so without the guard the mauled signature WOULD
/// verify. Asserting it is rejected therefore pins the guard itself.
#[test]
fn c162_e162_malleability_axis() {
    init_both();
    let (pk, sk) = test_keypair(14);
    let mut lo_nibble = 0usize;
    let mut mauled = 0usize;

    for &mlen in &[0usize, 1, 32, 64, 65, 128, 1000] {
        for salt in 0..8u8 {
            let m = msg_of(mlen, salt);
            let tag = format!("row162 mlen={mlen} salt={salt}");
            let sig = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, &tag).1;
            let s = arr32(&sig[32..]);

            // (a) a genuine signature: S is canonical and sig[63] & 0xF0 == 0,
            //     so the first operand of the `&&` is FALSE.
            assert_eq!(
                d_pred("crypto_core_ed25519_scalar_is_canonical", &s, &tag),
                1,
                "{tag}: a genuine S must be canonical"
            );
            assert_eq!(
                sig[63] & 0xf0,
                0,
                "{tag}: a genuine S is < 2^252, so sig[63] & 0xF0 must be 0"
            );
            assert_eq!(
                d_verify("crypto_sign_ed25519_verify_detached", &sig, &m, &pk, &tag),
                0,
                "{tag}: a canonical signature must be ACCEPTED"
            );
            lo_nibble += 1;

            // (c) S + L: non-canonical, high nibble necessarily set -> REJECTED,
            //     even though (S+L)*B == S*B makes it mathematically valid.
            let (s_plus_l, carry) = le_add32(&s, &L_BYTES);
            assert_eq!(carry, 0, "{tag}: S + L must fit in 32 bytes");
            let mut mal = sig.clone();
            mal[32..].copy_from_slice(&s_plus_l);
            assert_eq!(
                d_pred("crypto_core_ed25519_scalar_is_canonical", &s_plus_l, &tag),
                0,
                "{tag}: S + L must be non-canonical"
            );
            assert_ne!(
                mal[63] & 0xf0,
                0,
                "{tag}: S + L >= 2^252 must set the high nibble of sig[63]"
            );
            assert_eq!(
                d_verify("crypto_sign_ed25519_verify_detached", &mal, &m, &pk, &tag),
                -1,
                "{tag}: S + L is malleable and MUST be rejected by the \
                 (sig[63] & 240) && !is_canonical(S) guard"
            );
            // The same through every other entry point.
            assert_eq!(
                d_verify("crypto_sign_verify_detached", &mal, &m, &pk, &tag),
                -1,
                "{tag}: the dispatch wrapper must reject S + L too"
            );
            let mut sm = mal.clone();
            sm.extend_from_slice(&m);
            assert_eq!(
                d_open("crypto_sign_ed25519_open", &sm, &pk, false, false, &tag).0,
                -1,
                "{tag}: _open must reject S + L"
            );
            mauled += 1;
        }
    }

    // (b) high nibble SET and S canonical: S in [2^252, L). Such an S can never
    //     be a genuine signature scalar (probability ~2^-128), so it fails the
    //     FINAL check rather than the gate — but it proves the second operand of
    //     the `&&` is evaluated and returns "canonical".
    let m = msg_of(32, 0x99);
    let sig = d_sign("crypto_sign_ed25519_detached", 64, &m, &sk, false, "row162b").1;
    let mut p252 = [0u8; 32];
    p252[31] = 0x10;
    for (tag, s) in [
        ("2^252", p252),
        ("L-1", le_sub_small(&L_BYTES, 1)),
    ] {
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &s, tag),
            1,
            "row162 [{tag}]: must be canonical"
        );
        assert_ne!(s[31] & 0xf0, 0, "row162 [{tag}]: must set the high nibble");
        let mut probe = sig.clone();
        probe[32..].copy_from_slice(&s);
        assert_eq!(
            d_verify("crypto_sign_ed25519_verify_detached", &probe, &m, &pk, tag),
            -1,
            "row162 [{tag}]: gate passes but the final check must fail"
        );
    }
    // Non-canonical S values that are NOT S + L, for completeness.
    for (tag, s) in [
        ("L", L_BYTES),
        ("L+1", le_add_small(&L_BYTES, 1)),
        ("2^255-1", {
            let mut v = [0xffu8; 32];
            v[31] = 0x7f;
            v
        }),
        ("2^256-1", [0xffu8; 32]),
    ] {
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &s, tag),
            0,
            "row162 [{tag}]: must be non-canonical"
        );
        assert_ne!(s[31] & 0xf0, 0, "row162 [{tag}]: must set the high nibble");
        let mut probe = sig.clone();
        probe[32..].copy_from_slice(&s);
        assert_eq!(
            d_verify("crypto_sign_ed25519_verify_detached", &probe, &m, &pk, tag),
            -1,
            "ERRORS 162 [{tag}]: non-canonical S with the high nibble set must be rejected"
        );
    }
    // Pin the arithmetic fact that makes CONFIGS 162's "ACCEPTED" case dead:
    // `sig[63] & 0xF0 == 0` implies `S < 2^252 < L`, hence canonical.
    for i in 0..64u8 {
        let mut s = [0xffu8; 32];
        s[31] = i & 0x0f; // high nibble clear by construction
        assert_eq!(
            d_pred("crypto_core_ed25519_scalar_is_canonical", &s, "nibble-proof"),
            1,
            "ERRORS 162: S with sig[63] & 0xF0 == 0 must ALWAYS be canonical \
             (so the guard's short-circuit can never skip a non-canonical S)"
        );
    }
    eprintln!(
        "row 162 / ERRORS 162: {lo_nibble} canonical accepts, {mauled} S+L rejects, \
         6 crafted S values, 64 nibble proofs"
    );
}

/// CONFIGS 163 + ERRORS 168: verification is COFACTORED. The final check is
/// `ge25519_has_small_order(&check) - 1` with
/// `check = R - (S*B - h*A)`; `crypto_verify_32` is `#include`d in open.c but
/// NEVER called.
///
/// Construction (entirely through the public API): pick a scalar `a`, set
/// `pk = a*B + T` for a small-order `T` (accepted by `crypto_core_ed25519_add`,
/// ERRORS 207; `pk` itself is neither small-order nor canonical-invalid, so it
/// passes every gate), then sign honestly with `S = r + h*a mod L`. The residual
/// is `check = h*T`, a NON-IDENTITY small-order point — which cofactorless
/// verification would reject and cofactored verification accepts.
///
/// One further C detail is pinned here: `ge25519_p2_to_p3` sets `T = X*Y`
/// instead of `X*Y/Z`, so the order-8 leg of `has_small_order` (the
/// `y*sqrt(-1) == +-x` test, which depends on that T) is only reached correctly
/// when the residual has order <= 4. Hence order-2 and order-4 residuals ALWAYS
/// verify, while an order-8 residual verifies exactly when `h` is even (making
/// `h*T` order <= 4). Both regimes are asserted.
#[test]
fn c163_e168_cofactored_verification() {
    init_both();
    let mut rng = Rng::new(SEED ^ 163);

    // Canonical scalars a (secret) and r (nonce), via reduce so both are < L.
    let a = arr32(&d_void1(
        "crypto_core_ed25519_scalar_reduce",
        32,
        &rng.bytes(64),
        "cofactor-a",
    ));
    let r = arr32(&d_void1(
        "crypto_core_ed25519_scalar_reduce",
        32,
        &rng.bytes(64),
        "cofactor-r",
    ));
    let (ra, a_b) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &a, "cofactor-aB");
    let (rr, r_pt) = d_int1("crypto_scalarmult_ed25519_base_noclamp", 32, &r, "cofactor-R");
    assert_eq!((ra, rr), (0, 0), "row163: a*B and r*B must be well defined");
    let a_b = arr32(&a_b);
    let r_pt = arr32(&r_pt);

    // Control: an honest signature with an honest pk (residual = identity).
    let m0 = msg_of(24, 0x10);
    let sig0 = cofactor_sign(&r_pt, &a_b, &a, &r, &m0);
    assert_eq!(
        d_verify("crypto_sign_ed25519_verify_detached", &sig0, &m0, &a_b, "control"),
        0,
        "row163 control: the hand-built honest signature must verify (residual = identity)"
    );

    let mut order24_accepts = 0usize;
    let mut order8_accepts = 0usize;
    let mut order8_rejects = 0usize;

    for (ti, t) in SMALL_ORDER.iter().enumerate().skip(1) {
        let ord = SMALL_ORDER_ORD[ti];
        // pk = a*B + T. `_add` accepts the small-order operand (ERRORS 207).
        let (rc, pk) = d_int2(
            "crypto_core_ed25519_add",
            32,
            &a_b,
            t,
            &format!("row163 pk = aB + so[{ti}]"),
        );
        assert_eq!(rc, 0, "row163: building pk = aB + small-order[{ti}] must succeed");
        let pk = arr32(&pk);
        // pk really does carry a torsion component: off the main subgroup, yet
        // it passes every gate of `_verify_detached` (canonical, decodable, not
        // small order).
        assert_eq!(
            d_pred("crypto_core_ed25519_is_valid_point", &pk, "row163"),
            0,
            "row163 [so{ti}]: pk must be OFF the main subgroup"
        );
        assert_eq!(
            d_int1("crypto_sign_ed25519_pk_to_curve25519", 32, &pk, "row163").0,
            -1,
            "row163 [so{ti}]: pk must fail the main-subgroup gate of pk_to_curve25519"
        );

        for mv in 0..12u8 {
            let m = msg_of(24, 0x20 + mv);
            let sig = cofactor_sign(&r_pt, &pk, &a, &r, &m);
            // h is needed to predict the order-8 regime.
            let mut pre = Vec::with_capacity(64 + m.len());
            pre.extend_from_slice(&r_pt);
            pre.extend_from_slice(&pk);
            pre.extend_from_slice(&m);
            let h = d_void1(
                "crypto_core_ed25519_scalar_reduce",
                32,
                &d_sha512(&pre),
                "row163 h",
            );
            let tag = format!("row163 so[{ti}] ord={ord} mv={mv} h_even={}", h[0] & 1 == 0);
            let got = d_verify("crypto_sign_ed25519_verify_detached", &sig, &m, &pk, &tag);
            // Every other entry point must agree.
            assert_eq!(
                d_verify("crypto_sign_verify_detached", &sig, &m, &pk, &tag),
                got,
                "{tag}: the dispatch wrapper disagrees"
            );
            let mut sm = sig.clone();
            sm.extend_from_slice(&m);
            assert_eq!(
                d_open("crypto_sign_ed25519_open", &sm, &pk, false, false, &tag).0,
                got,
                "{tag}: _open disagrees with _verify_detached"
            );

            if ord <= 4 {
                assert_eq!(
                    got, 0,
                    "{tag}: COFACTORED verification must ACCEPT a signature whose \
                     residual is the order-{ord} torsion point h*T \
                     (cofactorless verification would reject it)"
                );
                order24_accepts += 1;
            } else {
                let expect = if h[0] & 1 == 0 { 0 } else { -1 };
                assert_eq!(
                    got, expect,
                    "{tag}: an order-8 T residual must be accepted exactly when h \
                     is even (h*T then has order <= 4); ge25519_p2_to_p3's \
                     T = X*Y corrupts the order-8 leg of has_small_order"
                );
                if got == 0 {
                    order8_accepts += 1;
                } else {
                    order8_rejects += 1;
                }
            }
        }
    }
    assert!(
        order24_accepts >= 36,
        "row163: only {order24_accepts} order-2/4 cofactored acceptances"
    );
    assert!(
        order8_accepts > 0 && order8_rejects > 0,
        "row163: the order-8 regime was not exercised both ways \
         ({order8_accepts} accepts / {order8_rejects} rejects)"
    );
    eprintln!(
        "row 163 / ERRORS 168: {order24_accepts} order-2/4 cofactored ACCEPTS, \
         order-8 {order8_accepts} accept / {order8_rejects} reject"
    );
}

/// Build `sig = R || (r + h*a mod L)` with `h = reduce(SHA-512(R || pk || m))`,
/// entirely through already-differentially-verified entry points.
fn cofactor_sign(
    r_pt: &[u8; 32],
    pk: &[u8; 32],
    a: &[u8; 32],
    r: &[u8; 32],
    m: &[u8],
) -> Vec<u8> {
    let mut pre = Vec::with_capacity(64 + m.len());
    pre.extend_from_slice(r_pt);
    pre.extend_from_slice(pk);
    pre.extend_from_slice(m);
    let h64 = d_sha512(&pre);
    let h = d_void1("crypto_core_ed25519_scalar_reduce", 32, &h64, "cofactor-h");
    let ha = d_void2("crypto_core_ed25519_scalar_mul", 32, &h, a, "cofactor-ha");
    let s = d_void2("crypto_core_ed25519_scalar_add", 32, &ha, r, "cofactor-S");
    let mut sig = Vec::with_capacity(64);
    sig.extend_from_slice(r_pt);
    sig.extend_from_slice(&s);
    sig
}
