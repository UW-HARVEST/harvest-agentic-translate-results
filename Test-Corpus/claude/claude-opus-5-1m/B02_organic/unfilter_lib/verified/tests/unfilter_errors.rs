//! Phase C — error/rejection paths of `unfilter`
//! (`ERRORS.md` rows E17, E18, E20…E25, E31).

mod common;

use common::{check_unfilter, libs, run_unfilter_forked, same, GuardedBuf, Rng, Runner};

const PAD: usize = 32;
const GUARD: u8 = 0x5A;

fn rows(h: i32, len: usize, filters: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..h.max(0) as usize {
        v.push(filters[y % filters.len()]);
        for _ in 0..len {
            v.push(rng.u8());
        }
    }
    v.extend(std::iter::repeat(GUARD).take(PAD));
    v
}

/// E17 — row 0 filter byte outside 0…4 ⇒ `return 0`, nothing modified.
#[test]
fn e17_row0_bad_filter() {
    let mut rng = Rng::new(0x2001);
    for bad in [5u8, 6, 0x0f, 0x7f, 0x80, 0xfe, 0xff] {
        for h in 1..=4 {
            for bpp in 1..=4 {
                let w = 5;
                let len = (w * bpp) as usize;
                let mut v = rows(h, len, &[0], &mut rng);
                v[0] = bad;
                let before = v.clone();
                let (ret, got) = check_unfilter(&format!("E17 bad={bad} h={h} bpp={bpp}"), w, h, bpp, &v);
                assert_eq!(ret, 0, "filter byte {bad} must be rejected");
                assert_eq!(got, before, "rejected input must leave the buffer untouched");
            }
        }
    }
}

/// E18 — filter byte of a row `y >= 1` outside 0…4 ⇒ `return 0` after the
/// earlier rows have already been de-filtered in place.
#[test]
fn e18_rowy_bad_filter() {
    let mut rng = Rng::new(0x2002);
    for bad in [5u8, 6, 0x7f, 0x80, 0xff] {
        for h in 2..=5 {
            for bady in 1..h {
                for bpp in [1, 3, 4] {
                    let w = 6;
                    let len = (w * bpp) as usize;
                    let mut v = rows(h, len, &[1, 2, 3, 4], &mut rng);
                    v[bady as usize * (len + 1)] = bad;
                    let (ret, _got) = check_unfilter(
                        &format!("E18 bad={bad} h={h} y={bady} bpp={bpp}"),
                        w,
                        h,
                        bpp,
                        &v,
                    );
                    assert_eq!(ret, 0, "filter byte {bad} in row {bady} must be rejected");
                }
            }
        }
    }
}

/// E31 — all 256 filter-byte values (the moral equivalent of an out-of-range
/// enum crossing the FFI boundary), in row 0 and in row 1.
#[test]
fn e31_all_256_filter_bytes() {
    let mut rng = Rng::new(0x2003);
    for f in 0..=255u8 {
        // row 0
        let (w, h, bpp) = (5, 1, 3);
        let len = (w * bpp) as usize;
        let mut v = rows(h, len, &[0], &mut rng);
        v[0] = f;
        let (ret, _) = check_unfilter(&format!("E31 row0 f={f}"), w, h, bpp, &v);
        assert_eq!(ret, if f <= 4 { 1 } else { 0 }, "row0 filter {f}");

        // row 1
        let (w, h, bpp) = (5, 3, 3);
        let len = (w * bpp) as usize;
        let mut v = rows(h, len, &[0], &mut rng);
        v[len + 1] = f;
        let (ret, _) = check_unfilter(&format!("E31 row1 f={f}"), w, h, bpp, &v);
        assert_eq!(ret, if f <= 4 { 1 } else { 0 }, "row1 filter {f}");
    }
}

/// E20 — `h <= 0`: the `if (h > 0)` guard means the row-0 filter byte is never
/// even read, so an otherwise-invalid filter byte is accepted.
#[test]
fn e20_non_positive_h() {
    let mut rng = Rng::new(0x2004);
    for h in [0i32, -1, -2, -1000, i32::MIN + 1] {
        for bpp in [0, 1, 4] {
            for w in [0, 1, 7] {
                let mut v = rows(1, (w * bpp.max(0)) as usize, &[0xff], &mut rng);
                let before = v.clone();
                let (ret, got) =
                    check_unfilter(&format!("E20 h={h} w={w} bpp={bpp}"), w, h, bpp, &mut v);
                assert_eq!(ret, 1, "h={h} must be accepted without touching anything");
                assert_eq!(got, before);
            }
        }
    }
}

/// E21/E22 — NULL `raw`: harmless for `h <= 0`, `SIGSEGV` for `h > 0`.
#[test]
fn e21_e22_null_raw() {
    let (c, r) = libs();
    let runner = Runner::new("unfilter-null");
    let buf_c = GuardedBuf::new(4096);
    let buf_r = GuardedBuf::new(4096);

    for h in [0i32, -3] {
        let oc = run_unfilter_forked(c, &runner, 4, h, 3, &buf_c, 0, &[], true);
        let or = run_unfilter_forked(r, &runner, 4, h, 3, &buf_r, 0, &[], true);
        same(&format!("E21 null raw h={h}"), &oc, &or);
        assert_eq!(oc.signal, None, "h={h} with NULL must not crash");
        assert_eq!(oc.ret, 1);
    }

    for h in [1i32, 2, 5] {
        let oc = run_unfilter_forked(c, &runner, 4, h, 3, &buf_c, 0, &[], true);
        let or = run_unfilter_forked(r, &runner, 4, h, 3, &buf_r, 0, &[], true);
        same(&format!("E22 null raw h={h}"), &oc, &or);
        assert_eq!(
            oc.signal,
            Some(libc::SIGSEGV),
            "NULL raw with h={h} must fault"
        );
    }
}

/// E23 — `len == 0` (w == 0 or bpp == 0): filter bytes are still validated.
#[test]
fn e23_zero_len() {
    let mut rng = Rng::new(0x2005);
    for &(w, bpp) in &[(0i32, 4i32), (4, 0), (0, 0)] {
        for f in [0u8, 1, 2, 3, 4, 5, 200] {
            for h in 1..=4 {
                let mut v = rows(h, 0, &[f], &mut rng);
                // bpp > len ⇒ rows >= 1 with filter 2/3/4 write bpp bytes past
                // the (empty) scanline, so keep plenty of slack.
                v.extend(std::iter::repeat(GUARD).take(64));
                let (ret, _) =
                    check_unfilter(&format!("E23 w={w} bpp={bpp} f={f} h={h}"), w, h, bpp, &v);
                if f > 4 {
                    // row 0 is rejected before anything else happens
                    assert_eq!(ret, 0, "w={w} bpp={bpp} f={f} h={h}");
                } else if h == 1 {
                    assert_eq!(ret, 1, "w={w} bpp={bpp} f={f} h={h}");
                }
                // For h >= 2 and bpp > len the C code's `for (x = 0; x < bpp; x++)`
                // prologue of filters 2/3/4 overwrites the *next* row's filter
                // byte, so the return value legitimately depends on the data;
                // `check_unfilter` already proved both libraries agree.
            }
        }
    }
}

/// E24 — negative `w` / `bpp` ⇒ negative `len`, the scanline pointer walks
/// *backwards*. Run in a forked child on a doubly guarded mapping so that both
/// libraries see byte-identical memory in both directions.
#[test]
fn e24_negative_w_bpp() {
    let (c, r) = libs();
    let runner = Runner::new("unfilter-neg");
    let buf_c = GuardedBuf::new(8192);
    let buf_r = GuardedBuf::new(8192);
    let mut rng = Rng::new(0x2006);
    let mid = 4096usize;

    for &(w, bpp) in &[
        (-1i32, 1i32),
        (-1, 4),
        (-4, 1),
        (-3, 3),
        (1, -1),
        (4, -1),
        (-2, -2),
        (-1, -8),
    ] {
        for h in 1..=4 {
            for f in [0u8, 1, 2, 3, 4, 9] {
                let init = {
                    let mut v = vec![0u8; mid + 512];
                    for b in v.iter_mut() {
                        *b = rng.u8();
                    }
                    // deterministic filter bytes all around the walk area
                    for k in 0..256usize {
                        v[mid - 128 + k] = if k % 3 == 0 { f } else { rng.u8() };
                    }
                    v
                };
                let oc = run_unfilter_forked(c, &runner, w, h, bpp, &buf_c, mid, &init, false);
                let or = run_unfilter_forked(r, &runner, w, h, bpp, &buf_r, mid, &init, false);
                same(
                    &format!("E24 w={w} bpp={bpp} h={h} f={f}"),
                    &oc,
                    &or,
                );
            }
        }
    }
}

/// E25 — filter byte exactly one past the valid range, at several `h`.
#[test]
fn e25_one_past_valid_filter() {
    let mut rng = Rng::new(0x2007);
    for h in 1..=3 {
        let (w, bpp) = (4, 2);
        let len = (w * bpp) as usize;
        for y in 0..h {
            let mut v = rows(h, len, &[0], &mut rng);
            v[y as usize * (len + 1)] = 5;
            let (ret, _) = check_unfilter(&format!("E25 h={h} y={y}"), w, h, bpp, &v);
            assert_eq!(ret, 0);
        }
    }
}
