//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E18). Rows E14..E17 are the unchecked
//! NULL-dereference paths: the C faults there, so they are compared by running
//! the identical call in a child process for each library and asserting both
//! children die with the SAME signal.

mod common;

use common::*;
use std::ffi::c_void;

fn pair() -> Pair {
    Pair::load()
}

/// A canonical, fully valid file (returns 0) that individual rows then break.
fn valid_builder(rng: &mut Rng) -> CafBuilder {
    let d = rng.desc();
    let k = rng.pakt();
    let mut b = CafBuilder::new();
    b.filler = rng.u8();
    b.push(Chunk::desc(&d));
    b.push(Chunk::pakt(&k));
    b.push(Chunk::data(rng.u32(), 34));
    b
}

/// Assert both libraries return exactly `want` and leave `*info` untouched.
fn assert_rc_and_untouched(p: &Pair, buf: &AlignedBuf, want: i32, ctx: &str) {
    for fill in [0xAAu8, 0x00, 0x5A, 0xFF] {
        let mut ci = ImaInfo::prefilled(fill);
        let mut ri = ImaInfo::prefilled(fill);
        let untouched = ImaInfo::prefilled(fill).raw();
        let ptr = buf.ptr() as *const c_void;
        let rc_c = unsafe { (p.c)(&mut ci, ptr) };
        let rc_r = unsafe { (p.r)(&mut ri, ptr) };
        assert_eq!(rc_c, want, "C return code ({ctx}) buf={}", buf.hex());
        assert_eq!(rc_r, want, "Rust return code ({ctx}) buf={}", buf.hex());
        assert_eq!(ci.raw(), untouched, "C wrote to *info on error path ({ctx})");
        assert_eq!(ri.raw(), untouched, "Rust wrote to *info on error path ({ctx})");
        assert_eq!(ci.raw(), ri.raw(), "info mismatch ({ctx})");
    }
}

// ---------------------------------------------------------------------------
// E1 — bad magic -> -1
// ---------------------------------------------------------------------------

#[test]
fn err_e1_bad_magic() {
    let p = pair();
    let mut rng = Rng::new(0xE0001);
    for i in 0..ITERS {
        let mut b = valid_builder(&mut rng);
        loop {
            b.magic = rng.u32().to_be_bytes();
            if &b.magic != b"caff" {
                break;
            }
        }
        assert_rc_and_untouched(&p, &b.build(), -1, &format!("E1 i={i} magic={:?}", b.magic));
    }
}

// ---------------------------------------------------------------------------
// E2 — bad version -> -2
// ---------------------------------------------------------------------------

#[test]
fn err_e2_bad_version() {
    let p = pair();
    let mut rng = Rng::new(0xE0002);
    for i in 0..ITERS {
        let mut b = valid_builder(&mut rng);
        loop {
            b.version = rng.u16();
            if b.version != 1 {
                break;
            }
        }
        assert_rc_and_untouched(&p, &b.build(), -2, &format!("E2 i={i} version={:#x}", b.version));
    }
}

// ---------------------------------------------------------------------------
// E3 — bad desc->format_id -> -3
// ---------------------------------------------------------------------------

#[test]
fn err_e3_bad_format_id() {
    let p = pair();
    let mut rng = Rng::new(0xE0003);
    for i in 0..ITERS {
        let mut d = rng.desc();
        loop {
            d.format_id = rng.u32().to_be_bytes();
            if &d.format_id != b"ima4" {
                break;
            }
        }
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34));
        assert_rc_and_untouched(&p, &b.build(), -3, &format!("E3 i={i} fmt={:?}", d.format_id));
    }
}

/// E1 is checked before E2, and both before the chunk loop.
#[test]
fn err_e1_precedes_e2_and_the_chunk_loop() {
    let p = pair();
    let mut rng = Rng::new(0xE0004);

    // wrong in both ways -> -1, not -2
    let mut b = valid_builder(&mut rng);
    b.magic = *b"xxxx";
    b.version = 0x1234;
    assert_rc_and_untouched(&p, &b.build(), -1, "E1<E2 precedence");

    // bad magic with a chunk list that would otherwise loop forever
    let mut b = CafBuilder::new();
    b.magic = *b"caFf";
    b.push(Chunk::new(b"junk", vec![0; 8]).with_declared_size(0));
    assert_rc_and_untouched(&p, &b.build(), -1, "E1 before loop");

    // bad version with a chunk list that would otherwise loop forever
    let mut b = CafBuilder::new();
    b.version = 2;
    b.push(Chunk::new(b"junk", vec![0; 8]).with_declared_size(0));
    assert_rc_and_untouched(&p, &b.build(), -2, "E2 before loop");
}

// ---------------------------------------------------------------------------
// E4 — magic one step off
// ---------------------------------------------------------------------------

#[test]
fn err_e4_magic_one_step_off() {
    let p = pair();
    let mut rng = Rng::new(0xE0005);
    let mut cands: Vec<[u8; 4]> = vec![
        *b"ffac", // little-endian spelling
        *b"cafg",
        *b"caef",
        *b"cbff",
        *b"baff",
        *b"caf\0",
        *b"CAFF",
        b"caff".map(|c| c ^ 1),
        [0, 0, 0, 0],
        [0xFF, 0xFF, 0xFF, 0xFF],
    ];
    // every single-bit flip of "caff"
    let base = u32::from_be_bytes(*b"caff");
    for bit in 0..32 {
        cands.push((base ^ (1u32 << bit)).to_be_bytes());
    }
    for (i, m) in cands.iter().enumerate() {
        assert!(m != b"caff");
        let mut b = valid_builder(&mut rng);
        b.magic = *m;
        assert_rc_and_untouched(&p, &b.build(), -1, &format!("E4 i={i} magic={m:?}"));
    }
    // and the exact valid value still succeeds
    let mut b = valid_builder(&mut rng);
    b.magic = *b"caff";
    let buf = b.build();
    p.diff(&buf, &"E4 control (valid magic)");
}

// ---------------------------------------------------------------------------
// E5 — version one step off
// ---------------------------------------------------------------------------

#[test]
fn err_e5_version_one_step_off() {
    let p = pair();
    let mut rng = Rng::new(0xE0006);
    for (i, v) in [0u16, 2, 3, 0x0100, 0x8000, 0x7FFF, 0xFFFF, 0x0101, 0x00FF]
        .iter()
        .enumerate()
    {
        let mut b = valid_builder(&mut rng);
        b.version = *v;
        assert_rc_and_untouched(&p, &b.build(), -2, &format!("E5 i={i} version={v:#06x}"));
    }
    // version == 1 with every possible `flags` value still succeeds: `flags`
    // is never read, so it must not leak into the version comparison.
    for f in [0u16, 1, 0xFFFF, 0x8000, 0x0100] {
        let mut b = valid_builder(&mut rng);
        b.version = 1;
        b.flags = f;
        p.diff(&b.build(), &format!("E5 control version=1 flags={f:#06x}"));
    }
}

// ---------------------------------------------------------------------------
// E6 — format_id one step off
// ---------------------------------------------------------------------------

#[test]
fn err_e6_format_one_step_off() {
    let p = pair();
    let mut rng = Rng::new(0xE0007);
    let mut cands: Vec<[u8; 4]> = vec![
        *b"4ami", // little-endian spelling
        *b"ima5",
        *b"ima3",
        *b"imb4",
        *b"jma4",
        *b"IMA4",
        *b"ima\0",
        [0, 0, 0, 0],
        [0xFF; 4],
    ];
    let base = u32::from_be_bytes(*b"ima4");
    for bit in 0..32 {
        cands.push((base ^ (1u32 << bit)).to_be_bytes());
    }
    for (i, f) in cands.iter().enumerate() {
        assert!(f != b"ima4");
        let mut d = rng.desc();
        d.format_id = *f;
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        assert_rc_and_untouched(&p, &b.build(), -3, &format!("E6 i={i} fmt={f:?}"));
    }
}

// ---------------------------------------------------------------------------
// E7 — out-of-range "enum" value for the chunk type across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e7_out_of_range_chunk_type() {
    let p = pair();
    let mut rng = Rng::new(0xE0008);

    // hand-picked near-collisions: 3 of 4 bytes match a recognised FourCC
    let mut cands: Vec<[u8; 4]> = Vec::new();
    for known in [b"desc", b"pakt", b"data"] {
        for pos in 0..4 {
            for delta in [1u8, 0x80, 0xFF] {
                let mut t = *known;
                t[pos] = t[pos].wrapping_add(delta);
                if &t != b"desc" && &t != b"pakt" && &t != b"data" {
                    cands.push(t);
                }
            }
        }
    }
    cands.push([0, 0, 0, 0]);
    cands.push([0xFF, 0xFF, 0xFF, 0xFF]);
    cands.push(*b"caff");
    cands.push(*b"free");
    cands.push(*b"ima4");
    for _ in 0..ITERS {
        cands.push(rng.unknown_fourcc());
    }

    for (i, t) in cands.iter().enumerate() {
        let d = rng.desc();
        let k = rng.pakt();
        let n = rng.below(33);
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        b.push(Chunk::new(t, rng.bytes(n)));
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(rng.u32(), 34));
        // an unrecognised type is skipped, so the parse still succeeds -- and
        // both libraries must agree on every output byte.
        p.diff(&b.build(), &format!("E7 i={i} type={t:?} n={n}"));
    }
}

// ---------------------------------------------------------------------------
// E8 — data-chunk size extremes
// ---------------------------------------------------------------------------

#[test]
fn err_e8_data_size_extremes() {
    let p = pair();
    let mut rng = Rng::new(0xE0009);
    for (i, sz) in [
        0i64,
        -1,
        1,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 32,
        -(1i64 << 32),
        1 << 62,
        -(1i64 << 62),
        0x7F7F_7F7F_7F7F_7F7F,
    ]
    .iter()
    .enumerate()
    {
        let mut b = valid_builder(&mut rng);
        let n = b.chunks.len() - 1;
        b.chunks[n].declared_size = *sz;
        let buf = b.build();
        p.diff(&buf, &format!("E8 i={i} size={sz:#x}"));
        // and confirm it is stored verbatim (reinterpreted as u64)
        let mut ci = ImaInfo::prefilled(0);
        assert_eq!(unsafe { (p.c)(&mut ci, buf.ptr() as *const c_void) }, 0);
        assert_eq!(ci.size, *sz as u64, "E8 info->size verbatim");
    }
}

// ---------------------------------------------------------------------------
// E9 — zero-size chunk skip does not hang
// ---------------------------------------------------------------------------

#[test]
fn err_e9_zero_size_skip() {
    let p = pair();
    let mut rng = Rng::new(0xE0010);
    for count in [1usize, 2, 5, 32] {
        let d = rng.desc();
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.filler = rng.u8();
        for _ in 0..count {
            let t = rng.unknown_fourcc();
            b.push(Chunk::new(&t, Vec::new()).with_declared_size(0));
        }
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff(&b.build(), &format!("E9 count={count}"));
    }
}

// ---------------------------------------------------------------------------
// E10 — negative-size chunk skip (pointer walks backwards)
// ---------------------------------------------------------------------------

#[test]
fn err_e10_negative_size_skip() {
    let p = pair();
    let mut rng = Rng::new(0xE0011);
    let desc_len = CHUNK_HDR + DESC_LEN;
    let pakt_len = CHUNK_HDR + PAKT_LEN;
    let data_len = CHUNK_HDR + CAF_DATA_LEN + 34;
    for back_extra in [0usize, 16, 32, 48] {
        let mut d = rng.desc();
        d.format_id = *b"ima4";
        let k = rng.pakt();
        let y = 24 + 16 + back_extra;
        let x = y + desc_len + pakt_len + data_len;
        let len = x + 16 + 64;
        let mut buf = AlignedBuf::new(len, 0);
        let filler = vec![rng.u8(); len];
        buf.write(0, &filler);
        buf.write(0, b"caff");
        buf.write(4, &1u16.to_be_bytes());
        buf.write(6, &rng.u16().to_be_bytes());
        // U1 hops forward over the real chunks
        buf.write(8, &rng.unknown_fourcc());
        buf.write(16, &((x - 24) as i64).to_be_bytes());
        // real chunks
        buf.write(y, b"desc");
        buf.write(y + 8, &(DESC_LEN as i64).to_be_bytes());
        buf.write(y + 16, &d.encode());
        buf.write(y + desc_len, b"pakt");
        buf.write(y + desc_len + 8, &(PAKT_LEN as i64).to_be_bytes());
        buf.write(y + desc_len + 16, &k.encode());
        let doff = y + desc_len + pakt_len;
        buf.write(doff, b"data");
        buf.write(doff + 8, &(34i64).to_be_bytes());
        // U2 hops backwards to Y
        buf.write(x, &rng.unknown_fourcc());
        buf.write(x + 8, &(-((x + 16 - y) as i64)).to_be_bytes());
        p.diff(&buf, &format!("E10 back_extra={back_extra}"));
    }
}

// ---------------------------------------------------------------------------
// E11 — channel_count / frame_count extremes
// ---------------------------------------------------------------------------

#[test]
fn err_e11_field_extremes() {
    let p = pair();
    let mut rng = Rng::new(0xE0012);
    for ch in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
        for fc in [0i64, -1, i64::MIN, i64::MAX, 1] {
            let mut d = rng.desc();
            d.channels_per_frame = ch;
            let mut k = rng.pakt();
            k.frame_count = fc;
            let mut b = CafBuilder::new();
            b.push(Chunk::desc(&d));
            b.push(Chunk::pakt(&k));
            b.push(Chunk::data(0, 34));
            let buf = b.build();
            p.diff(&buf, &format!("E11 ch={ch:#x} fc={fc:#x}"));
            let mut ci = ImaInfo::prefilled(0);
            assert_eq!(unsafe { (p.c)(&mut ci, buf.ptr() as *const c_void) }, 0);
            assert_eq!(ci.channel_count, ch, "E11 channel_count verbatim");
            assert_eq!(ci.frame_count, fc as u64, "E11 frame_count verbatim");
        }
    }
}

// ---------------------------------------------------------------------------
// E12 — the UB domain of the `double` -> `unsigned long long` conversion
// ---------------------------------------------------------------------------

#[test]
fn err_e12_sample_rate_ub_domain() {
    let p = pair();
    let mut rng = Rng::new(0xE0013);
    for (i, raw) in interesting_f64_bits().iter().enumerate() {
        let mut d = rng.desc();
        d.sample_rate_raw = *raw;
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        p.diff(
            &b.build(),
            &format!("E12 i={i} raw={raw:#018x} f64={}", f64::from_bits(*raw)),
        );
    }
}

/// Negative control: the *naive* translation of `conv64.u = desc->sample_rate`
/// (Rust's saturating `as u64` cast) must DISAGREE with the C for some of the
/// inputs the tests use. This proves the sample-rate rows are not vacuous and
/// that the harness would catch that bug class.
#[test]
fn err_e12_negative_control_naive_cast_would_be_caught() {
    let p = pair();
    let mut rng = Rng::new(0xE0014);
    let mut disagreements = 0usize;
    let mut total = 0usize;
    for raw in interesting_f64_bits() {
        let mut d = rng.desc();
        d.sample_rate_raw = raw;
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        let buf = b.build();
        let mut ci = ImaInfo::prefilled(0);
        assert_eq!(unsafe { (p.c)(&mut ci, buf.ptr() as *const c_void) }, 0);

        // naive/incorrect model: Rust's saturating float->int cast, then the
        // same byte swap + bit reinterpretation the C performs.
        let naive_bits = (f64::from_bits(raw) as u64).swap_bytes();
        total += 1;
        if ci.sample_rate.to_bits() != naive_bits {
            disagreements += 1;
        }
    }
    assert!(
        disagreements > 10,
        "negative control is vacuous: only {disagreements}/{total} inputs \
         distinguish the correct x86-64 conversion from Rust's saturating cast"
    );
    eprintln!("negative control: {disagreements}/{total} inputs would expose a naive `as u64` cast");
}

// ---------------------------------------------------------------------------
// E13 — NULL `info` on the error paths (C returns before touching it)
// ---------------------------------------------------------------------------

#[test]
fn err_e13_null_info_on_error_paths() {
    let p = pair();
    let mut rng = Rng::new(0xE0015);

    // E1 path
    let mut b = valid_builder(&mut rng);
    b.magic = *b"nope";
    let buf = b.build();
    assert_eq!(unsafe { (p.c)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -1);
    assert_eq!(unsafe { (p.r)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -1);

    // E2 path
    let mut b = valid_builder(&mut rng);
    b.version = 7;
    let buf = b.build();
    assert_eq!(unsafe { (p.c)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -2);
    assert_eq!(unsafe { (p.r)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -2);

    // E3 path
    let mut d = rng.desc();
    d.format_id = *b"pcm ";
    let k = rng.pakt();
    let mut b = CafBuilder::new();
    b.push(Chunk::desc(&d));
    b.push(Chunk::pakt(&k));
    b.push(Chunk::data(0, 34));
    let buf = b.build();
    assert_eq!(unsafe { (p.c)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -3);
    assert_eq!(unsafe { (p.r)(std::ptr::null_mut(), buf.ptr() as *const c_void) }, -3);
}

// ---------------------------------------------------------------------------
// E18 — unaligned `data` pointer
// ---------------------------------------------------------------------------

#[test]
fn err_e18_unaligned_data_pointer() {
    let p = pair();
    let mut rng = Rng::new(0xE0016);
    for skew in 1..8usize {
        // error paths too, not just the happy path
        let mut b = valid_builder(&mut rng);
        b.skew = skew;
        b.magic = *b"cafg";
        assert_rc_and_untouched(&p, &b.build(), -1, &format!("E18 skew={skew} E1"));

        let mut b = valid_builder(&mut rng);
        b.skew = skew;
        b.version = 0xFFFF;
        assert_rc_and_untouched(&p, &b.build(), -2, &format!("E18 skew={skew} E2"));

        let mut d = rng.desc();
        d.format_id = *b"alac";
        let k = rng.pakt();
        let mut b = CafBuilder::new();
        b.skew = skew;
        b.push(Chunk::desc(&d));
        b.push(Chunk::pakt(&k));
        b.push(Chunk::data(0, 34));
        assert_rc_and_untouched(&p, &b.build(), -3, &format!("E18 skew={skew} E3"));
    }
}

// ---------------------------------------------------------------------------
// E14/E15/E16/E17 — fatal-signal parity, measured in child processes
// ---------------------------------------------------------------------------

mod crash {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    const ENV_CASE: &str = "IMA_CRASH_CASE";
    const ENV_LIB: &str = "IMA_CRASH_LIB";

    fn run_child(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
        let exe = std::env::current_exe().expect("current_exe");
        let out = Command::new(exe)
            .args(["crash::crash_child", "--exact", "--nocapture", "--test-threads=1"])
            .env(ENV_CASE, case)
            .env(ENV_LIB, which)
            .output()
            .expect("spawn child");
        (out.status.code(), out.status.signal())
    }

    /// Child entry point. Does nothing unless the env vars are set.
    #[test]
    fn crash_child() {
        let case = match std::env::var(ENV_CASE) {
            Ok(c) => c,
            Err(_) => return,
        };
        let which = std::env::var(ENV_LIB).unwrap();
        let p = Pair::load();
        let f = if which == "c" { p.c } else { p.r };
        let mut rng = Rng::new(0xDEAD_BEEF);

        match case.as_str() {
            // E14: NULL info on the SUCCESS path -- the C writes through NULL.
            "e14_null_info_success" => {
                let b = valid_builder(&mut rng);
                let buf = b.build();
                let rc = unsafe { f(std::ptr::null_mut(), buf.ptr() as *const c_void) };
                println!("survived rc={rc}");
            }
            // E15: NULL data -- dereferenced immediately.
            "e15_null_data" => {
                let mut info = ImaInfo::prefilled(0);
                let rc = unsafe { f(&mut info, std::ptr::null()) };
                println!("survived rc={rc}");
            }
            // E16: `data` chunk before any `desc` -> desc stays NULL.
            "e16_missing_desc" => {
                let k = rng.pakt();
                let mut b = CafBuilder::new();
                b.push(Chunk::pakt(&k));
                b.push(Chunk::data(0, 34));
                let buf = b.build();
                let mut info = ImaInfo::prefilled(0);
                let rc = unsafe { f(&mut info, buf.ptr() as *const c_void) };
                println!("survived rc={rc}");
            }
            // E17: valid `desc`, no `pakt` -> pakt stays NULL.
            "e17_missing_pakt" => {
                let d = DescFields::valid();
                let mut b = CafBuilder::new();
                b.push(Chunk::desc(&d));
                b.push(Chunk::data(0, 34));
                let buf = b.build();
                let mut info = ImaInfo::prefilled(0);
                let rc = unsafe { f(&mut info, buf.ptr() as *const c_void) };
                println!("survived rc={rc}");
            }
            other => panic!("unknown crash case {other}"),
        }
        // Flush before the process is torn down.
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    #[test]
    fn crash_parity() {
        if std::env::var(ENV_CASE).is_ok() {
            return; // we are the child; `crash_child` does the work
        }
        for case in [
            "e14_null_info_success",
            "e15_null_data",
            "e16_missing_desc",
            "e17_missing_pakt",
        ] {
            let (cc, cs) = run_child(case, "c");
            let (rc, rs) = run_child(case, "r");
            assert_eq!(
                cs, rs,
                "{case}: signal mismatch -- C died with {cs:?} (exit {cc:?}), \
                 Rust died with {rs:?} (exit {rc:?})"
            );
            assert_eq!(
                cc.is_some(),
                rc.is_some(),
                "{case}: one side exited normally and the other did not \
                 (C exit {cc:?}/sig {cs:?}, Rust exit {rc:?}/sig {rs:?})"
            );
            eprintln!("{case}: C sig={cs:?} exit={cc:?} | Rust sig={rs:?} exit={rc:?}");
        }
    }
}
