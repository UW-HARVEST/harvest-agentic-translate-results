//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH `.so`s through `libloading` and compares the returned
//! `int` plus all five `ima_info` fields byte-for-byte.

mod common;

use common::*;

const SEED: u64 = 0x1BADB002_C0FFEE01;
const N: usize = 256;

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

fn desc_chunk(rng: &mut Rng, sample_bits: u64, channels: u32) -> Chunk {
    let body = DescBody {
        sample_rate_bits: sample_bits,
        format_id: FOURCC_IMA4,
        format_flags: rng.u32(),
        bytes_per_packet: rng.u32(),
        frames_per_packet: rng.u32(),
        channels_per_frame: channels,
        bits_per_channel: rng.u32(),
    };
    Chunk::exact(FOURCC_DESC, body.bytes()).with_pad(rng.u32().to_be_bytes())
}

fn pakt_chunk(rng: &mut Rng, frame_count: i64) -> Chunk {
    let body = PaktBody {
        packet_count: rng.u64() as i64,
        frame_count,
        priming_frames: rng.u32() as i32,
        remainder_frames: rng.u32() as i32,
    };
    Chunk::exact(FOURCC_PAKT, body.bytes()).with_pad(rng.u32().to_be_bytes())
}

fn data_chunk(rng: &mut Rng, declared_size: Option<i64>, nblocks: usize) -> Chunk {
    let blocks = rng.bytes(nblocks * 34);
    let payload = data_payload(rng.u32(), &blocks);
    let mut c = Chunk::exact(FOURCC_DATA, payload).with_pad(rng.u32().to_be_bytes());
    if let Some(s) = declared_size {
        c = c.with_size(s);
    }
    c
}

fn unknown_chunk(rng: &mut Rng, len: usize) -> Chunk {
    let cc = unknown_fourcc(rng);
    Chunk::exact(cc, rng.bytes(len)).with_pad(rng.u32().to_be_bytes())
}

/// `desc` chunk with a fully random (valid-`ima4`) body.
fn desc_rand(rng: &mut Rng) -> Chunk {
    let bits = rng.u64();
    let ch = rng.u32();
    desc_chunk(rng, bits, ch)
}

/// `pakt` chunk with a fully random body.
fn pakt_rand(rng: &mut Rng) -> Chunk {
    let fc = rng.u64() as i64;
    pakt_chunk(rng, fc)
}

/// `desc` chunk with an explicit sample-rate bit pattern, random channels.
fn desc_bits(rng: &mut Rng, bits: u64) -> Chunk {
    let ch = rng.u32();
    desc_chunk(rng, bits, ch)
}

/// `desc` chunk with explicit channels, random sample rate.
fn desc_ch(rng: &mut Rng, ch: u32) -> Chunk {
    let bits = rng.u64();
    desc_chunk(rng, bits, ch)
}

/// `data` chunk with a random number of blocks.
fn data_rand(rng: &mut Rng, declared: Option<i64>) -> Chunk {
    let n = rng.below(4);
    data_chunk(rng, declared, n)
}

/// unknown chunk with a random payload length in `0..max`.
fn unknown_rand(rng: &mut Rng, max: usize) -> Chunk {
    let l = rng.below(max);
    unknown_chunk(rng, l)
}

/// Fully random valid file (random sample rate, channels and frame count).
fn simple_rand(rng: &mut Rng) -> File {
    let bits = rng.u64();
    let ch = rng.u32();
    let fc = rng.u64() as i64;
    simple_file(rng, bits, ch, fc)
}

/// Standard `desc`, `pakt`, `data` file. Returns the built file.
fn simple_file(rng: &mut Rng, sample_bits: u64, channels: u32, frames: i64) -> File {
    let chunks = vec![
        desc_chunk(rng, sample_bits, channels),
        pakt_chunk(rng, frames),
        data_chunk(rng, None, 3),
    ];
    with_tail(build_valid(rng.u16(), &chunks), 64, rng)
}

/// Run `assert_same` and additionally require the C to have *succeeded*, so a
/// row cannot silently pass by both implementations erroring out.
#[track_caller]
fn expect_ok(label: &str, f: &File, off: usize) -> Outcome {
    let o = assert_same(label, &f.bytes, off);
    assert_eq!(o.ret, 0, "{label}: expected the C to accept this file, got {o:?}");
    o
}

/// Interesting `sample_rate` bit patterns (axis I of CONFIGS.md).
fn sample_rate_corpus() -> Vec<(&'static str, u64)> {
    let f = |x: f64| x.to_bits();
    let mut v = vec![
        ("0.0", f(0.0)),
        ("-0.0", f(-0.0)),
        ("1.0", f(1.0)),
        ("-1.0", f(-1.0)),
        ("0.5", f(0.5)),
        ("-0.5", f(-0.5)),
        ("1.5", f(1.5)),
        ("8000", f(8000.0)),
        ("11025", f(11025.0)),
        ("22050", f(22050.0)),
        ("32000", f(32000.0)),
        ("44100", f(44100.0)),
        ("44100.7", f(44100.7)),
        ("48000", f(48000.0)),
        ("88200", f(88200.0)),
        ("96000", f(96000.0)),
        ("192000", f(192000.0)),
        ("-44100", f(-44100.0)),
        ("1e-300", f(1e-300)),
        ("-1e-300", f(-1e-300)),
        ("2^62", f(4611686018427387904.0)),
        ("2^63-1024", f(9223372036854774784.0)),
        ("2^63", f(9223372036854775808.0)),
        ("2^63+2^11", f(9223372036854777856.0)),
        ("2^64", f(18446744073709551616.0)),
        ("2^64+2^12", f(18446744073709555712.0)),
        ("1e300", f(1e300)),
        ("-9e18", f(-9e18)),
        ("-2^63", f(-9223372036854775808.0)),
        ("-2^63-2^11", f(-9223372036854777856.0)),
        ("-1e300", f(-1e300)),
        ("+Inf", f(f64::INFINITY)),
        ("-Inf", f(f64::NEG_INFINITY)),
        ("qNaN", f(f64::NAN)),
        ("-qNaN", f(-f64::NAN)),
        ("sNaN", 0x7FF0_0000_0000_0001),
        ("NaN payload", 0x7FF8_DEAD_BEEF_1234),
        ("MIN_POSITIVE", f(f64::MIN_POSITIVE)),
        ("subnormal/2", f(f64::MIN_POSITIVE / 2.0)),
        ("smallest subnormal", 1u64),
        ("-smallest subnormal", 0x8000_0000_0000_0001),
        ("f64::MAX", f(f64::MAX)),
        ("f64::MIN", f(f64::MIN)),
        ("f64::EPSILON", f(f64::EPSILON)),
        ("all ones", u64::MAX),
        ("all zeros", 0u64),
    ];
    // a handful of exact powers of two around the conversion boundary
    for e in 50..70 {
        v.push(("2^e", (2f64).powi(e).to_bits()));
    }
    v
}

// ---------------------------------------------------------------------------
// C01 — minimal valid file
// ---------------------------------------------------------------------------

#[test]
fn c01_minimal_valid_file() {
    let mut rng = Rng::new(SEED ^ 0x01);
    for i in 0..N {
        let ch = rng.u32() % 9;
        let fc = rng.u64() as i64;
        let f = simple_file(&mut rng, (44100.0f64 + i as f64).to_bits(), ch, fc);
        let o = expect_ok("C01", &f, 0);
        // Independent expectation check: blocks == data-chunk payload + 4.
        let data_off = *f.chunk_offsets.last().unwrap();
        let buf = Buf::new(&f.bytes, 0);
        let (c, _) = call_both(&buf);
        assert_eq!(
            c.blocks - buf.base(),
            data_off + CHUNK_HDR + 4,
            "C01: blocks offset wrong (o={o:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// C02 — pakt before desc
// ---------------------------------------------------------------------------

#[test]
fn c02_pakt_before_desc() {
    let mut rng = Rng::new(SEED ^ 0x02);
    for _ in 0..N {
        let chunks = vec![
            pakt_rand(&mut rng),
            desc_rand(&mut rng),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C02", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C03 — zero unknown chunks (desc, pakt, data back to back)
// ---------------------------------------------------------------------------

#[test]
fn c03_no_unknown_chunks() {
    let mut rng = Rng::new(SEED ^ 0x03);
    for _ in 0..N {
        let f = simple_rand(&mut rng);
        expect_ok("C03", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C04 — exactly one unknown chunk between desc and pakt
// ---------------------------------------------------------------------------

#[test]
fn c04_one_unknown_chunk() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..N {
        let len = rng.below(65);
        let chunks = vec![
            desc_rand(&mut rng),
            unknown_chunk(&mut rng, len),
            pakt_rand(&mut rng),
            data_chunk(&mut rng, None, 1),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C04", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C05 — many unknown chunks interleaved at random positions
// ---------------------------------------------------------------------------

#[test]
fn c05_many_unknown_chunks() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..N {
        let nunk = 2 + rng.below(7);
        let mut chunks: Vec<Chunk> = Vec::new();
        let desc_first = rng.bool();
        let (a, b) = if desc_first {
            (
                desc_rand(&mut rng),
                pakt_rand(&mut rng),
            )
        } else {
            (
                pakt_rand(&mut rng),
                desc_rand(&mut rng),
            )
        };
        chunks.push(a);
        for _ in 0..nunk / 2 {
            let l = rng.below(65);
            chunks.push(unknown_chunk(&mut rng, l));
        }
        chunks.push(b);
        for _ in 0..nunk - nunk / 2 {
            let l = rng.below(65);
            chunks.push(unknown_chunk(&mut rng, l));
        }
        chunks.push(data_rand(&mut rng, None));
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C05", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C06 — unknown chunk with size == 0
// ---------------------------------------------------------------------------

#[test]
fn c06_unknown_chunk_size_zero() {
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..N {
        let chunks = vec![
            unknown_chunk(&mut rng, 0),
            desc_rand(&mut rng),
            unknown_chunk(&mut rng, 0),
            pakt_rand(&mut rng),
            unknown_chunk(&mut rng, 0),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C06", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C07 — unknown chunk with a large positive size
// ---------------------------------------------------------------------------

#[test]
fn c07_unknown_chunk_large_size() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..64 {
        let len = 512 + rng.below(3585); // 512..4096
        let chunks = vec![
            desc_rand(&mut rng),
            unknown_chunk(&mut rng, len),
            pakt_rand(&mut rng),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C07", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C08 — negative chunk size: the walk moves *backwards*
//
// Physical order:  desc | pakt | J | data | K
//   J declares size = off(K) - off(J) - 16   (positive jump over `data`)
//   K declares size = off(data) - off(K) - 16 (NEGATIVE jump back to `data`)
// ---------------------------------------------------------------------------

#[test]
fn c08_negative_chunk_size_walks_backwards() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..N {
        let desc = desc_rand(&mut rng);
        let pakt = pakt_rand(&mut rng);
        let nb = 1 + rng.below(3);
        let data = data_chunk(&mut rng, None, nb);
        let j_cc = unknown_fourcc(&mut rng);
        let k_cc = unknown_fourcc(&mut rng);

        let off_desc = FILE_HDR;
        let off_pakt = off_desc + desc.total();
        let off_j = off_pakt + pakt.total();
        let off_data = off_j + CHUNK_HDR;
        let off_k = off_data + data.total();

        let j = Chunk {
            fourcc: j_cc,
            pad: rng.u32().to_be_bytes(),
            size: (off_k as i64) - (off_j as i64) - CHUNK_HDR as i64,
            payload: Vec::new(),
        };
        let k = Chunk {
            fourcc: k_cc,
            pad: rng.u32().to_be_bytes(),
            size: (off_data as i64) - (off_k as i64) - CHUNK_HDR as i64,
            payload: Vec::new(),
        };
        assert!(k.size < 0, "K must jump backwards");

        let chunks = vec![desc, pakt, j, data, k];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        // sanity: physical offsets are what we predicted
        assert_eq!(f.chunk_offsets, vec![off_desc, off_pakt, off_j, off_data, off_k]);
        let o = expect_ok("C08", &f, 0);
        let buf = Buf::new(&f.bytes, 0);
        let (c, _) = call_both(&buf);
        assert_eq!(
            c.blocks - buf.base(),
            off_data + CHUNK_HDR + 4,
            "C08: backwards walk must land on the data chunk ({o:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// C09 / C10 — duplicate desc / pakt chunks: last one wins
// ---------------------------------------------------------------------------

#[test]
fn c09_duplicate_desc_last_wins() {
    let mut rng = Rng::new(SEED ^ 0x09);
    for _ in 0..N {
        let first = desc_rand(&mut rng);
        let second_bits = rng.u64();
        let second_ch = rng.u32();
        let second = desc_chunk(&mut rng, second_bits, second_ch);
        let chunks = vec![
            first,
            pakt_rand(&mut rng),
            second,
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C09", &f, 0);
        assert_eq!(
            o.channel_count, second_ch,
            "C09: the LAST desc chunk must win"
        );
    }
}

#[test]
fn c10_duplicate_pakt_last_wins() {
    let mut rng = Rng::new(SEED ^ 0x0A);
    for _ in 0..N {
        let f1 = rng.u64() as i64;
        let f2 = rng.u64() as i64;
        let chunks = vec![
            desc_rand(&mut rng),
            pakt_chunk(&mut rng, f1),
            pakt_chunk(&mut rng, f2),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C10", &f, 0);
        assert_eq!(
            o.frame_count, f2 as u64,
            "C10: the LAST pakt chunk must win"
        );
    }
}

// ---------------------------------------------------------------------------
// C11 — a second data chunk is never reached (the first one breaks the loop)
// ---------------------------------------------------------------------------

#[test]
fn c11_second_data_chunk_ignored() {
    let mut rng = Rng::new(SEED ^ 0x0B);
    for _ in 0..N {
        let desc = desc_rand(&mut rng);
        let pakt = pakt_rand(&mut rng);
        let d1 = data_chunk(&mut rng, None, 2);
        let d2 = data_chunk(&mut rng, None, 5);
        let off_d1 = FILE_HDR + desc.total() + pakt.total();
        let chunks = vec![desc, pakt, d1, d2];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C11", &f, 0);
        let buf = Buf::new(&f.bytes, 0);
        let (c, _) = call_both(&buf);
        assert_eq!(c.blocks - buf.base(), off_d1 + CHUNK_HDR + 4, "C11: {o:?}");
    }
}

// ---------------------------------------------------------------------------
// C12 / C13 / C14 — the data chunk's declared size becomes info->size
// ---------------------------------------------------------------------------

#[test]
fn c12_data_size_zero() {
    let mut rng = Rng::new(SEED ^ 0x0C);
    for _ in 0..N {
        let chunks = vec![
            desc_rand(&mut rng),
            pakt_rand(&mut rng),
            data_chunk(&mut rng, Some(0), 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C12", &f, 0);
        assert_eq!(o.size, 0);
    }
}

#[test]
fn c13_data_size_random_positive() {
    let mut rng = Rng::new(SEED ^ 0x0D);
    for _ in 0..N {
        let s = (rng.u32() >> 1) as i64;
        let chunks = vec![
            desc_rand(&mut rng),
            pakt_rand(&mut rng),
            data_chunk(&mut rng, Some(s), 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C13", &f, 0);
        assert_eq!(o.size, s as u64);
    }
}

#[test]
fn c14_data_size_extremes() {
    let mut rng = Rng::new(SEED ^ 0x0E);
    let extremes: [i64; 8] = [
        -1,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        -0x1_0000_0000,
        0x7FFF_FFFF,
        -0x7FFF_FFFF,
    ];
    for &s in &extremes {
        for _ in 0..32 {
            let chunks = vec![
                desc_rand(&mut rng),
                pakt_rand(&mut rng),
                data_chunk(&mut rng, Some(s), 2),
            ];
            let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
            let o = expect_ok("C14", &f, 0);
            assert_eq!(o.size, s as u64, "C14 size={s}");
        }
    }
}

// ---------------------------------------------------------------------------
// C15..C24 — the sample_rate axis. The C does an *arithmetic* double -> u64
// conversion, byte-swaps, then reinterprets the bits as a double, so every one
// of these is a distinct code path (in-range / >= 2^63 / NaN / out-of-range).
// ---------------------------------------------------------------------------

fn run_sample_rate_case(label: &str, bits: u64, rng: &mut Rng) {
    let chunks = vec![
        desc_bits(rng, bits),
        pakt_rand(rng),
        data_chunk(rng, None, 2),
    ];
    let f = with_tail(build_valid(rng.u16(), &chunks), 64, rng);
    for off in [0usize, 1, 4] {
        let o = assert_same(label, &f.bytes, off);
        assert_eq!(o.ret, 0, "{label}: bits={bits:#018x}");
    }
}

#[test]
fn c15_c23_sample_rate_corpus() {
    let mut rng = Rng::new(SEED ^ 0x0F);
    for (name, bits) in sample_rate_corpus() {
        run_sample_rate_case(name, bits, &mut rng);
    }
}

#[test]
fn c15_canonical_audio_rates() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for r in [
        8000.0f64, 11025.0, 16000.0, 22050.0, 24000.0, 32000.0, 44100.0, 48000.0, 64000.0,
        88200.0, 96000.0, 176400.0, 192000.0, 352800.0, 384000.0,
    ] {
        run_sample_rate_case("C15", r.to_bits(), &mut rng);
    }
}

#[test]
fn c16_sample_rate_signed_zero() {
    let mut rng = Rng::new(SEED ^ 0x11);
    run_sample_rate_case("C16 +0.0", 0.0f64.to_bits(), &mut rng);
    run_sample_rate_case("C16 -0.0", (-0.0f64).to_bits(), &mut rng);
}

#[test]
fn c17_sample_rate_fractional() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for v in [
        0.5f64, -0.5, 1.5, -1.5, 0.9999999999, -0.9999999999, 44100.7, -44100.7, 1e-300,
        -1e-300, 2.220446049250313e-16,
    ] {
        run_sample_rate_case("C17", v.to_bits(), &mut rng);
    }
}

#[test]
fn c18_sample_rate_negative_integral() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for v in [-1.0f64, -2.0, -44100.0, -1e9, -9e18, -4.6e18] {
        run_sample_rate_case("C18", v.to_bits(), &mut rng);
    }
}

#[test]
fn c19_sample_rate_at_and_above_2_63() {
    let mut rng = Rng::new(SEED ^ 0x14);
    let two63 = 9223372036854775808.0f64;
    for v in [
        two63 - 1024.0,
        two63,
        two63 + 2048.0,
        two63 * 1.5,
        two63 * 2.0,          // 2^64
        two63 * 2.0 + 4096.0, // just past 2^64
        two63 * 4.0,
        1e300,
        f64::MAX,
    ] {
        run_sample_rate_case("C19", v.to_bits(), &mut rng);
    }
}

#[test]
fn c20_sample_rate_below_negative_2_63() {
    let mut rng = Rng::new(SEED ^ 0x15);
    let two63 = 9223372036854775808.0f64;
    for v in [-two63, -two63 - 2048.0, -two63 * 2.0, -1e300, f64::MIN] {
        run_sample_rate_case("C20", v.to_bits(), &mut rng);
    }
}

#[test]
fn c21_sample_rate_infinities() {
    let mut rng = Rng::new(SEED ^ 0x16);
    run_sample_rate_case("C21 +Inf", f64::INFINITY.to_bits(), &mut rng);
    run_sample_rate_case("C21 -Inf", f64::NEG_INFINITY.to_bits(), &mut rng);
}

#[test]
fn c22_sample_rate_nans() {
    let mut rng = Rng::new(SEED ^ 0x17);
    let mut nans = vec![
        f64::NAN.to_bits(),
        (-f64::NAN).to_bits(),
        0x7FF8_0000_0000_0000u64, // canonical qNaN
        0xFFF8_0000_0000_0000,    // negative qNaN
        0x7FF0_0000_0000_0001,    // sNaN
        0xFFF0_0000_0000_0001,    // negative sNaN
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
    ];
    let mut r2 = Rng::new(SEED ^ 0x1717);
    for _ in 0..24 {
        let payload = r2.u64() & 0x000F_FFFF_FFFF_FFFF;
        if payload != 0 {
            nans.push(0x7FF0_0000_0000_0000 | payload);
            nans.push(0xFFF0_0000_0000_0000 | payload);
        }
    }
    for bits in nans {
        run_sample_rate_case("C22", bits, &mut rng);
    }
}

#[test]
fn c23_sample_rate_subnormals() {
    let mut rng = Rng::new(SEED ^ 0x18);
    let mut cases = vec![
        f64::MIN_POSITIVE.to_bits(),
        (f64::MIN_POSITIVE / 2.0).to_bits(),
        1u64,
        0x8000_0000_0000_0001,
        0x000F_FFFF_FFFF_FFFF,
        0x800F_FFFF_FFFF_FFFF,
    ];
    let mut r2 = Rng::new(SEED ^ 0x1818);
    for _ in 0..24 {
        let m = r2.u64() & 0x000F_FFFF_FFFF_FFFF;
        cases.push(m);
        cases.push(0x8000_0000_0000_0000 | m);
    }
    for bits in cases {
        run_sample_rate_case("C23", bits, &mut rng);
    }
}

#[test]
fn c24_sample_rate_fully_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x19);
    let mut bitgen = Rng::new(0xF00D_BEEF_1234_5678);
    for _ in 0..1024 {
        run_sample_rate_case("C24", bitgen.u64(), &mut rng);
    }
    // Sweep the whole exponent field with random mantissas: this is where the
    // double -> u64 conversion switches between its code paths.
    for exp in 0u64..2048 {
        let bits = (exp << 52) | (bitgen.u64() & 0x000F_FFFF_FFFF_FFFF);
        run_sample_rate_case("C24 exp", bits, &mut rng);
        run_sample_rate_case("C24 exp-neg", bits | 0x8000_0000_0000_0000, &mut rng);
    }
}

// ---------------------------------------------------------------------------
// C25 / C26 — channel_count and frame_count pass through unchecked
// ---------------------------------------------------------------------------

#[test]
fn c25_channel_count_axis() {
    let mut rng = Rng::new(SEED ^ 0x1A);
    let mut fixed: Vec<u32> = vec![0, 1, 2, 3, 4, 6, 8, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    let mut r2 = Rng::new(SEED ^ 0x1A1A);
    for _ in 0..64 {
        fixed.push(r2.u32());
    }
    for ch in fixed {
        let chunks = vec![
            desc_ch(&mut rng, ch),
            pakt_rand(&mut rng),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C25", &f, 0);
        assert_eq!(o.channel_count, ch, "C25 ch={ch:#x}");
    }
}

#[test]
fn c26_frame_count_axis() {
    let mut rng = Rng::new(SEED ^ 0x1B);
    let mut fixed: Vec<i64> = vec![0, 1, -1, i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1];
    let mut r2 = Rng::new(SEED ^ 0x1B1B);
    for _ in 0..64 {
        fixed.push(r2.u64() as i64);
    }
    for fc in fixed {
        let chunks = vec![
            desc_rand(&mut rng),
            pakt_chunk(&mut rng, fc),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C26", &f, 0);
        assert_eq!(o.frame_count, fc as u64, "C26 fc={fc}");
    }
}

// ---------------------------------------------------------------------------
// C27 / C28 — fields the C never reads must not change anything
// ---------------------------------------------------------------------------

#[test]
fn c27_header_flags_are_ignored() {
    let mut rng = Rng::new(SEED ^ 0x1C);
    let bits = 44100.0f64.to_bits();
    let mut reference: Option<(u64, u64, u64, u32)> = None;
    let mut flags: Vec<u16> = vec![0x0000, 0x0001, 0xFFFF, 0x8000, 0x00FF, 0xFF00];
    let mut r2 = Rng::new(SEED ^ 0x1C1C);
    for _ in 0..32 {
        flags.push(r2.u16());
    }
    for fl in flags {
        // deterministic bodies so the only varying input is `flags`
        let mut r = Rng::new(0xAAAA_BBBB_CCCC_DDDD);
        let chunks = vec![
            desc_chunk(&mut r, bits, 2),
            pakt_chunk(&mut r, 1234),
            data_chunk(&mut r, Some(100), 2),
        ];
        let f = with_tail(build(*b"caff", 1, fl, &chunks), 64, &mut rng);
        let o = expect_ok("C27", &f, 0);
        let key = (o.size, o.sample_bits, o.frame_count, o.channel_count);
        match reference {
            None => reference = Some(key),
            Some(r0) => assert_eq!(r0, key, "C27: flags={fl:#06x} changed the output"),
        }
    }
}

#[test]
fn c28_unused_fields_are_ignored() {
    let mut rng = Rng::new(SEED ^ 0x1D);
    let bits = 48000.0f64.to_bits();
    let mut reference: Option<(u64, u64, u64, u32)> = None;
    for mode in 0..34usize {
        let filler = |i: usize, m: usize| -> u32 {
            match m {
                0 => 0,
                1 => 0xFFFF_FFFF,
                _ => 0x9E37_79B9u32.wrapping_mul((i as u32).wrapping_add(m as u32)),
            }
        };
        let desc_body = DescBody {
            sample_rate_bits: bits,
            format_id: FOURCC_IMA4,
            format_flags: filler(0, mode),
            bytes_per_packet: filler(1, mode),
            frames_per_packet: filler(2, mode),
            channels_per_frame: 2,
            bits_per_channel: filler(3, mode),
        };
        let pakt_body = PaktBody {
            packet_count: (((filler(4, mode) as u64) << 32) | filler(5, mode) as u64) as i64,
            frame_count: 999,
            priming_frames: filler(6, mode) as i32,
            remainder_frames: filler(7, mode) as i32,
        };
        let chunks = vec![
            Chunk::exact(FOURCC_DESC, desc_body.bytes()).with_pad(filler(8, mode).to_be_bytes()),
            Chunk::exact(FOURCC_PAKT, pakt_body.bytes()).with_pad(filler(9, mode).to_be_bytes()),
            Chunk::exact(FOURCC_DATA, data_payload(filler(10, mode), &[0xAB; 68]))
                .with_pad(filler(11, mode).to_be_bytes()),
        ];
        let f = with_tail(build_valid(0x1234, &chunks), 64, &mut rng);
        let o = expect_ok("C28", &f, 0);
        let key = (o.size, o.sample_bits, o.frame_count, o.channel_count);
        match reference {
            None => reference = Some(key),
            Some(r0) => assert_eq!(r0, key, "C28: unused-field fill mode {mode} changed output"),
        }
    }
}

// ---------------------------------------------------------------------------
// C29 — unaligned buffer start
// ---------------------------------------------------------------------------

#[test]
fn c29_unaligned_buffers() {
    let mut rng = Rng::new(SEED ^ 0x1E);
    for off in 0..16usize {
        for _ in 0..32 {
            let f = simple_rand(&mut rng);
            let o = assert_same("C29", &f.bytes, off);
            assert_eq!(o.ret, 0, "C29 off={off}");
        }
    }
}

// ---------------------------------------------------------------------------
// C30 — the returned `blocks` pointer is identical (absolutely) in both
// ---------------------------------------------------------------------------

#[test]
fn c30_blocks_pointer_is_identical_across_addresses() {
    let mut rng = Rng::new(SEED ^ 0x1F);
    for _ in 0..N {
        let f = simple_rand(&mut rng);
        let data_off = *f.chunk_offsets.last().unwrap();
        for off in [0usize, 3, 7] {
            let buf = Buf::new(&f.bytes, off);
            let (c, r) = call_both(&buf);
            assert_eq!(c, r, "C30 divergence off={off}: C={c:?} RUST={r:?}");
            assert_eq!(c.ret, 0);
            assert_eq!(
                c.blocks,
                buf.base() + data_off + CHUNK_HDR + 4,
                "C30: absolute blocks pointer"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — chunk payload larger than the struct the C reads
// ---------------------------------------------------------------------------

#[test]
fn c31_oversized_chunk_payloads() {
    let mut rng = Rng::new(SEED ^ 0x20);
    for _ in 0..N {
        let extra_d = rng.below(200);
        let extra_p = rng.below(200);
        let mut dbody = DescBody::ima4(f64::from_bits(rng.u64()), rng.u32()).bytes();
        dbody.extend(rng.bytes(extra_d));
        let mut pbody = PaktBody::simple(rng.u64() as i64).bytes();
        pbody.extend(rng.bytes(extra_p));
        let chunks = vec![
            Chunk::exact(FOURCC_DESC, dbody),
            Chunk::exact(FOURCC_PAKT, pbody),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C31", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C32 — declared chunk size SMALLER than the struct the C reads.
//
// Hand-built overlap: the `desc` chunk declares `size = 8` (only the
// `sample_rate` field), so the walk continues at desc+24, which is exactly
// where the C also reads `desc->format_id`. We put a chunk header there whose
// FourCC bytes are `ima4` (an unknown chunk type, therefore skipped) so that
// `desc->format_id == 'ima4'`, and whose first payload word doubles as
// `desc->channels_per_frame`.
// ---------------------------------------------------------------------------

#[test]
fn c32_undersized_desc_chunk_reads_past_declared_size() {
    let mut rng = Rng::new(SEED ^ 0x21);
    for _ in 0..N {
        let sample_bits = rng.u64();
        let channels = rng.u32();
        let overlap_payload_len = 16usize;
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"caff");
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&rng.u16().to_be_bytes());
        // desc chunk header, declared size 8
        let off_desc = bytes.len();
        bytes.extend_from_slice(FOURCC_DESC.as_slice());
        bytes.extend_from_slice(&rng.u32().to_be_bytes());
        bytes.extend_from_slice(&8i64.to_be_bytes());
        // desc payload: sample_rate only
        bytes.extend_from_slice(&sample_bits.to_ne_bytes());
        // overlap chunk header at off_desc+24 == desc_body+8
        let off_overlap = bytes.len();
        assert_eq!(off_overlap, off_desc + 24);
        bytes.extend_from_slice(FOURCC_IMA4.as_slice()); // doubles as desc->format_id
        bytes.extend_from_slice(&rng.u32().to_be_bytes()); // desc->format_flags
        bytes.extend_from_slice(&(overlap_payload_len as i64).to_be_bytes());
        // overlap payload; payload[0..4] doubles as desc->channels_per_frame
        bytes.extend_from_slice(&channels.to_be_bytes());
        bytes.extend(rng.bytes(overlap_payload_len - 4));
        // pakt chunk
        let frames = rng.u64() as i64;
        let pbody = PaktBody::simple(frames).bytes();
        bytes.extend_from_slice(FOURCC_PAKT.as_slice());
        bytes.extend_from_slice(&rng.u32().to_be_bytes());
        bytes.extend_from_slice(&(pbody.len() as i64).to_be_bytes());
        bytes.extend_from_slice(&pbody);
        // data chunk
        let dpayload = data_payload(rng.u32(), &rng.bytes(68));
        let off_data = bytes.len();
        bytes.extend_from_slice(FOURCC_DATA.as_slice());
        bytes.extend_from_slice(&rng.u32().to_be_bytes());
        bytes.extend_from_slice(&(dpayload.len() as i64).to_be_bytes());
        bytes.extend_from_slice(&dpayload);
        bytes.extend(rng.bytes(64));

        let buf = Buf::new(&bytes, 0);
        let (c, r) = call_both(&buf);
        assert_eq!(c, r, "C32 divergence: C={c:?} RUST={r:?}{}", hex(&bytes));
        assert_eq!(c.ret, 0, "C32: expected success, got {c:?}");
        assert_eq!(c.channel_count, channels, "C32: overlapped channel_count");
        assert_eq!(c.frame_count, frames as u64, "C32: frame_count");
        assert_eq!(c.blocks, buf.base() + off_data + CHUNK_HDR + 4);
    }
}

// ---------------------------------------------------------------------------
// C33 — full cross-product smoke: every axis randomized simultaneously
// ---------------------------------------------------------------------------

#[test]
fn c33_full_cross_product_smoke() {
    let mut rng = Rng::new(SEED ^ 0x22);
    let corpus = sample_rate_corpus();
    for i in 0..4096usize {
        // sample_rate: half from the corpus, half fully random
        let bits = if rng.bool() {
            corpus[rng.below(corpus.len())].1
        } else {
            rng.u64()
        };
        let channels = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => rng.u32(),
        };
        let frames = match rng.below(4) {
            0 => 0,
            1 => -1,
            2 => i64::MIN,
            _ => rng.u64() as i64,
        };
        let data_size = match rng.below(5) {
            0 => None,
            1 => Some(0),
            2 => Some(-1),
            3 => Some(i64::MAX),
            _ => Some(rng.u64() as i64),
        };

        let mut chunks: Vec<Chunk> = Vec::new();
        let n_pre = rng.below(4);
        for _ in 0..n_pre {
            let l = rng.below(48);
            chunks.push(unknown_chunk(&mut rng, l));
        }
        let dup_desc = rng.below(8) == 0;
        let dup_pakt = rng.below(8) == 0;
        if rng.bool() {
            if dup_desc {
                chunks.push(desc_rand(&mut rng));
            }
            chunks.push(desc_chunk(&mut rng, bits, channels));
            if dup_pakt {
                chunks.push(pakt_rand(&mut rng));
            }
            chunks.push(pakt_chunk(&mut rng, frames));
        } else {
            if dup_pakt {
                chunks.push(pakt_rand(&mut rng));
            }
            chunks.push(pakt_chunk(&mut rng, frames));
            if dup_desc {
                chunks.push(desc_rand(&mut rng));
            }
            chunks.push(desc_chunk(&mut rng, bits, channels));
        }
        let n_mid = rng.below(4);
        for _ in 0..n_mid {
            let l = rng.below(48);
            chunks.push(unknown_chunk(&mut rng, l));
        }
        chunks.push(data_rand(&mut rng, data_size));

        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let off = rng.below(8);
        let o = assert_same("C33", &f.bytes, off);
        assert_eq!(o.ret, 0, "C33 iter={i}");
        assert_eq!(o.channel_count, channels, "C33 iter={i} channels");
        assert_eq!(o.frame_count, frames as u64, "C33 iter={i} frames");
    }
}

// ---------------------------------------------------------------------------
// C34 / C35 — chunk-type boundary: near-miss FourCCs are "unknown" (skipped)
// ---------------------------------------------------------------------------

#[test]
fn c34_perturbed_known_fourccs_are_skipped() {
    let mut rng = Rng::new(SEED ^ 0x23);
    for base in [FOURCC_DESC, FOURCC_PAKT, FOURCC_DATA] {
        for byte in 0..4usize {
            for delta in [1u8, 255, 0x20, 0x80] {
                let mut cc = base;
                cc[byte] = cc[byte].wrapping_add(delta);
                if cc == FOURCC_DESC || cc == FOURCC_PAKT || cc == FOURCC_DATA {
                    continue;
                }
                let odd = Chunk::exact(cc, rng.bytes(24)).with_pad(rng.u32().to_be_bytes());
                let chunks = vec![
                    odd,
                    desc_rand(&mut rng),
                    pakt_rand(&mut rng),
                    data_chunk(&mut rng, None, 2),
                ];
                let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
                let o = expect_ok("C34", &f, 0);
                let buf = Buf::new(&f.bytes, 0);
                let (c, _) = call_both(&buf);
                assert_eq!(
                    c.blocks - buf.base(),
                    *f.chunk_offsets.last().unwrap() + CHUNK_HDR + 4,
                    "C34 cc={cc:?} {o:?}"
                );
            }
        }
    }
}

#[test]
fn c35_out_of_range_chunk_type_values() {
    // `chunk->type` is a plain u32 across the FFI boundary: every value that is
    // not one of the three known FourCCs is a legal input meaning "skip".
    let mut rng = Rng::new(SEED ^ 0x24);
    let mut probes: Vec<u32> = vec![0x0000_0000, 0xFFFF_FFFF, 0x8000_0000, 0x7FFF_FFFF, 1, 2];
    for base in [FOURCC_DESC, FOURCC_PAKT, FOURCC_DATA, FOURCC_IMA4] {
        let v = u32::from_be_bytes(base);
        probes.extend([
            v.wrapping_sub(1),
            v.wrapping_add(1),
            v ^ 0x8000_0000,
            v ^ 0x0000_0080,
            v.swap_bytes(),
            !v,
        ]);
    }
    let mut r2 = Rng::new(SEED ^ 0x2424);
    for _ in 0..128 {
        probes.push(r2.u32());
    }
    for v in probes {
        let cc = v.to_be_bytes();
        if cc == FOURCC_DESC || cc == FOURCC_PAKT || cc == FOURCC_DATA {
            continue;
        }
        let l = rng.below(40);
        let odd = Chunk::exact(cc, rng.bytes(l)).with_pad(rng.u32().to_be_bytes());
        let chunks = vec![
            desc_rand(&mut rng),
            odd,
            pakt_rand(&mut rng),
            data_chunk(&mut rng, None, 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        expect_ok("C35", &f, 0);
    }
}

// ---------------------------------------------------------------------------
// C36 — the `static` bswap/btoh helpers, exercised indirectly but exhaustively
// through every big-endian field the parser reads.
// ---------------------------------------------------------------------------

#[test]
fn c36_byteswap_helpers_via_all_be_fields() {
    let mut rng = Rng::new(SEED ^ 0x25);
    // Walk single-bit patterns through every big-endian field so each byte lane
    // of bswap16/32/64 is proved independently.
    for bit in 0..64u32 {
        let v64 = 1u64 << bit;
        let v32 = if bit < 32 { 1u32 << bit } else { 0 };
        let chunks = vec![
            desc_ch(&mut rng, v32),
            pakt_chunk(&mut rng, v64 as i64),
            data_chunk(&mut rng, Some(v64 as i64), 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C36 bit", &f, 0);
        assert_eq!(o.frame_count, v64);
        assert_eq!(o.size, v64);
        assert_eq!(o.channel_count, v32);
        // and the complement
        let chunks = vec![
            desc_ch(&mut rng, !v32),
            pakt_chunk(&mut rng, !v64 as i64),
            data_chunk(&mut rng, Some(!v64 as i64), 2),
        ];
        let f = with_tail(build_valid(rng.u16(), &chunks), 64, &mut rng);
        let o = expect_ok("C36 !bit", &f, 0);
        assert_eq!(o.frame_count, !v64);
        assert_eq!(o.size, !v64);
        assert_eq!(o.channel_count, !v32);
    }
    // bswap16 is only used for `header->version`; every 16-bit value is probed
    // in tests/errors.rs (`e2e_all_versions_rejected`).
}

// ---------------------------------------------------------------------------
// C37 — `info` ALIASES the input buffer.
//
// `ima_parse` writes `info->blocks` and `info->size` *before* reading
// `pakt->frame_count`, `desc->channels_per_frame` and `desc->sample_rate`, so if
// the caller points `info` at the data buffer the interleaving of those writes
// and reads becomes observable. Both the returned `ima_info` *and* the final
// buffer contents must match byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn c37_info_aliasing_the_input_buffer() {
    let mut rng = Rng::new(SEED ^ 0x26);
    for _ in 0..96 {
        let f = simple_rand(&mut rng);
        let len = f.bytes.len();
        // sweep the whole buffer, plus a few unaligned info pointers
        let mut offs: Vec<usize> = (0..len.saturating_sub(40)).step_by(4).collect();
        offs.extend([0usize, 1, 2, 3, 5, 7, 9, 13]);
        for io in offs {
            if io + 40 > len + 64 {
                continue;
            }
            let ((co, cb), (ro, rb)) = call_both_aliased(&f.bytes, 0, io);
            assert_eq!(
                co, ro,
                "C37 divergence (info_off={io}): C={co:?} RUST={ro:?}"
            );
            assert_eq!(
                cb, rb,
                "C37 buffer divergence (info_off={io}):\n C  ={}\n RUST={}",
                hex(&cb),
                hex(&rb)
            );
        }
    }
}

#[test]
fn c37b_info_aliasing_with_error_paths() {
    // On the error paths nothing is written, so an aliased `info` must leave the
    // buffer completely untouched in both implementations.
    let mut rng = Rng::new(SEED ^ 0x27);
    for magic in [*b"junk", *b"caff"] {
        for ver in [0u16, 1, 7] {
            let d = desc_rand(&mut rng);
            let p = pakt_rand(&mut rng);
            let da = data_rand(&mut rng, None);
            let flags = rng.u16();
            let f = with_tail(build(magic, ver, flags, &[d, p, da]), 64, &mut rng);
            for io in [0usize, 8, 16, 24, 40] {
                let ((co, cb), (ro, rb)) = call_both_aliased(&f.bytes, 0, io);
                assert_eq!(co, ro, "C37b divergence magic={magic:?} ver={ver} io={io}");
                assert_eq!(cb, rb, "C37b buffer divergence");
                if &magic != b"caff" {
                    assert_eq!(co.ret, -1);
                    assert_eq!(cb, f.bytes, "C37b: -1 path must not write");
                } else if ver != 1 {
                    assert_eq!(co.ret, -2);
                    assert_eq!(cb, f.bytes, "C37b: -2 path must not write");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C24b — high-volume sweep of the `double -> u64 -> bswap64 -> double` pipeline.
//
// This is the only genuinely non-obvious computation in the library (an
// *arithmetic* double->unsigned-long-long conversion whose out-of-range and NaN
// results are C-UB but have a definite x86-64 codegen), so it gets a dedicated
// 300k-value sweep. The file is built once and only the 8 `sample_rate` bytes
// are patched per iteration.
// ---------------------------------------------------------------------------

#[test]
fn c24b_sample_rate_pipeline_high_volume_sweep() {
    let mut rng = Rng::new(SEED ^ 0x28);
    // desc is the FIRST chunk, so desc body starts at FILE_HDR + CHUNK_HDR and
    // `sample_rate` is at its offset 0.
    let chunks = vec![
        desc_bits(&mut rng, 0),
        pakt_rand(&mut rng),
        data_rand(&mut rng, None),
    ];
    let f = with_tail(build_valid(0x1234, &chunks), 64, &mut rng);
    let sr_off = FILE_HDR + CHUNK_HDR;
    assert_eq!(f.chunk_offsets[0], FILE_HDR);

    let mut buf = Buf::new(&f.bytes, 0);
    let mut gen = Rng::new(0x5EED_5EED_5EED_5EED);

    let mut check = |buf: &mut Buf, bits: u64, tag: &str| {
        buf.write_at(sr_off, &bits.to_ne_bytes());
        let (c, r) = call_both(buf);
        assert_eq!(
            c, r,
            "C24b divergence ({tag}) sample_rate bits={bits:#018x} ({:?}): C={c:?} RUST={r:?}",
            f64::from_bits(bits)
        );
        assert_eq!(c.ret, 0, "C24b {tag}");
    };

    // 1. uniform random 64-bit patterns
    for _ in 0..150_000 {
        let b = gen.u64();
        check(&mut buf, b, "uniform");
    }
    // 2. exponent-focused: every exponent, random mantissa, both signs
    for _ in 0..40 {
        for exp in 0u64..2048 {
            let m = gen.u64() & 0x000F_FFFF_FFFF_FFFF;
            check(&mut buf, (exp << 52) | m, "exp");
            check(&mut buf, 0x8000_0000_0000_0000 | (exp << 52) | m, "exp-neg");
        }
    }
    // 3. dense sweep right around the 2^63 conversion boundary, where the C
    //    switches between `cvttsd2si` and `subsd`+`cvttsd2si`+`xor`.
    let two63 = 9223372036854775808.0f64.to_bits();
    for d in 0..4096u64 {
        check(&mut buf, two63.wrapping_sub(d), "below 2^63");
        check(&mut buf, two63.wrapping_add(d), "above 2^63");
        check(&mut buf, 0x8000_0000_0000_0000 | two63.wrapping_sub(d), "below -2^63");
        check(&mut buf, 0x8000_0000_0000_0000 | two63.wrapping_add(d), "above -2^63");
    }
    // 4. dense sweep around 2^64 and around 1.0 / 0.0
    let two64 = 18446744073709551616.0f64.to_bits();
    let one = 1.0f64.to_bits();
    for d in 0..4096u64 {
        check(&mut buf, two64.wrapping_sub(d), "below 2^64");
        check(&mut buf, two64.wrapping_add(d), "above 2^64");
        check(&mut buf, one.wrapping_sub(d), "below 1.0");
        check(&mut buf, one.wrapping_add(d), "above 1.0");
        check(&mut buf, d, "near +0");
        check(&mut buf, 0x8000_0000_0000_0000 | d, "near -0");
    }
}
