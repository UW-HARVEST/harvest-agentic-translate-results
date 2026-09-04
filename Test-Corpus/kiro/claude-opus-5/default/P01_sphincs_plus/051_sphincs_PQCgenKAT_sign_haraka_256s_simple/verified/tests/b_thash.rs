//! Phase B, CONFIGS.md rows 12-18: `thash` for both `THASH` variants.
//!
//! The `inblocks` argument is never range checked by the C: the scratch buffers
//! are `SPX_VLA`s sized at run time.  Rows 16-17 therefore include 0 and values
//! past anything the library uses internally.

mod common;

use common::params::*;
use common::*;

type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);

fn check_inblocks(libs: &Libs, seed: u64, inblocks: u32) {
    let (fc, fr) = libs.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(seed ^ (inblocks as u64) << 32);
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);
    let ib = inblocks as usize;

    for rep in 0..16 {
        let inp = match rep {
            0 => vec![0u8; ib * SPX_N],
            1 => vec![0xFFu8; ib * SPX_N],
            _ => rng.bytes(ib * SPX_N),
        };
        // `thash` mutates nothing in `addr`, but it is passed as `uint32_t*`,
        // so give each side its own copy and compare afterwards too.
        let base = if rep == 2 { [0u32; 8] } else { rng.addr() };
        let mut aa = base;
        let mut ab = base;
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            fc(a.as_mut_ptr(), inp.as_ptr(), inblocks, cc.ptr(), aa.as_mut_ptr());
            fr(b.as_mut_ptr(), inp.as_ptr(), inblocks, cr.ptr(), ab.as_mut_ptr());
        }
        eq(&format!("SPX_thash(inblocks={inblocks}, rep={rep})"), &a, &b);
        eq(
            &format!("SPX_thash addr side effect (inblocks={inblocks})"),
            &u32s_as_bytes(&aa),
            &u32s_as_bytes(&ab),
        );
    }
}

#[test]
fn row12_inblocks_1() {
    check_inblocks(&load(), 12, 1);
}

#[test]
fn row13_inblocks_2() {
    check_inblocks(&load(), 13, 2);
}

#[test]
fn row14_inblocks_wots_len() {
    check_inblocks(&load(), 14, SPX_WOTS_LEN as u32);
}

#[test]
fn row15_inblocks_fors_trees() {
    check_inblocks(&load(), 15, SPX_FORS_TREES as u32);
}

#[test]
fn row16_inblocks_zero() {
    check_inblocks(&load(), 16, 0);
}

#[test]
fn row17_inblocks_past_internal_max() {
    let libs = load();
    for extra in [1u32, 2, 17, 96] {
        check_inblocks(&libs, 17, THASH_MAX_INTERNAL as u32 + extra);
    }
}

/// Row 18: the same sweep once more, plus every intermediate value, so that the
/// robust variant's `inblocks * SPX_N` bitmask length is exercised at every
/// size rather than only at the four the library happens to use.
#[test]
fn row18_inblocks_dense_sweep() {
    let libs = load();
    let (fc, fr) = libs.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(18);
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);
    let top = THASH_MAX_INTERNAL as u32 + 4;
    for inblocks in 0..=top {
        let ib = inblocks as usize;
        let inp = rng.bytes(ib * SPX_N);
        let mut aa = rng.addr();
        let mut ab = aa;
        let mut a = vec![0u8; SPX_N];
        let mut b = vec![0u8; SPX_N];
        unsafe {
            fc(a.as_mut_ptr(), inp.as_ptr(), inblocks, cc.ptr(), aa.as_mut_ptr());
            fr(b.as_mut_ptr(), inp.as_ptr(), inblocks, cr.ptr(), ab.as_mut_ptr());
        }
        eq(&format!("SPX_thash dense inblocks={inblocks}"), &a, &b);
    }
    eprintln!(
        "[{}] thash inblocks swept 0..={} (internal max {})",
        tag(),
        top,
        THASH_MAX_INTERNAL
    );
}
