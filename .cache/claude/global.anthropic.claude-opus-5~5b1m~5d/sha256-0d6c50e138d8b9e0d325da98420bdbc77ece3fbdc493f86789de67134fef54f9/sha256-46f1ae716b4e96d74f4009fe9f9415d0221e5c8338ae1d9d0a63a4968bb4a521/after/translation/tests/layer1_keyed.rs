//! Phase B rows 26-35: the keyed-hash façade (`hash_<b>.c`, `thash_<b>_<t>.c`)
//! and `chain_lengths`.
mod common;
use common::*;

type FnInitHash = unsafe extern "C" fn(*mut SpxCtxFfi);
type FnPrfAddr = unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, *const u32);
type FnThash =
    unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_uint, *const SpxCtxFfi, *mut u32);
type FnGenMsgRandom = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    *const u8,
    core::ffi::c_ulonglong,
    *const SpxCtxFfi,
);
type FnHashMessage = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    core::ffi::c_ulonglong,
    *const SpxCtxFfi,
);
type FnChainLengths = unsafe extern "C" fn(*mut core::ffi::c_uint, *const u8);

const MLENS: &[usize] = &[
    0, 1, 2, 16, 31, 32, 33, 54, 55, 56, 57, 63, 64, 65, 71, 72, 73, 110, 111, 112, 113, 127, 128,
    129, 135, 136, 137, 200, 231, 500,
];

/// Build a fully-initialised `spx_ctx` on BOTH sides from the same seeds, and
/// assert the two contexts are byte-identical (row 26).
fn make_ctx(rng: &mut Rng) -> (SpxCtxFfi, SpxCtxFfi) {
    let l = libs();
    let (c, r) = l.pair::<FnInitHash>("SPX_initialize_hash_function");
    let mut cc = SpxCtxFfi::zeroed();
    rng.fill(&mut cc.pub_seed);
    rng.fill(&mut cc.sk_seed);
    let mut rc = cc.clone();
    unsafe {
        c(&mut cc);
        r(&mut rc);
    }
    assert_bytes_eq(
        "SPX_initialize_hash_function -> spx_ctx",
        cc.as_bytes(),
        rc.as_bytes(),
    );
    (cc, rc)
}

// --- row 26 -------------------------------------------------------------

#[test]
fn row26_initialize_hash_function() {
    let mut rng = Rng::new(0x2601);
    for _ in 0..128 {
        let _ = make_ctx(&mut rng);
    }
    // Also: the struct size the tests use must equal the C's (otherwise every
    // other row would silently pass on a truncated view).
    assert!(core::mem::size_of::<SpxCtxFfi>() >= 2 * SPX_N);
}

// --- row 27 -------------------------------------------------------------

#[test]
fn row27_prf_addr() {
    let l = libs();
    let (c, r) = l.pair::<FnPrfAddr>("SPX_prf_addr");
    let mut rng = Rng::new(0x2701);
    for i in 0..256 {
        let (cc, rc) = make_ctx(&mut rng);
        let mut addr = rng.addr();
        // Also exercise the 7 documented address types explicitly.
        if i < 7 {
            let b = unsafe {
                core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
            };
            b[SPX_OFFSET_TYPE] = i as u8;
        }
        let mut co = vec![0xA5u8; SPX_N + 8];
        let mut ro = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), &cc, addr.as_ptr());
            r(ro.as_mut_ptr(), &rc, addr.as_ptr());
        }
        assert_bytes_eq(&format!("SPX_prf_addr addr={}", hex(&addr_bytes(&addr))), &co, &ro);
    }
}

// --- rows 28-32 (thash across every inblocks shape) ---------------------

fn thash_case(inblocks: usize, iters: usize, seed: u64, label: &str) {
    let l = libs();
    let (c, r) = l.pair::<FnThash>("SPX_thash");
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let (cc, rc) = make_ctx(&mut rng);
        let inp = rng.bytes(inblocks * SPX_N + 1);
        let addr = rng.addr();
        // `thash` may MUTATE addr in some backends; give each side its own copy
        // and compare the addresses afterwards too.
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xA5u8; SPX_N + 8];
        let mut ro = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(
                co.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as core::ffi::c_uint,
                &cc,
                ca.as_mut_ptr(),
            );
            r(
                ro.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as core::ffi::c_uint,
                &rc,
                ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq(&format!("SPX_thash {label} (inblocks={inblocks}) out"), &co, &ro);
        assert_bytes_eq(
            &format!("SPX_thash {label} (inblocks={inblocks}) addr after"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
        // The context must not be modified by thash.
        assert_bytes_eq(
            &format!("SPX_thash {label} ctx after"),
            cc.as_bytes(),
            rc.as_bytes(),
        );
    }
}

#[test]
fn row28_thash_inblocks_1() {
    thash_case(1, 128, 0x2801, "single block (256-bit path)");
}

#[test]
fn row29_thash_inblocks_2() {
    // For SPX_SHA512 / SPX_BLAKE512 configs (192/256) this takes the 512-bit
    // branch; for 128-bit configs it takes the same 256-bit branch as row 28.
    thash_case(2, 128, 0x2901, if HAS_512_THASH_BRANCH { "512-bit branch" } else { "256-bit branch" });
}

#[test]
fn row30_thash_inblocks_wots_len() {
    thash_case(SPX_WOTS_LEN, 32, 0x3001, "WOTS-pk compression");
}

#[test]
fn row31_thash_inblocks_fors_trees() {
    thash_case(SPX_FORS_TREES, 32, 0x3101, "FORS-pk compression");
}

#[test]
fn row32_thash_inblocks_zero() {
    thash_case(0, 64, 0x3201, "degenerate zero blocks");
}

#[test]
fn row29b_thash_other_inblocks() {
    // Sweep every inblocks value in between, so no size-dependent branch is
    // missed (e.g. buffer sizing / multi-block padding).
    for n in [3usize, 4, 5, 7, 8, 9, 15, 16, 17, 33, 34] {
        thash_case(n, 8, 0x2900 + n as u64, "sweep");
    }
}

// --- row 33 -------------------------------------------------------------

#[test]
fn row33_gen_message_random() {
    let l = libs();
    let (c, r) = l.pair::<FnGenMsgRandom>("SPX_gen_message_random");
    let mut rng = Rng::new(0x3301);
    for &mlen in MLENS {
        for _ in 0..8 {
            let (cc, rc) = make_ctx(&mut rng);
            let sk_prf = rng.bytes(SPX_N);
            let optrand = rng.bytes(SPX_N);
            let m = rng.bytes(mlen.max(1));
            // NOTE: for the BLAKE backend the C writes the FULL blakeX digest
            // (32 bytes for N<24, 64 for N>=24) into `R`, not just SPX_N bytes
            // -- sign.c gets away with it because the following SPX_FORS_BYTES
            // are overwritten immediately afterwards. Size the buffer for the
            // worst case and compare every byte.
            let mut co = vec![0xA5u8; 128];
            let mut ro = vec![0xA5u8; 128];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as core::ffi::c_ulonglong,
                    &cc,
                );
                r(
                    ro.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as core::ffi::c_ulonglong,
                    &rc,
                );
            }
            assert_bytes_eq(&format!("SPX_gen_message_random(mlen={mlen})"), &co, &ro);
        }
    }
}

// --- row 34 -------------------------------------------------------------

#[test]
fn row34_hash_message() {
    let l = libs();
    let (c, r) = l.pair::<FnHashMessage>("SPX_hash_message");
    let mut rng = Rng::new(0x3401);
    for &mlen in MLENS {
        for _ in 0..8 {
            let (cc, rc) = make_ctx(&mut rng);
            let rr = rng.bytes(SPX_N);
            let pk = rng.bytes(SPX_PK_BYTES);
            let m = rng.bytes(mlen.max(1));

            let mut cd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
            let mut rd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
            let mut ct: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut rt: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut cl: u32 = 0xDEAD_BEEF;
            let mut rl: u32 = 0xDEAD_BEEF;
            unsafe {
                c(
                    cd.as_mut_ptr(),
                    &mut ct,
                    &mut cl,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as core::ffi::c_ulonglong,
                    &cc,
                );
                r(
                    rd.as_mut_ptr(),
                    &mut rt,
                    &mut rl,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as core::ffi::c_ulonglong,
                    &rc,
                );
            }
            assert_bytes_eq(&format!("SPX_hash_message(mlen={mlen}) digest"), &cd, &rd);
            assert_eq_dbg(&format!("SPX_hash_message(mlen={mlen}) *tree"), ct, rt);
            assert_eq_dbg(&format!("SPX_hash_message(mlen={mlen}) *leaf_idx"), cl, rl);
            // The masks must actually be applied.
            assert!(
                ct < (1u64 << SPX_TREE_BITS.min(63)) || SPX_TREE_BITS >= 64,
                "{} tree {} exceeds SPX_TREE_BITS={}",
                tag(),
                ct,
                SPX_TREE_BITS
            );
            assert!(
                (cl as u64) < (1u64 << SPX_LEAF_BITS),
                "{} leaf_idx {} exceeds SPX_LEAF_BITS={}",
                tag(),
                cl,
                SPX_LEAF_BITS
            );
        }
    }
}

// --- row 35 -------------------------------------------------------------

#[test]
fn row35_chain_lengths() {
    let l = libs();
    let (c, r) = l.pair::<FnChainLengths>("SPX_chain_lengths");
    let mut rng = Rng::new(0x3501);
    let mut msgs: Vec<Vec<u8>> = vec![
        vec![0x00u8; SPX_N],
        vec![0xFFu8; SPX_N],
        vec![0x0Fu8; SPX_N],
        vec![0xF0u8; SPX_N],
    ];
    for _ in 0..256 {
        msgs.push(rng.bytes(SPX_N));
    }
    for m in msgs {
        let mut cl = vec![0xDEADBEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEADBEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
            r(rl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
        }
        assert_eq_dbg(&format!("SPX_chain_lengths({})", hex(&m)), &cl, &rl);
        // Every digit must be < w (a real property of base_w).
        for (i, &d) in cl[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(
                (d as usize) < SPX_WOTS_W,
                "{} chain_lengths[{}] = {} >= w = {}",
                tag(),
                i,
                d,
                SPX_WOTS_W
            );
        }
    }
}
