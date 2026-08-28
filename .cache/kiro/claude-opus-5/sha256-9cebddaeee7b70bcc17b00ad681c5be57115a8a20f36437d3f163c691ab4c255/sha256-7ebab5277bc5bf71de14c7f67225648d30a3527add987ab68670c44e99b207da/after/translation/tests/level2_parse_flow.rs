//! Level 2: `ima_parse` control flow — file-type / version / format-id
//! validation, chunk-table walking, and where `blocks` ends up.

mod harness;
use harness::*;

fn valid_desc() -> Vec<u8> {
    desc_body_rate(44100.0, 2)
}

fn valid_pakt() -> Vec<u8> {
    pakt_body(1234, 56789, 2, 3)
}

fn valid_data() -> Vec<u8> {
    data_body(0, &[0x11u8; 68])
}

fn minimal_valid() -> Vec<u8> {
    Caf::new()
        .valid_header()
        .chunk(FOURCC_DESC, &valid_desc())
        .chunk(FOURCC_PAKT, &valid_pakt())
        .chunk(FOURCC_DATA, &valid_data())
        .build()
}

// ---------------------------------------------------------------------------
// header->type  ->  -1
// ---------------------------------------------------------------------------

#[test]
fn baseline_valid_stream() {
    let out = assert_same("minimal valid", &minimal_valid());
    assert_eq!(out.ret, 0);
    assert_eq!(out.info.frame_count(), 56789);
    assert_eq!(out.info.channel_count(), 2);
    assert_eq!(out.info.size(), valid_data().len() as u64);
}

#[test]
fn bad_file_type_returns_minus_one() {
    for ty in [
        b"CAFF", b"ffac", b"caf ", b"\0\0\0\0", b"caFf", b"cafF", b"Caff", b"aiff", b"wave",
        b"\xff\xff\xff\xff",
    ] {
        let bytes = Caf::new()
            .header(ty, 1, 0)
            .chunk(FOURCC_DESC, &valid_desc())
            .chunk(FOURCC_PAKT, &valid_pakt())
            .chunk(FOURCC_DATA, &valid_data())
            .build();
        let out = assert_same(&format!("file type {:?}", String::from_utf8_lossy(ty)), &bytes);
        assert_eq!(out.ret, -1);
    }
}

/// Vary one byte of the file type at a time across all 256 values: only the
/// exact `caff` spelling may be accepted, and the -1 path must not touch
/// `info`.
#[test]
fn file_type_single_byte_sweep() {
    for lane in 0..4usize {
        for v in 0..=255u8 {
            let mut ty = *FOURCC_CAFF;
            ty[lane] = v;
            let bytes = Caf::new()
                .header(&ty, 1, 0)
                .chunk(FOURCC_DESC, &valid_desc())
                .chunk(FOURCC_PAKT, &valid_pakt())
                .chunk(FOURCC_DATA, &valid_data())
                .build();
            let out = assert_same(&format!("file type lane={lane} v={v:#04x}"), &bytes);
            if ty == *FOURCC_CAFF {
                assert_eq!(out.ret, 0);
            } else {
                assert_eq!(out.ret, -1, "lane={lane} v={v:#04x}");
                assert_eq!(out.info.0, InfoBuf::poisoned().0, "-1 must not write info");
            }
        }
    }
}

/// `flags` is never read; both sides must ignore it.
#[test]
fn header_flags_are_ignored() {
    for flags in [0u16, 1, 0xFF, 0xFF00, 0xFFFF, 0x1234] {
        let bytes = Caf::new()
            .header(FOURCC_CAFF, 1, flags)
            .chunk(FOURCC_DESC, &valid_desc())
            .chunk(FOURCC_PAKT, &valid_pakt())
            .chunk(FOURCC_DATA, &valid_data())
            .build();
        let out = assert_same(&format!("flags={flags:#06x}"), &bytes);
        assert_eq!(out.ret, 0);
    }
}

#[test]
fn version_rejection_does_not_write_info() {
    for version in [0u16, 2, 3, 0x100, 0xFFFF] {
        let bytes = Caf::new()
            .header(FOURCC_CAFF, version, 0)
            .chunk(FOURCC_DESC, &valid_desc())
            .chunk(FOURCC_PAKT, &valid_pakt())
            .chunk(FOURCC_DATA, &valid_data())
            .build();
        let out = assert_same(&format!("version={version}"), &bytes);
        assert_eq!(out.ret, -2);
        assert_eq!(out.info.0, InfoBuf::poisoned().0);
    }
}

// ---------------------------------------------------------------------------
// desc->format_id  ->  -3
// ---------------------------------------------------------------------------

#[test]
fn format_id_single_byte_sweep() {
    for lane in 0..4usize {
        for v in 0..=255u8 {
            let mut fid = *FOURCC_IMA4;
            fid[lane] = v;
            let desc = desc_body(44100.0f64.to_bits().to_be_bytes(), &fid, 0, 34, 64, 2, 16);
            let bytes = Caf::new()
                .valid_header()
                .chunk(FOURCC_DESC, &desc)
                .chunk(FOURCC_PAKT, &valid_pakt())
                .chunk(FOURCC_DATA, &valid_data())
                .build();
            let out = assert_same(&format!("format_id lane={lane} v={v:#04x}"), &bytes);
            if fid == *FOURCC_IMA4 {
                assert_eq!(out.ret, 0);
            } else {
                assert_eq!(out.ret, -3, "lane={lane} v={v:#04x}");
                assert_eq!(out.info.0, InfoBuf::poisoned().0, "-3 must not write info");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// chunk walking
// ---------------------------------------------------------------------------

/// Every ordering of desc / pakt / data. Chunks after `data` are unreachable
/// because the loop breaks there.
#[test]
fn chunk_permutations() {
    let desc = valid_desc();
    let pakt = valid_pakt();
    let data = valid_data();

    let orders: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for order in orders {
        let mut caf = Caf::new().valid_header();
        let mut data_first = false;
        for (pos, which) in order.iter().enumerate() {
            match which {
                0 => caf = caf.chunk(FOURCC_DESC, &desc),
                1 => caf = caf.chunk(FOURCC_PAKT, &pakt),
                _ => {
                    caf = caf.chunk(FOURCC_DATA, &data);
                    if pos < 2 {
                        data_first = true;
                    }
                }
            }
        }
        let bytes = caf.build();
        let label = format!("order {order:?}");
        // If `data` comes first, `desc`/`pakt` stay NULL and the C dereferences
        // a null pointer. Skip those: undefined behaviour on both sides.
        if data_first {
            continue;
        }
        let out = assert_same(&label, &bytes);
        assert_eq!(out.ret, 0, "{label}");
    }
}

/// Unknown chunks must be skipped using the 64-bit size at offset 8 and a
/// 16-byte header.
#[test]
fn unknown_chunks_are_skipped() {
    for filler_sizes in [
        vec![0usize],
        vec![1],
        vec![7],
        vec![8],
        vec![16],
        vec![100],
        vec![0, 0, 0],
        vec![3, 5, 9, 17],
        vec![255, 1, 64],
    ] {
        let mut caf = Caf::new().valid_header();
        let mut rng = Rng::new(filler_sizes.len() as u64 * 7919);
        for (i, n) in filler_sizes.iter().enumerate() {
            let ty = [b'x', b'y', b'0' + (i as u8 % 10), b'z'];
            caf = caf.chunk(&ty, &rng.bytes(*n));
        }
        caf = caf
            .chunk(FOURCC_DESC, &valid_desc())
            .chunk(FOURCC_PAKT, &valid_pakt());
        for (i, n) in filler_sizes.iter().enumerate() {
            let ty = [b'q', b'w', b'0' + (i as u8 % 10), b'r'];
            caf = caf.chunk(&ty, &rng.bytes(*n));
        }
        caf = caf.chunk(FOURCC_DATA, &valid_data());
        let bytes = caf.build();
        let out = assert_same(&format!("fillers {filler_sizes:?}"), &bytes);
        assert_eq!(out.ret, 0);
    }
}

/// A repeated `desc` or `pakt` chunk: the last one before `data` wins.
#[test]
fn later_desc_and_pakt_override_earlier() {
    let bad_desc = desc_body(0f64.to_bits().to_be_bytes(), b"nope", 0, 0, 0, 99, 0);
    let bytes = Caf::new()
        .valid_header()
        .chunk(FOURCC_DESC, &bad_desc)
        .chunk(FOURCC_PAKT, &pakt_body(0, 111, 0, 0))
        .chunk(FOURCC_DESC, &valid_desc())
        .chunk(FOURCC_PAKT, &pakt_body(0, 222, 0, 0))
        .chunk(FOURCC_DATA, &valid_data())
        .build();
    let out = assert_same("override", &bytes);
    assert_eq!(out.ret, 0);
    assert_eq!(out.info.frame_count(), 222);
    assert_eq!(out.info.channel_count(), 2);
}

/// `blocks` must land at `data_chunk_start + 16 + 4` (chunk header plus
/// `struct caf_data`), and `info->size` must be the data chunk's size field.
#[test]
fn blocks_offset_and_size() {
    for pre_fillers in 0..4usize {
        let mut caf = Caf::new().valid_header();
        let mut offset = 8usize;
        for i in 0..pre_fillers {
            let body = vec![0xEEu8; 8 * (i + 1)];
            caf = caf.chunk(&[b'f', b'i', b'l', b'0' + i as u8], &body);
            offset += 16 + body.len();
        }
        let desc = valid_desc();
        caf = caf.chunk(FOURCC_DESC, &desc);
        offset += 16 + desc.len();
        let pakt = valid_pakt();
        caf = caf.chunk(FOURCC_PAKT, &pakt);
        offset += 16 + pakt.len();

        let data = valid_data();
        caf = caf.chunk(FOURCC_DATA, &data);
        let expected_blocks = offset + 16 + 4;

        let bytes = caf.build();
        let out = assert_same(&format!("blocks off, fillers={pre_fillers}"), &bytes);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.size(), data.len() as u64);

        // Confirm the absolute offset, not just C/Rust agreement.
        let buf = AlignedBuf::new(&bytes);
        let mut info = InfoBuf::poisoned();
        let rf = rust_ima_parse();
        assert_eq!(unsafe { rf(info.0.as_mut_ptr(), buf.ptr()) }, 0);
        assert_eq!(
            info.blocks() - buf.ptr() as u64,
            expected_blocks as u64,
            "fillers={pre_fillers}"
        );
    }
}

/// The chunk padding bytes (offsets 4..8 of `caf_chunk`) are never read.
#[test]
fn chunk_padding_is_ignored() {
    let mut bytes = minimal_valid();
    let baseline = assert_same("padding baseline", &bytes);
    let mut rng = Rng::new(0xBAD_0FAD_u64);
    // Overwrite bytes 4..8 of each of the three chunk headers.
    let mut off = 8usize;
    for len in [
        valid_desc().len(),
        valid_pakt().len(),
        valid_data().len(),
    ] {
        let junk = rng.bytes(4);
        bytes[off + 4..off + 8].copy_from_slice(&junk);
        off += 16 + len;
    }
    let out = assert_same("padding scrambled", &bytes);
    assert_eq!(out.ret, baseline.ret);
    assert_eq!(out.info.frame_count(), baseline.info.frame_count());
    assert_eq!(out.info.channel_count(), baseline.info.channel_count());
}

/// Unaligned `data` pointers: the C reads 16/32/64-bit fields straight out of
/// the buffer, so both sides must agree for every skew.
#[test]
fn unaligned_input_pointer() {
    let bytes = minimal_valid();
    for skew in 0..8usize {
        let out = assert_same_skew(&format!("skew={skew}"), &bytes, skew);
        assert_eq!(out.ret, 0);
    }
}

/// Fields the parser never reads must not influence the result.
#[test]
fn unread_desc_and_pakt_fields_are_ignored() {
    let mut rng = Rng::new(0x1615_4E4F_5245u64);
    for i in 0..200 {
        let desc = desc_body(
            44100.0f64.to_bits().to_be_bytes(),
            FOURCC_IMA4,
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            7,
            rng.next_u32(),
        );
        let pakt = pakt_body(
            rng.next_u64() as i64,
            0x1234_5678,
            rng.next_u32() as i32,
            rng.next_u32() as i32,
        );
        let bytes = Caf::new()
            .valid_header()
            .chunk(FOURCC_DESC, &desc)
            .chunk(FOURCC_PAKT, &pakt)
            .chunk(FOURCC_DATA, &data_body(rng.next_u32(), &[0u8; 34]))
            .build();
        let out = assert_same(&format!("ignored fields #{i}"), &bytes);
        assert_eq!(out.ret, 0);
        assert_eq!(out.info.frame_count(), 0x1234_5678);
        assert_eq!(out.info.channel_count(), 7);
    }
}
