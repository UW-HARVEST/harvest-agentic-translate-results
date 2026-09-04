//! Phase B, CONFIGS.md rows 1-7: the address setters/copiers of
//! `app/src/address.c` and the scalar conversions of `app/src/utils.c`.

mod common;

use common::params::*;
use common::*;

type SetU32 = unsafe extern "C" fn(*mut u32, u32);
type SetU64 = unsafe extern "C" fn(*mut u32, u64);
type Copy2 = unsafe extern "C" fn(*mut u32, *const u32);

/// Runs a `void f(uint32_t addr[8], uint32_t v)` setter on both sides over a
/// value sweep and a randomised set of start addresses.
fn check_u32_setter(libs: &Libs, name: &str, values: &[u32]) {
    let (fc, fr) = libs.pair::<SetU32>(name);
    let mut rng = Rng::new(0x5157_u64 ^ name.len() as u64);
    for round in 0..64u32 {
        let start = if round == 0 { [0u32; 8] } else { rng.addr() };
        for &v in values {
            let mut a = start;
            let mut b = start;
            unsafe {
                fc(a.as_mut_ptr(), v);
                fr(b.as_mut_ptr(), v);
            }
            eq(
                &format!("{name}(v={v:#x}) round {round}"),
                &u32s_as_bytes(&a),
                &u32s_as_bytes(&b),
            );
        }
    }
}

#[test]
fn row01_single_byte_setters() {
    let libs = load();
    // 0..=255 plus values whose low byte must survive truncation.
    let mut vals: Vec<u32> = (0u32..=255).collect();
    vals.extend_from_slice(&[
        0x100, 0x101, 0x1FF, 0xDEAD_BEEF, 0xFFFF_FF00, 0xFFFF_FFFF, 0x7FFF_FFFF,
    ]);
    for name in [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ] {
        check_u32_setter(&libs, name, &vals);
    }
}

#[test]
fn row02_set_tree_addr() {
    let libs = load();
    let (fc, fr) = libs.pair::<SetU64>("SPX_set_tree_addr");
    let mut rng = Rng::new(2);
    let mut vals: Vec<u64> = vec![0, 1, 2, u64::MAX, u64::MAX - 1];
    for k in 0..64 {
        vals.push(1u64 << k);
        vals.push((1u64 << k).wrapping_sub(1));
    }
    for _ in 0..64 {
        vals.push(rng.next_u64());
    }
    for round in 0..8 {
        let start = if round == 0 { [0u32; 8] } else { rng.addr() };
        for &v in &vals {
            let mut a = start;
            let mut b = start;
            unsafe {
                fc(a.as_mut_ptr(), v);
                fr(b.as_mut_ptr(), v);
            }
            eq(
                &format!("SPX_set_tree_addr({v:#x})"),
                &u32s_as_bytes(&a),
                &u32s_as_bytes(&b),
            );
        }
    }
}

#[test]
fn row03_four_byte_setters() {
    let libs = load();
    let mut rng = Rng::new(3);
    let mut vals: Vec<u32> = vec![0, 1, 2, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];
    for k in 0..32 {
        vals.push(1u32 << k);
    }
    for _ in 0..128 {
        vals.push(rng.next_u32());
    }
    for name in ["SPX_set_keypair_addr", "SPX_set_tree_index"] {
        check_u32_setter(&libs, name, &vals);
    }
}

#[test]
fn row04_copiers() {
    let libs = load();
    let mut rng = Rng::new(4);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let (fc, fr) = libs.pair::<Copy2>(name);
        for _ in 0..256 {
            let src = rng.addr();
            let dst = rng.addr();
            let mut a = dst;
            let mut b = dst;
            unsafe {
                fc(a.as_mut_ptr(), src.as_ptr());
                fr(b.as_mut_ptr(), src.as_ptr());
            }
            eq(name, &u32s_as_bytes(&a), &u32s_as_bytes(&b));
        }
        // all-zero and all-ones extremes
        for (src, dst) in [([0u32; 8], [0xFFFF_FFFFu32; 8]), ([0xFFFF_FFFFu32; 8], [0u32; 8])] {
            let mut a = dst;
            let mut b = dst;
            unsafe {
                fc(a.as_mut_ptr(), src.as_ptr());
                fr(b.as_mut_ptr(), src.as_ptr());
            }
            eq(name, &u32s_as_bytes(&a), &u32s_as_bytes(&b));
        }
    }
}

type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type U32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;

#[test]
fn row05_ull_to_bytes() {
    let libs = load();
    let (fc, fr) = libs.pair::<UllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(5);
    let mut vals: Vec<u64> = vec![0, 1, 0xFF, 0x100, 0x0102_0304_0506_0708, u64::MAX];
    for _ in 0..64 {
        vals.push(rng.next_u64());
    }
    for &outlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 32] {
        for &v in &vals {
            // sentinel-filled buffers, so an under- or over-write shows up
            let mut a = vec![0xA5u8; outlen + 8];
            let mut b = vec![0xA5u8; outlen + 8];
            unsafe {
                fc(a.as_mut_ptr(), outlen as u32, v);
                fr(b.as_mut_ptr(), outlen as u32, v);
            }
            eq(&format!("SPX_ull_to_bytes(outlen={outlen}, {v:#x})"), &a, &b);
        }
    }
}

#[test]
fn row06_u32_to_bytes() {
    let libs = load();
    let (fc, fr) = libs.pair::<U32ToBytes>("SPX_u32_to_bytes");
    let mut rng = Rng::new(6);
    let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0xFFFF_FFFF];
    for _ in 0..256 {
        vals.push(rng.next_u32());
    }
    for &v in &vals {
        let mut a = [0xA5u8; 12];
        let mut b = [0xA5u8; 12];
        unsafe {
            fc(a.as_mut_ptr(), v);
            fr(b.as_mut_ptr(), v);
        }
        eq(&format!("SPX_u32_to_bytes({v:#x})"), &a, &b);
    }
}

#[test]
fn row07_bytes_to_ull() {
    let libs = load();
    let (fc, fr) = libs.pair::<BytesToUll>("SPX_bytes_to_ull");
    let (uc, _ur) = libs.pair::<UllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(7);
    for &inlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8] {
        for _ in 0..64 {
            let inp = rng.bytes(inlen.max(1));
            let vc = unsafe { fc(inp.as_ptr(), inlen as u32) };
            let vr = unsafe { fr(inp.as_ptr(), inlen as u32) };
            assert_eq!(
                vc, vr,
                "SPX_bytes_to_ull(inlen={inlen}) on {}: {vc:#x} vs {vr:#x}",
                hex(&inp)
            );
        }
        // round trip against ull_to_bytes for the widths that fit in u64
        if inlen <= 8 {
            for _ in 0..64 {
                let v = rng.next_u64() >> (64 - 8 * inlen.max(1)).min(63);
                let mut enc = vec![0u8; inlen];
                unsafe { uc(enc.as_mut_ptr(), inlen as u32, v) };
                let back_c = unsafe { fc(enc.as_ptr(), inlen as u32) };
                let back_r = unsafe { fr(enc.as_ptr(), inlen as u32) };
                assert_eq!(back_c, back_r, "round trip inlen={inlen} v={v:#x}");
            }
        }
    }
    // all-zero / all-ones extremes
    for inlen in 0..=8usize {
        for fillv in [0x00u8, 0xFFu8] {
            let inp = vec![fillv; inlen.max(1)];
            let vc = unsafe { fc(inp.as_ptr(), inlen as u32) };
            let vr = unsafe { fr(inp.as_ptr(), inlen as u32) };
            assert_eq!(vc, vr, "SPX_bytes_to_ull(inlen={inlen}, fill={fillv:#x})");
        }
    }
    let _ = SPX_N;
}
