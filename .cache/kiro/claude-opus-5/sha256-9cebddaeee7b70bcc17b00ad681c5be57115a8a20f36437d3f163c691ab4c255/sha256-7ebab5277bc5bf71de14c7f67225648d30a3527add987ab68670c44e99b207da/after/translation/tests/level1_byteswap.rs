//! Level 1: the byte-swap primitives.
//!
//! `ima_bswap16/32/64` and `ima_btoh16/32/64` are `static` in the C and are
//! not exported, so they are exercised through the observable fields that pass
//! straight through them:
//!
//!   * `ima_btoh32` -> `info->channel_count` (from `desc->channels_per_frame`)
//!   * `ima_btoh64` -> `info->frame_count`   (from `pakt->frame_count`)
//!                  -> `info->size`          (from the data chunk's size field)
//!   * `ima_btoh16` -> the `version` check (observable as the -2 return)

mod harness;
use harness::*;

/// Builds a minimal valid stream, letting the caller override the raw bytes of
/// `desc->channels_per_frame`, `pakt->frame_count` and the data chunk size.
fn stream(chan_raw: [u8; 4], frame_raw: [u8; 8], data_size: i64) -> Vec<u8> {
    let mut desc = desc_body(
        1.0f64.to_bits().to_be_bytes(),
        FOURCC_IMA4,
        0,
        34,
        64,
        0,
        16,
    );
    desc[24..28].copy_from_slice(&chan_raw);

    let mut pakt = pakt_body(0, 0, 0, 0);
    pakt[8..16].copy_from_slice(&frame_raw);

    Caf::new()
        .valid_header()
        .chunk(FOURCC_DESC, &desc)
        .chunk(FOURCC_PAKT, &pakt)
        .chunk_raw(FOURCC_DATA, data_size, &data_body(0, &[0u8; 68]))
        .build()
}

#[test]
fn btoh32_every_single_byte_position() {
    // One byte set at a time, every value: covers all four lanes of bswap32.
    for lane in 0..4usize {
        for v in 0..=255u8 {
            let mut raw = [0u8; 4];
            raw[lane] = v;
            let bytes = stream(raw, [0; 8], 68);
            assert_same(&format!("btoh32 lane={lane} v={v:#04x}"), &bytes);
        }
    }
}

#[test]
fn btoh32_random_words() {
    let mut rng = Rng::new(0x32_5747_11);
    for i in 0..4000 {
        let raw = rng.next_u32().to_le_bytes();
        let bytes = stream(raw, [0; 8], 68);
        assert_same(&format!("btoh32 rand #{i}"), &bytes);
    }
}

#[test]
fn btoh32_edge_words() {
    for raw_u in [
        0u32,
        1,
        0xFF,
        0xFF00,
        0xFF_0000,
        0xFF00_0000,
        0x8000_0000,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x0102_0304,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x8080_8080,
        0x7F7F_7F7F,
    ] {
        let bytes = stream(raw_u.to_le_bytes(), [0; 8], 68);
        assert_same(&format!("btoh32 edge {raw_u:#010x}"), &bytes);
    }
}

#[test]
fn btoh64_every_single_byte_position() {
    for lane in 0..8usize {
        for v in 0..=255u8 {
            let mut raw = [0u8; 8];
            raw[lane] = v;
            let bytes = stream([0; 4], raw, 68);
            assert_same(&format!("btoh64 lane={lane} v={v:#04x}"), &bytes);
        }
    }
}

#[test]
fn btoh64_random_words() {
    let mut rng = Rng::new(0x64_5747_11);
    for i in 0..4000 {
        let raw = rng.next_u64().to_le_bytes();
        let bytes = stream([0; 4], raw, 68);
        assert_same(&format!("btoh64 rand #{i}"), &bytes);
    }
}

#[test]
fn btoh64_edge_words() {
    for raw_u in [
        0u64,
        1,
        0xFF,
        0xFF00,
        0xFF_0000,
        0xFF00_0000,
        0xFF_0000_0000,
        0xFF00_0000_0000,
        0xFF_0000_0000_0000,
        0xFF00_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x0102_0304_0506_0708,
        0xDEAD_BEEF_CAFE_BABE,
    ] {
        let bytes = stream([0; 4], raw_u.to_le_bytes(), 68);
        assert_same(&format!("btoh64 edge {raw_u:#018x}"), &bytes);
    }
}

/// `info->size = chunk_size`, where `chunk_size` is
/// `(ima_s64_t)ima_btoh64(chunk->size)` for the *data* chunk. Negative and
/// huge values must round-trip identically into the unsigned `size` field.
#[test]
fn data_chunk_size_sign_and_width() {
    for size in [
        0i64,
        1,
        68,
        -1,
        -68,
        i64::MIN,
        i64::MAX,
        0x0102_0304_0506_0708,
        -0x0102_0304_0506_0708,
        0xFF,
        0x100,
        0x7FFF_FFFF,
        -0x8000_0000i64,
    ] {
        let bytes = stream([0; 4], [0; 8], size);
        let out = assert_same(&format!("data size={size}"), &bytes);
        assert_eq!(out.ret, 0, "size={size} should still parse");
        assert_eq!(out.info.size(), size as u64, "size={size} field");
    }
}

/// `ima_btoh16(header->version) != 1` must reject every 16-bit value except the
/// big-endian encoding of 1. Sweeping all 65536 values pins down bswap16.
#[test]
fn btoh16_version_full_sweep() {
    let desc = desc_body_rate(44100.0, 2);
    let pakt = pakt_body(0, 0, 0, 0);
    let data = data_body(0, &[0u8; 68]);

    let cf = c_ima_parse();
    let rf = rust_ima_parse();

    for raw in 0..=u16::MAX {
        let mut bytes = Caf::new()
            .header(FOURCC_CAFF, 0, 0)
            .chunk(FOURCC_DESC, &desc)
            .chunk(FOURCC_PAKT, &pakt)
            .chunk(FOURCC_DATA, &data)
            .build();
        bytes[4..6].copy_from_slice(&raw.to_le_bytes());

        let buf = AlignedBuf::new(&bytes);
        let mut ci = InfoBuf::poisoned();
        let mut ri = InfoBuf::poisoned();
        let cr = unsafe { cf(ci.0.as_mut_ptr(), buf.ptr()) };
        let rr = unsafe { rf(ri.0.as_mut_ptr(), buf.ptr()) };
        assert_eq!(cr, rr, "version raw={raw:#06x}: C={cr} Rust={rr}");
        if cr == 0 {
            assert_eq!(ci.0, ri.0, "version raw={raw:#06x}: info mismatch");
            assert_eq!(raw, 0x0100, "only BE 1 should be accepted");
        } else {
            assert_eq!(cr, -2, "version raw={raw:#06x} should be rejected with -2");
        }
    }
}
