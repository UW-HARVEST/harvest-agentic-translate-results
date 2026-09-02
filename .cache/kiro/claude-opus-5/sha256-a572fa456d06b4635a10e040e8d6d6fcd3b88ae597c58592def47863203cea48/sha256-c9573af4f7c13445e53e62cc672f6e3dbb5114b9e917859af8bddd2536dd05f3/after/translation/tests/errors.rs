//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! Every test constructs the exact invalid input, calls BOTH `.so`s and asserts
//! the *same* error code (not merely "both failed"), plus that neither wrote
//! anything into the caller's `ima_info`.

mod common;

use common::*;

const SEED: u64 = 0x0BADF00D_5EED0001;

/// The C returns before touching `*info` on every error path, so the sentinel
/// must survive verbatim.
#[track_caller]
fn expect_err(label: &str, bytes: &[u8], off: usize, want: i32) {
    let o = assert_same(label, bytes, off);
    assert_eq!(o.ret, want, "{label}: expected {want}, got {o:?}");
    let s = ImaInfo::sentinel();
    assert_eq!(o.blocks, s.blocks as usize, "{label}: *info was modified");
    assert_eq!(o.size, s.size, "{label}: *info was modified");
    assert_eq!(
        o.sample_bits,
        s.sample_rate.to_bits(),
        "{label}: *info was modified"
    );
    assert_eq!(o.frame_count, s.frame_count, "{label}: *info was modified");
    assert_eq!(
        o.channel_count, s.channel_count,
        "{label}: *info was modified"
    );
}

fn desc_body_with_format(rng: &mut Rng, format_id: [u8; 4]) -> Vec<u8> {
    DescBody {
        sample_rate_bits: rng.u64(),
        format_id,
        format_flags: rng.u32(),
        bytes_per_packet: rng.u32(),
        frames_per_packet: rng.u32(),
        channels_per_frame: rng.u32(),
        bits_per_channel: rng.u32(),
    }
    .bytes()
}

fn pakt_any(rng: &mut Rng) -> Chunk {
    let fc = rng.u64() as i64;
    let body = PaktBody {
        packet_count: rng.u64() as i64,
        frame_count: fc,
        priming_frames: rng.u32() as i32,
        remainder_frames: rng.u32() as i32,
    }
    .bytes();
    Chunk::exact(FOURCC_PAKT, body)
}

fn data_any(rng: &mut Rng) -> Chunk {
    let payload = data_payload(rng.u32(), &rng.bytes(68));
    Chunk::exact(FOURCC_DATA, payload)
}

/// A complete, well-formed file except that `desc.format_id` is `format_id`.
fn file_with_format(rng: &mut Rng, format_id: [u8; 4]) -> File {
    let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(rng, format_id));
    let p = pakt_any(rng);
    let da = data_any(rng);
    let flags = rng.u16();
    with_tail(build_valid(flags, &[d, p, da]), 64, rng)
}

// ===========================================================================
// E1 — header magic must be `caff`  =>  -1
// ===========================================================================

#[test]
fn e1a_zero_magic() {
    let mut rng = Rng::new(SEED ^ 0x101);
    let f = with_tail(build([0, 0, 0, 0], 1, 0, &[]), 128, &mut rng);
    expect_err("E1a", &f.bytes, 0, -1);
}

#[test]
fn e1b_little_endian_spelling_of_the_literal() {
    // The C literal is 'f' | 'f'<<8 | 'a'<<16 | 'c'<<24; its *native* byte
    // order is "ffac". Because the read is byte-swapped, only "caff" matches.
    let mut rng = Rng::new(SEED ^ 0x102);
    let f = with_tail(build(*b"ffac", 1, 0, &[]), 128, &mut rng);
    expect_err("E1b", &f.bytes, 0, -1);
}

#[test]
fn e1c_near_miss_magics() {
    let mut rng = Rng::new(SEED ^ 0x103);
    let mut cases: Vec<[u8; 4]> = vec![
        *b"caf\0", *b"Caff", *b"cafF", *b"cAff", *b"caFf", b"caff".map(|c| c ^ 0x01),
        *b"ffca", *b"affc", *b"fcaf", *b"CAFF", b"caff".map(|c| c.wrapping_add(1)),
    ];
    // every single-byte perturbation of "caff", every delta 1..=255
    for i in 0..4usize {
        for d in 1u8..=255 {
            let mut m = *b"caff";
            m[i] = m[i].wrapping_add(d);
            cases.push(m);
        }
    }
    for m in cases {
        if &m == b"caff" {
            continue;
        }
        let f = with_tail(build(m, 1, rng.u16(), &[]), 128, &mut rng);
        expect_err("E1c", &f.bytes, 0, -1);
    }
}

#[test]
fn e1d_randomized_magics() {
    let mut rng = Rng::new(SEED ^ 0x104);
    let mut n = 0;
    while n < 2048 {
        let m = rng.u32().to_be_bytes();
        if &m == b"caff" {
            continue;
        }
        let ver = rng.u16();
        let fl = rng.u16();
        let f = with_tail(build(m, ver, fl, &[]), 128, &mut rng);
        expect_err("E1d", &f.bytes, 0, -1);
        n += 1;
    }
}

#[test]
fn e1e_magic_is_checked_before_everything_else() {
    // A file that is *entirely* garbage after the magic still gets -1, proving
    // the magic check happens first and reads nothing else.
    let mut rng = Rng::new(SEED ^ 0x105);
    for _ in 0..256 {
        let mut bytes = rng.bytes(8 + 64);
        let m = loop {
            let c = rng.u32().to_be_bytes();
            if &c != b"caff" {
                break c;
            }
        };
        bytes[0..4].copy_from_slice(&m);
        expect_err("E1e", &bytes, 0, -1);
    }
}

// ===========================================================================
// E2 — header version must be 1  =>  -2
// ===========================================================================

#[test]
fn e2a_version_zero() {
    let mut rng = Rng::new(SEED ^ 0x201);
    let f = with_tail(build(*b"caff", 0, 0, &[]), 128, &mut rng);
    expect_err("E2a", &f.bytes, 0, -2);
}

#[test]
fn e2b_version_two_one_past_valid() {
    let mut rng = Rng::new(SEED ^ 0x202);
    let f = with_tail(build(*b"caff", 2, 0, &[]), 128, &mut rng);
    expect_err("E2b", &f.bytes, 0, -2);
}

#[test]
fn e2c_version_ffff() {
    let mut rng = Rng::new(SEED ^ 0x203);
    let f = with_tail(build(*b"caff", 0xFFFF, 0, &[]), 128, &mut rng);
    expect_err("E2c", &f.bytes, 0, -2);
}

#[test]
fn e2d_version_native_endian_one() {
    // 0x0100 big-endian == 1 little-endian; must still be rejected.
    let mut rng = Rng::new(SEED ^ 0x204);
    let f = with_tail(build(*b"caff", 0x0100, 0, &[]), 128, &mut rng);
    expect_err("E2d", &f.bytes, 0, -2);
}

#[test]
fn e2e_all_versions_rejected() {
    // Exhaustive over the whole 16-bit space: this also proves bswap16.
    let mut rng = Rng::new(SEED ^ 0x205);
    let tail: Vec<u8> = rng.bytes(64);
    for v in 0u32..=0xFFFF {
        let v = v as u16;
        if v == 1 {
            continue;
        }
        let mut bytes = Vec::with_capacity(8 + tail.len());
        bytes.extend_from_slice(b"caff");
        bytes.extend_from_slice(&v.to_be_bytes());
        bytes.extend_from_slice(&0xBEEFu16.to_be_bytes());
        bytes.extend_from_slice(&tail);
        let buf = Buf::new(&bytes, 0);
        let (c, r) = call_both(&buf);
        assert_eq!(c, r, "E2e divergence at version {v:#06x}: C={c:?} R={r:?}");
        assert_eq!(c.ret, -2, "E2e version {v:#06x}");
    }
}

#[test]
fn e2f_flags_do_not_affect_the_version_check() {
    let mut rng = Rng::new(SEED ^ 0x206);
    for fl in [0x0000u16, 0xFFFF, 0x00FF, 0xFF00, 0x1234] {
        let f = with_tail(build(*b"caff", 0, fl, &[]), 128, &mut rng);
        expect_err("E2f/-2", &f.bytes, 0, -2);
        // and with a valid version the same flags must let the parse proceed
        let bad = file_with_format(&mut rng, *b"XXXX");
        let mut b2 = bad.bytes.clone();
        b2[6..8].copy_from_slice(&fl.to_be_bytes());
        expect_err("E2f/-3", &b2, 0, -3);
    }
}

// ===========================================================================
// E3 — desc.format_id must be `ima4`  =>  -3
// ===========================================================================

#[test]
fn e3a_format_id_zero() {
    let mut rng = Rng::new(SEED ^ 0x301);
    let f = file_with_format(&mut rng, [0, 0, 0, 0]);
    expect_err("E3a", &f.bytes, 0, -3);
}

#[test]
fn e3b_format_id_little_endian_spelling() {
    // The C literal is '4' | 'a'<<8 | 'm'<<16 | 'i'<<24 => native bytes "4ami".
    let mut rng = Rng::new(SEED ^ 0x302);
    let f = file_with_format(&mut rng, *b"4ami");
    expect_err("E3b", &f.bytes, 0, -3);
}

#[test]
fn e3c_format_id_single_byte_perturbations() {
    let mut rng = Rng::new(SEED ^ 0x303);
    for i in 0..4usize {
        for d in [1u8, 2, 0x20, 0x80, 0xFF] {
            let mut m = FOURCC_IMA4;
            m[i] = m[i].wrapping_add(d);
            if m == FOURCC_IMA4 {
                continue;
            }
            let f = file_with_format(&mut rng, m);
            expect_err("E3c", &f.bytes, 0, -3);
        }
    }
}

#[test]
fn e3d_other_known_codec_fourccs() {
    let mut rng = Rng::new(SEED ^ 0x304);
    for m in [
        *b"alac", *b"lpcm", *b"ima5", *b"IMA4", *b"aac ", *b"ulaw", *b"alaw", *b"MAC3",
        b"ima4".map(|c| c ^ 0x20),
    ] {
        let f = file_with_format(&mut rng, m);
        expect_err("E3d", &f.bytes, 0, -3);
    }
}

#[test]
fn e3e_randomized_format_ids() {
    let mut rng = Rng::new(SEED ^ 0x305);
    let mut n = 0;
    while n < 1024 {
        let m = rng.u32().to_be_bytes();
        if m == FOURCC_IMA4 {
            continue;
        }
        let f = file_with_format(&mut rng, m);
        expect_err("E3e", &f.bytes, 0, -3);
        n += 1;
    }
}

#[test]
fn e3f_desc_after_pakt_with_bad_format() {
    let mut rng = Rng::new(SEED ^ 0x306);
    for _ in 0..256 {
        let m = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let p = pakt_any(&mut rng);
        let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, m));
        let da = data_any(&mut rng);
        let flags = rng.u16();
        let f = with_tail(build_valid(flags, &[p, d, da]), 64, &mut rng);
        expect_err("E3f", &f.bytes, 0, -3);
    }
}

#[test]
fn e3g_duplicate_desc_last_is_bad() {
    let mut rng = Rng::new(SEED ^ 0x307);
    for _ in 0..256 {
        let good = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, FOURCC_IMA4));
        let bad_cc = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let bad = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_cc));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);
        let flags = rng.u16();
        let f = with_tail(build_valid(flags, &[good, p, bad, da]), 64, &mut rng);
        expect_err("E3g", &f.bytes, 0, -3);
    }
}

#[test]
fn e3h_duplicate_desc_last_is_good() {
    let mut rng = Rng::new(SEED ^ 0x308);
    for _ in 0..256 {
        let bad_cc = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let bad = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_cc));
        let good = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, FOURCC_IMA4));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);
        let flags = rng.u16();
        let f = with_tail(build_valid(flags, &[bad, p, good, da]), 64, &mut rng);
        let o = assert_same("E3h", &f.bytes, 0);
        assert_eq!(o.ret, 0, "E3h: the LAST desc must win, got {o:?}");
    }
}

// ===========================================================================
// G1 — `info == NULL` on every error path (nothing is written before the
// early returns, so a NULL `info` must be harmless and give the same code)
// ===========================================================================

#[test]
fn g1_null_info_on_all_error_paths() {
    let mut rng = Rng::new(SEED ^ 0x401);
    let cases: Vec<(&str, Vec<u8>, i32)> = vec![
        (
            "-1",
            with_tail(build(*b"junk", 1, 0, &[]), 128, &mut rng).bytes,
            -1,
        ),
        (
            "-2",
            with_tail(build(*b"caff", 7, 0, &[]), 128, &mut rng).bytes,
            -2,
        ),
        ("-3", file_with_format(&mut rng, *b"nope").bytes, -3),
    ];
    for (label, bytes, want) in cases {
        let buf = Buf::new(&bytes, 0);
        let (c, r) = call_both_null_info(&buf);
        assert_eq!(c, r, "G1/{label}: C={c} RUST={r}");
        assert_eq!(c, want, "G1/{label}");
    }
}

// ===========================================================================
// G2 — the same error paths with unaligned `data` pointers
// ===========================================================================

#[test]
fn g2_error_paths_with_unaligned_buffers() {
    let mut rng = Rng::new(SEED ^ 0x402);
    for off in 0..16usize {
        let a = with_tail(build(*b"junk", 1, rng.u16(), &[]), 128, &mut rng);
        expect_err("G2/-1", &a.bytes, off, -1);
        let b = with_tail(build(*b"caff", 3, rng.u16(), &[]), 128, &mut rng);
        expect_err("G2/-2", &b.bytes, off, -2);
        let c = file_with_format(&mut rng, *b"zzzz");
        expect_err("G2/-3", &c.bytes, off, -3);
    }
}

// ===========================================================================
// Generic boundaries: truncated / minimal buffers
// ===========================================================================

#[test]
fn g3_minimal_and_truncated_buffers() {
    let mut rng = Rng::new(SEED ^ 0x403);
    // 0..8 bytes of real content, zero-filled beyond (Buf over-allocates), so
    // the reads stay inside the mapping just as they would in a C caller that
    // over-allocates.
    for n in 0..=8usize {
        let bytes = vec![0u8; n];
        expect_err("G3/zeros", &bytes, 0, -1);
    }
    // exactly the 8-byte header, magic ok, version 0 -> -2
    let mut hdr = Vec::new();
    hdr.extend_from_slice(b"caff");
    hdr.extend_from_slice(&0u16.to_be_bytes());
    hdr.extend_from_slice(&0u16.to_be_bytes());
    expect_err("G3/hdr-only", &hdr, 0, -2);
    // magic split across the truncation point
    for n in 1..4usize {
        let mut b = b"caff"[..n].to_vec();
        b.extend(rng.bytes(0));
        expect_err("G3/partial-magic", &b, 0, -1);
    }
}

// ===========================================================================
// G4 — out-of-range "enum" values across the FFI boundary.
//
// `chunk->type` (u32), `header->version` (u16) and `desc->format_id` (u32) are
// all plain integers in C: any value with no matching constant is a real input.
// ===========================================================================

#[test]
fn g4_out_of_range_chunk_types_before_a_failing_format_check() {
    let mut rng = Rng::new(SEED ^ 0x404);
    let mut probes: Vec<u32> = vec![0, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    for base in [FOURCC_DESC, FOURCC_PAKT, FOURCC_DATA, FOURCC_IMA4] {
        let v = u32::from_be_bytes(base);
        probes.extend([v.wrapping_sub(1), v.wrapping_add(1), !v, v.swap_bytes()]);
    }
    let mut r2 = Rng::new(SEED ^ 0x4444);
    for _ in 0..128 {
        probes.push(r2.u32());
    }
    for v in probes {
        let cc = v.to_be_bytes();
        if cc == FOURCC_DESC || cc == FOURCC_PAKT || cc == FOURCC_DATA {
            continue;
        }
        let len = rng.below(40);
        let odd = Chunk::exact(cc, rng.bytes(len));
        let bad_fmt = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_fmt));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);
        let flags = rng.u16();
        let f = with_tail(build_valid(flags, &[odd, d, p, da]), 64, &mut rng);
        expect_err("G4", &f.bytes, 0, -3);
    }
}

#[test]
fn g4b_out_of_range_versions_and_formats_combined() {
    let mut rng = Rng::new(SEED ^ 0x405);
    // version wrong AND format wrong: the version check must win (-2, not -3).
    for _ in 0..256 {
        let bad_fmt = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_fmt));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);
        let ver = loop {
            let v = rng.u16();
            if v != 1 {
                break v;
            }
        };
        let flags = rng.u16();
        let f = with_tail(build(*b"caff", ver, flags, &[d, p, da]), 64, &mut rng);
        expect_err("G4b", &f.bytes, 0, -2);
    }
    // magic wrong AND version wrong AND format wrong: magic wins (-1).
    for _ in 0..256 {
        let bad_fmt = rng.u32().to_be_bytes();
        let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_fmt));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);
        let magic = loop {
            let m = rng.u32().to_be_bytes();
            if &m != b"caff" {
                break m;
            }
        };
        let ver = rng.u16();
        let flags = rng.u16();
        let f = with_tail(build(magic, ver, flags, &[d, p, da]), 64, &mut rng);
        expect_err("G4b/-1", &f.bytes, 0, -1);
    }
}

// ===========================================================================
// G5 — a negative-size skip chunk on the way to a *failing* format check
// ===========================================================================

#[test]
fn g5_negative_skip_then_error() {
    let mut rng = Rng::new(SEED ^ 0x406);
    for _ in 0..128 {
        let bad_fmt = loop {
            let c = rng.u32().to_be_bytes();
            if c != FOURCC_IMA4 {
                break c;
            }
        };
        let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, bad_fmt));
        let p = pakt_any(&mut rng);
        let da = data_any(&mut rng);

        let off_d = FILE_HDR;
        let off_p = off_d + d.total();
        let off_j = off_p + p.total();
        let off_da = off_j + CHUNK_HDR;
        let off_k = off_da + da.total();
        let j = Chunk {
            fourcc: unknown_fourcc(&mut rng),
            pad: rng.u32().to_be_bytes(),
            size: off_k as i64 - off_j as i64 - CHUNK_HDR as i64,
            payload: Vec::new(),
        };
        let k = Chunk {
            fourcc: unknown_fourcc(&mut rng),
            pad: rng.u32().to_be_bytes(),
            size: off_da as i64 - off_k as i64 - CHUNK_HDR as i64,
            payload: Vec::new(),
        };
        assert!(k.size < 0);
        let flags = rng.u16();
        let f = with_tail(build_valid(flags, &[d, p, j, da, k]), 64, &mut rng);
        expect_err("G5", &f.bytes, 0, -3);
    }
}

// ===========================================================================
// U1 / U3 / U4 — documented UB. Executed in *child processes* so we can compare
// the fault behaviour of the two libraries without killing the test binary.
// ===========================================================================

fn child_env() -> bool {
    std::env::var_os("IMA_UB_CHILD").is_some()
}

#[cfg(unix)]
fn run_child(name: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args([name, "--exact", "--ignored", "--test-threads=1", "--nocapture"])
        .env("IMA_UB_CHILD", "1")
        .output()
        .expect("spawn child test");
    (out.status.code(), out.status.signal())
}

/// Helper: the raw call used by the ignored child tests.
fn raw_call(which: usize, data: *const std::ffi::c_void) -> i32 {
    let mut info = ImaInfo::sentinel();
    unsafe {
        let (c, r) = fn_ptrs();
        let f: ImaParseFn = std::mem::transmute(if which == 0 { c } else { r });
        f(&mut info, data)
    }
}

#[test]
#[ignore]
fn ub_child_null_data_c() {
    if !child_env() {
        return;
    }
    let _ = raw_call(0, std::ptr::null());
}

#[test]
#[ignore]
fn ub_child_null_data_rust() {
    if !child_env() {
        return;
    }
    let _ = raw_call(1, std::ptr::null());
}

fn file_missing_desc(rng: &mut Rng) -> Vec<u8> {
    // valid header, pakt then data, NO desc  =>  desc == NULL at the format check
    let p = pakt_any(rng);
    let da = data_any(rng);
    let flags = rng.u16();
    with_tail(build_valid(flags, &[p, da]), 64, rng).bytes
}

fn file_missing_pakt(rng: &mut Rng) -> Vec<u8> {
    // valid header, desc (ima4) then data, NO pakt  =>  pakt == NULL
    let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(rng, FOURCC_IMA4));
    let da = data_any(rng);
    let flags = rng.u16();
    with_tail(build_valid(flags, &[d, da]), 64, rng).bytes
}

#[test]
#[ignore]
fn ub_child_null_desc_c() {
    if !child_env() {
        return;
    }
    let mut rng = Rng::new(1);
    let b = file_missing_desc(&mut rng);
    let buf = Buf::new(&b, 0);
    let _ = raw_call(0, buf.ptr());
}

#[test]
#[ignore]
fn ub_child_null_desc_rust() {
    if !child_env() {
        return;
    }
    let mut rng = Rng::new(1);
    let b = file_missing_desc(&mut rng);
    let buf = Buf::new(&b, 0);
    let _ = raw_call(1, buf.ptr());
}

#[test]
#[ignore]
fn ub_child_null_pakt_c() {
    if !child_env() {
        return;
    }
    let mut rng = Rng::new(2);
    let b = file_missing_pakt(&mut rng);
    let buf = Buf::new(&b, 0);
    let _ = raw_call(0, buf.ptr());
}

#[test]
#[ignore]
fn ub_child_null_pakt_rust() {
    if !child_env() {
        return;
    }
    let mut rng = Rng::new(2);
    let b = file_missing_pakt(&mut rng);
    let buf = Buf::new(&b, 0);
    let _ = raw_call(1, buf.ptr());
}

#[cfg(unix)]
#[test]
fn u1_u3_u4_null_derefs_fault_identically_in_both() {
    if child_env() {
        return; // never recurse
    }
    for (label, cname, rname) in [
        ("U1 data==NULL", "ub_child_null_data_c", "ub_child_null_data_rust"),
        ("U3 desc==NULL", "ub_child_null_desc_c", "ub_child_null_desc_rust"),
        ("U4 pakt==NULL", "ub_child_null_pakt_c", "ub_child_null_pakt_rust"),
    ] {
        let (cc, cs) = run_child(cname);
        let (rc, rs) = run_child(rname);
        eprintln!("{label}: C exit={cc:?} sig={cs:?}   RUST exit={rc:?} sig={rs:?}");
        assert!(
            cs.is_some(),
            "{label}: the C implementation was expected to fault, got exit code {cc:?}"
        );
        assert_eq!(
            cs, rs,
            "{label}: C faulted with signal {cs:?} but Rust with {rs:?}"
        );
    }
}

/// U2 (`data` chunk never found) is deliberately not executed: the C walks off
/// the buffer with an unbounded pointer stride and either faults at an
/// unpredictable address or loops forever. The Rust performs the identical
/// pointer walk (`byte_add(chunk, 16).wrapping_offset(size)`), so there is no
/// behaviour to compare that is reproducible enough to assert on.
#[test]
fn u2_unterminated_chunk_walk_is_documented_only() {
    // Structural check: a file with no `data` chunk is exactly the U2 input.
    let mut rng = Rng::new(SEED ^ 0x501);
    let d = Chunk::exact(FOURCC_DESC, desc_body_with_format(&mut rng, FOURCC_IMA4));
    let p = pakt_any(&mut rng);
    let f = build_valid(0, &[d, p]);
    assert!(
        !f.bytes.windows(4).any(|w| w == FOURCC_DATA),
        "U2 fixture must not contain a data chunk"
    );
}
