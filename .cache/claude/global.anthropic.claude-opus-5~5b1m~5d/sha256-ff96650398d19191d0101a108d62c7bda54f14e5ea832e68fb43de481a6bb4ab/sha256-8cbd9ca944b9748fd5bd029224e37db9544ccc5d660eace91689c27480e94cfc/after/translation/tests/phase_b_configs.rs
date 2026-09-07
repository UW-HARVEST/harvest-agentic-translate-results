//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//! Both implementations are invoked through their `.so` exports.

mod common;
use common::*;

const N: usize = 3_000;

/// Row 1 — every explicit `cur_blocksize` case, randomized other fields.
#[test]
fn cfg_01_blocksize_explicit_cases() {
    let mut rng = Rng::new(0xB5_0001);
    for bs in BLOCKSIZE_CASES {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.cur_blocksize = bs;
            assert_same("cfg01", &t);
        }
    }
}

fn is_bs_case(v: u32) -> bool {
    BLOCKSIZE_CASES.contains(&v)
}

/// Row 2 — `default` arm with `cur_blocksize <= 256`.
#[test]
fn cfg_02_blocksize_default_low() {
    let mut rng = Rng::new(0xB5_0002);
    for &bs in &[0u32, 1, 2, 3, 191, 193, 255] {
        let mut t = rng.random_tflac();
        t.cur_blocksize = bs;
        assert_same("cfg02/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let bs = rng.range(0, 256);
        if is_bs_case(bs) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.cur_blocksize = bs;
        assert_same("cfg02/rand", &t);
        n += 1;
    }
}

/// Row 3 — `default` arm with `cur_blocksize > 256`.
#[test]
fn cfg_03_blocksize_default_high() {
    let mut rng = Rng::new(0xB5_0003);
    for &bs in &[257u32, 258, 1000, 65535, 65536, 1 << 30, u32::MAX - 1, u32::MAX] {
        let mut t = rng.random_tflac();
        t.cur_blocksize = bs;
        assert_same("cfg03/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let bs = rng.range(257, u32::MAX);
        if is_bs_case(bs) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.cur_blocksize = bs;
        assert_same("cfg03/rand", &t);
        n += 1;
    }
}

/// Row 4 — every explicit `samplerate` case.
#[test]
fn cfg_04_samplerate_explicit_cases() {
    let mut rng = Rng::new(0xB5_0004);
    for sr in SAMPLERATE_CASES {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.samplerate = sr;
            assert_same("cfg04", &t);
        }
    }
}

fn is_sr_case(v: u32) -> bool {
    SAMPLERATE_CASES.contains(&v)
}

/// Row 5 — `%1000==0 && /1000<256` → code `0x0C`.
#[test]
fn cfg_05_samplerate_khz_in_range() {
    let mut rng = Rng::new(0xB5_0005);
    for k in 0u32..256 {
        let sr = k * 1000;
        if is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg05/sweep", &t);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(0, 255) * 1000;
        if is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg05/rand", &t);
        n += 1;
    }
}

/// Row 6 — `%1000==0 && /1000>=256` → silent no-op.
#[test]
fn cfg_06_samplerate_khz_overflow() {
    let mut rng = Rng::new(0xB5_0006);
    for &sr in &[256_000u32, 257_000, 1_000_000, 4_294_000_000] {
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg06/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let k = rng.range(256, 4_294_967); // k*1000 must fit in u32
        let sr = k * 1000;
        if is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg06/rand", &t);
        n += 1;
    }
}

/// Row 7 — `%1000!=0 && <65536` → code `0x0D`.
#[test]
fn cfg_07_samplerate_hz_small() {
    let mut rng = Rng::new(0xB5_0007);
    for &sr in &[1u32, 2, 999, 1001, 22051, 44101, 65534, 65535] {
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg07/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(1, 65_535);
        if sr % 1000 == 0 || is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg07/rand", &t);
        n += 1;
    }
}

/// Row 8 — `%1000!=0 && >=65536 && %10==0 && /10<65536` → code `0x0E`.
#[test]
fn cfg_08_samplerate_dahz_in_range() {
    let mut rng = Rng::new(0xB5_0008);
    for &sr in &[65_540u32, 65_550, 100_010, 655_340, 655_350] {
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg08/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(6554, 65_535) * 10; // >=65540, /10 < 65536
        if sr % 1000 == 0 || sr < 65_536 || is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg08/rand", &t);
        n += 1;
    }
}

/// Row 9 — `%10==0 && /10>=65536` → silent no-op.
#[test]
fn cfg_09_samplerate_dahz_overflow() {
    let mut rng = Rng::new(0xB5_0009);
    for &sr in &[655_360u32, 655_370, 4_294_967_290] {
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg09/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(65_536, 429_496_729) * 10;
        if sr % 1000 == 0 || is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg09/rand", &t);
        n += 1;
    }
}

/// Row 10 — `%1000!=0 && >=65536 && %10!=0` → silent no-op.
#[test]
fn cfg_10_samplerate_no_branch() {
    let mut rng = Rng::new(0xB5_000A);
    for &sr in &[65_537u32, 65_539, 176_401, 4_294_967_295] {
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg10/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(65_536, u32::MAX);
        if sr % 10 == 0 || is_sr_case(sr) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.samplerate = sr;
        assert_same("cfg10/rand", &t);
        n += 1;
    }
}

/// Row 11 — CM 0, channels 0 (u32 wrap).
#[test]
fn cfg_11_mode0_channels_zero() {
    let mut rng = Rng::new(0xB5_000B);
    for _ in 0..N {
        let mut t = rng.random_tflac();
        t.channel_mode = (rng.next_u8() / 4) * 4; // any value with %4 == 0
        t.channels = 0;
        assert_same("cfg11", &t);
    }
}

/// Row 12 — CM 0, channels 1..=8 (legal FLAC counts).
#[test]
fn cfg_12_mode0_channels_legal() {
    let mut rng = Rng::new(0xB5_000C);
    for ch in 1u32..=8 {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.channel_mode = 0;
            t.channels = ch;
            assert_same("cfg12", &t);
        }
    }
}

/// Row 13 — CM 0, channels 9..=16 (nibble overflow).
#[test]
fn cfg_13_mode0_channels_nibble_overflow() {
    let mut rng = Rng::new(0xB5_000D);
    for ch in 9u32..=16 {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.channel_mode = 4; // %4 == 0
            t.channels = ch;
            assert_same("cfg13", &t);
        }
    }
}

/// Row 14 — CM 0, channels over the whole u32 range (shift truncation).
#[test]
fn cfg_14_mode0_channels_full_range() {
    let mut rng = Rng::new(0xB5_000E);
    for &ch in &[0x0FFF_FFFFu32, 0x1000_0000, 0x1000_0001, 0x8000_0000, u32::MAX] {
        let mut t = rng.random_tflac();
        t.channel_mode = 0;
        t.channels = ch;
        assert_same("cfg14/fixed", &t);
    }
    for _ in 0..(N * 4) {
        let mut t = rng.random_tflac();
        t.channel_mode = 0;
        t.channels = rng.spicy_u32();
        assert_same("cfg14/rand", &t);
    }
}

/// Rows 15/16/17 — CM 1, 2, 3: `channels` must be ignored.
#[test]
fn cfg_15_16_17_stereo_modes_ignore_channels() {
    let mut rng = Rng::new(0xB5_000F);
    for residue in 1u8..=3 {
        for _ in 0..(N * 2) {
            let mut t = rng.random_tflac();
            // any u8 with this %4 residue
            t.channel_mode = ((rng.next_u8() / 4) * 4).wrapping_add(residue);
            assert_eq!(t.channel_mode % 4, residue);
            t.channels = rng.spicy_u32();
            let row = match residue {
                1 => "cfg15/left_side",
                2 => "cfg16/side_right",
                _ => "cfg17/mid_side",
            };
            assert_same(row, &t);
        }
    }
}

/// Row 18 — `channel_mode` swept over the entire u8 range.
#[test]
fn cfg_18_channel_mode_full_u8_sweep() {
    let mut rng = Rng::new(0xB5_0010);
    for cm in 0u8..=255 {
        for _ in 0..64 {
            let mut t = rng.random_tflac();
            t.channel_mode = cm;
            assert_same("cfg18", &t);
        }
    }
}

/// Row 19 — every explicit `bitdepth` case.
#[test]
fn cfg_19_bitdepth_explicit_cases() {
    let mut rng = Rng::new(0xB5_0011);
    for bd in BITDEPTH_CASES {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.bitdepth = bd;
            assert_same("cfg19", &t);
        }
    }
}

/// Row 20 — `bitdepth` outside the listed set (empty `default`).
#[test]
fn cfg_20_bitdepth_default() {
    let mut rng = Rng::new(0xB5_0012);
    for bd in 0u32..=64 {
        if BITDEPTH_CASES.contains(&bd) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.bitdepth = bd;
        assert_same("cfg20/sweep", &t);
    }
    for &bd in &[u32::MAX, u32::MAX - 1, 1 << 31] {
        let mut t = rng.random_tflac();
        t.bitdepth = bd;
        assert_same("cfg20/fixed", &t);
    }
    let mut n = 0;
    while n < N {
        let bd = rng.spicy_u32();
        if BITDEPTH_CASES.contains(&bd) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.bitdepth = bd;
        assert_same("cfg20/rand", &t);
        n += 1;
    }
}

/// Row 21 — pre-existing `frame_header` must be fully overwritten.
#[test]
fn cfg_21_frame_header_preset() {
    let mut rng = Rng::new(0xB5_0013);
    for &pre in &[0u32, 0xFFFF_FFFF, 0xAAAA_AAAA, 0x5555_5555] {
        for _ in 0..N {
            let mut t = rng.random_tflac();
            t.frame_header = pre;
            assert_same("cfg21/fixed", &t);
        }
    }
    for _ in 0..N {
        let mut t = rng.random_tflac();
        t.frame_header = rng.next_u32();
        assert_same("cfg21/rand", &t);
    }
}

/// Row 22 — fully randomized 24-byte image, padding bytes included.
#[test]
fn cfg_22_full_struct_image_including_padding() {
    let mut rng = Rng::new(0xB5_0014);
    for _ in 0..(N * 10) {
        let raw = rng.random_raw();
        assert_same_raw("cfg22", &raw);
    }
    // Explicit padding patterns.
    for &fill in &[0x00u8, 0xFF, 0xA5] {
        let mut raw = rng.random_raw();
        raw.0[13] = fill;
        raw.0[14] = fill;
        raw.0[15] = fill;
        assert_same_raw("cfg22/padding", &raw);
    }
}

/// Row 23 — blocksize boundaries × samplerate boundaries.
#[test]
fn cfg_23_blocksize_x_samplerate_boundaries() {
    let mut rng = Rng::new(0xB5_0015);
    const BS: [u32; 10] = [0, 1, 192, 256, 257, 576, 4608, 32768, 32769, u32::MAX];
    const SR: [u32; 10] =
        [0, 8000, 22050, 65535, 65536, 65537, 256_000, 655_350, 655_360, u32::MAX];
    for bs in BS {
        for sr in SR {
            for _ in 0..200 {
                let mut t = rng.random_tflac();
                t.cur_blocksize = bs;
                t.samplerate = sr;
                assert_same("cfg23", &t);
            }
        }
    }
}

/// Row 24 — all CM residues × all BD states × BS default-low/high.
#[test]
fn cfg_24_mode_x_bitdepth_x_blocksize() {
    let mut rng = Rng::new(0xB5_0016);
    let mut bds: Vec<u32> = BITDEPTH_CASES.to_vec();
    bds.push(0); // representative of the empty `default`
    bds.push(33);
    for residue in 0u8..4 {
        for &bd in &bds {
            for &bs in &[100u32, 1000, 256, 257] {
                for _ in 0..200 {
                    let mut t = rng.random_tflac();
                    t.channel_mode = ((rng.next_u8() / 4) * 4).wrapping_add(residue);
                    t.bitdepth = bd;
                    t.cur_blocksize = bs;
                    assert_same("cfg24", &t);
                }
            }
        }
    }
}

/// Row 25 — unstructured fuzz over the full range of every field.
#[test]
fn cfg_25_unstructured_fuzz() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0025);
    for _ in 0..200_000 {
        let raw = rng.random_raw();
        assert_same_raw("cfg25", &raw);
    }
    // Plus a pass using field-wise "spicy" values.
    for _ in 0..100_000 {
        let t = rng.random_tflac();
        assert_same("cfg25/spicy", &t);
    }
}

/// Row 26 — repeated invocation and C-then-Rust vs Rust-then-C orderings.
#[test]
fn cfg_26_repeated_and_interleaved_invocation() {
    let p = pair();
    let mut rng = Rng::new(0xB5_0017);
    for _ in 0..(N * 5) {
        let raw = rng.random_raw();

        // Calling twice must be idempotent, identically in both.
        let c1 = p.c.call_raw(&raw);
        let c2 = p.c.call_raw(&c1);
        let r1 = p.rust.call_raw(&raw);
        let r2 = p.rust.call_raw(&r1);
        assert_eq!(c1, r1, "cfg26 first call: input {raw:?}");
        assert_eq!(c2, r2, "cfg26 second call: input {raw:?}");

        // C output fed to Rust and vice versa must land on the same value.
        let c_then_r = p.rust.call_raw(&c1);
        let r_then_c = p.c.call_raw(&r1);
        assert_eq!(c_then_r, r_then_c, "cfg26 interleaved: input {raw:?}");
        assert_eq!(c_then_r, c2, "cfg26 interleaved vs repeat: input {raw:?}");
    }
}

/// Row 27 — guard bytes before and after the struct must be untouched.
#[test]
fn cfg_27_no_out_of_bounds_writes() {
    let p = pair();
    let mut rng = Rng::new(0xB5_0018);
    const PAD: usize = 24;
    for _ in 0..(N * 2) {
        let body = rng.random_raw();

        let run = |imp: &Impl| -> Vec<u8> {
            // 4-byte aligned backing store; PAD is a multiple of 4.
            let mut words = vec![0u32; (PAD * 2 + TFLAC_SIZE) / 4];
            let buf: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, words.len() * 4)
            };
            for (i, b) in buf.iter_mut().enumerate() {
                *b = 0x5A ^ (i as u8);
            }
            buf[PAD..PAD + TFLAC_SIZE].copy_from_slice(&body.0);
            unsafe { imp.call_ptr(buf[PAD..].as_mut_ptr()) };
            buf.to_vec()
        };

        let c_buf = run(&p.c);
        let r_buf = run(&p.rust);
        assert_eq!(c_buf, r_buf, "cfg27 whole buffer diverged, body {body:?}");

        // And the guard regions specifically are pristine in both.
        for (i, b) in c_buf.iter().enumerate() {
            if i < PAD || i >= PAD + TFLAC_SIZE {
                assert_eq!(*b, 0x5A ^ (i as u8), "cfg27 C wrote out of bounds at {i}");
                assert_eq!(r_buf[i], 0x5A ^ (i as u8), "cfg27 Rust wrote out of bounds at {i}");
            }
        }
    }
}
