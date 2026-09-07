//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! `hdr_bitrate` has no error return, no sentinel and no validation, so the
//! "error surface" is the set of unchecked / out-of-range conditions. For each,
//! C and Rust must agree on the *exact* value produced (or, for the null
//! pointer, on the exact fatal signal).

mod common;

use common::{assert_same, libs, make_h1, make_h2, Rng, ROWS};

/// The C table, needed to express the expected aliasing results explicitly
/// rather than merely "both agree".
const P0_L2: [u32; 15] = [0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256];

/// ERRORS row 1 — `h == NULL`.
///
/// The C dereferences without a null check, so the process dies. Assert that
/// *both* libraries kill the process with the *same* signal, by re-executing
/// this test binary in a child process that performs the null call.
mod null_pointer {
    use super::*;
    use std::process::Command;

    const ENV: &str = "HDR_BITRATE_NULL_TARGET";

    /// Child-side helper: when `$HDR_BITRATE_NULL_TARGET` is set to `c` or
    /// `rust`, call that library with a null pointer (and die). Otherwise this
    /// test is a no-op so a normal run stays green.
    #[test]
    fn null_child_helper() {
        let which = match std::env::var(ENV) {
            Ok(v) => v,
            Err(_) => return, // not the child; nothing to do
        };
        let l = libs();
        let f = if which == "c" { l.c } else { l.rust };
        let v = unsafe { f(std::ptr::null()) };
        // Unreachable in practice; if a platform tolerates the read, report it.
        println!("RETURNED {v}");
        std::process::exit(0);
    }

    /// Run the helper in a child and report `(signal, exit_code)`.
    fn run_child(which: &str) -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let exe = std::env::current_exe().expect("current_exe");
        let out = Command::new(exe)
            .args(["--exact", "null_pointer::null_child_helper", "--nocapture"])
            .env(ENV, which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        (out.status.signal(), out.status.code())
    }

    #[test]
    fn err01_null_pointer_same_signal() {
        if std::env::var(ENV).is_ok() {
            return; // we are the child; don't recurse
        }
        let c = run_child("c");
        let r = run_child("rust");
        assert_eq!(
            c, r,
            "null-pointer behaviour differs: C (signal, code) = {c:?}, Rust = {r:?}"
        );
        // And it really is a fatal memory fault, not a graceful return.
        assert_eq!(
            c.0,
            Some(libc::SIGSEGV),
            "expected both to die with SIGSEGV, got {c:?}"
        );
    }
}

/// ERRORS row 2 — layer index `-1` with plane 0: flat offset `-15..=-1`,
/// i.e. a read *before* the whole `halfrate` object. C yields 0 for all 16
/// bitrate nibbles.
#[test]
fn err02_layer_reserved_plane0() {
    let mut rng = Rng::new(0x2222_0002);
    for nibble in 0u8..16 {
        for _ in 0..256 {
            let buf = [rng.next_u8(), make_h1(0, 0, &mut rng), make_h2(nibble, &mut rng)];
            let got = assert_same(&buf, &format!("reserved layer, plane 0, nibble {nibble}"));
            assert_eq!(got, 0, "C is documented to yield 0 here (nibble {nibble})");
        }
    }
}

/// ERRORS row 3 — layer index `-1` with plane 1: flat offset `30..=45`, which
/// aliases plane 0 / layer 2.
#[test]
fn err03_layer_reserved_plane1_aliases_p0l2() {
    let mut rng = Rng::new(0x2222_0003);
    for nibble in 0u8..16 {
        for _ in 0..256 {
            let buf = [rng.next_u8(), make_h1(1, 0, &mut rng), make_h2(nibble, &mut rng)];
            let got = assert_same(&buf, &format!("reserved layer, plane 1, nibble {nibble}"));
            let expect = if nibble == 15 { 0 } else { P0_L2[nibble as usize] };
            assert_eq!(
                got, expect,
                "reserved layer on plane 1 must alias halfrate[0][2] (nibble {nibble})"
            );
        }
    }
}

/// ERRORS row 4 — bitrate nibble `0xF` ("bad") for every row selector other than
/// the very last: reads element 0 of the following row, which is 0.
#[test]
fn err04_bad_bitrate_nibble_all_rows() {
    let mut rng = Rng::new(0x2222_0004);
    for &(plane, raw_layer) in ROWS.iter() {
        for _ in 0..512 {
            let buf = [rng.next_u8(), make_h1(plane, raw_layer, &mut rng), make_h2(0xF, &mut rng)];
            let got = assert_same(&buf, &format!("bad bitrate, plane {plane}, raw layer {raw_layer}"));
            assert_eq!(got, 0, "bad bitrate nibble must yield 0 (p={plane} l={raw_layer})");
        }
    }
}

/// ERRORS row 5 — bitrate nibble `0xF` on the *last* row (plane 1, layer index
/// 2): flat offset 90, one byte past the whole object.
#[test]
fn err05_bad_bitrate_last_row_past_end() {
    let mut rng = Rng::new(0x2222_0005);
    for _ in 0..2048 {
        // raw layer 3 -> index 2, plane bit set -> plane 1
        let buf = [rng.next_u8(), make_h1(1, 3, &mut rng), make_h2(0xF, &mut rng)];
        let got = assert_same(&buf, "bad bitrate, last table row (one past end)");
        assert_eq!(got, 0, "one-past-the-end read must yield 0");
    }
}

/// ERRORS row 6 — both fields out of range at once.
#[test]
fn err06_both_fields_out_of_range() {
    let mut rng = Rng::new(0x2222_0006);
    for plane in 0u8..2 {
        for _ in 0..1024 {
            let buf = [rng.next_u8(), make_h1(plane, 0, &mut rng), make_h2(0xF, &mut rng)];
            let got = assert_same(&buf, &format!("reserved layer + bad bitrate, plane {plane}"));
            assert_eq!(got, 0, "both-out-of-range must yield 0 (plane {plane})");
        }
    }
}

/// ERRORS row 7 — bitrate nibble 0 ("free format"): valid encoding, unusable
/// rate, no check.
#[test]
fn err07_free_format_nibble_zero() {
    let mut rng = Rng::new(0x2222_0007);
    for &(plane, raw_layer) in ROWS.iter() {
        for _ in 0..512 {
            let buf = [rng.next_u8(), make_h1(plane, raw_layer, &mut rng), make_h2(0, &mut rng)];
            let got = assert_same(&buf, &format!("free format, p={plane} l={raw_layer}"));
            assert_eq!(got, 0, "free-format nibble must yield 0");
        }
    }
}

/// ERRORS row 8 — undefined / ignored bits must be ignored identically.
#[test]
fn err08_ignored_bits_never_validated() {
    let mut rng = Rng::new(0x2222_0008);
    for &(plane, raw_layer) in ROWS.iter() {
        for nibble in 0u8..16 {
            let canonical = [0u8, (plane << 3) | (raw_layer << 1), nibble << 4];
            let base = assert_same(&canonical, "canonical bits");
            for _ in 0..32 {
                let noisy = [
                    rng.next_u8(),
                    (plane << 3) | (raw_layer << 1) | (rng.next_u8() & 0xF1),
                    (nibble << 4) | (rng.next_u8() & 0x0F),
                ];
                let got = assert_same(&noisy, "noisy ignored bits");
                assert_eq!(got, base, "ignored bits changed the result: {noisy:02x?}");
            }
        }
    }
}

/// ERRORS rows 9 & 10 — guard-page proofs that neither library reads `h[0]` or
/// anything past `h[2]`. (Same construction as Phase B rows 14/15, asserted here
/// as an error-surface property: an over-read would be a fatal fault.)
mod guard {
    use super::*;

    struct Mapping {
        base: *mut u8,
        len: usize,
    }
    impl Drop for Mapping {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.base as *mut libc::c_void, self.len);
            }
        }
    }

    fn ps() -> usize {
        unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
    }

    fn two_pages(none_page: usize) -> Mapping {
        let p = ps();
        let len = 2 * p;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let m = Mapping { base: base as *mut u8, len };
        assert_eq!(
            unsafe {
                libc::mprotect(m.base.add(none_page * p) as *mut libc::c_void, p, libc::PROT_NONE)
            },
            0,
            "mprotect failed"
        );
        m
    }

    /// ERRORS row 9 — reading past `h[2]` would fault; neither may do so.
    #[test]
    fn err09_no_over_read_past_h2() {
        let p = ps();
        let m = two_pages(1);
        let l = libs();
        let mut rng = Rng::new(0x2222_0009);
        for _ in 0..2048 {
            let h = unsafe { m.base.add(p - 3) };
            unsafe {
                *h = rng.next_u8();
                *h.add(1) = rng.next_u8();
                *h.add(2) = rng.next_u8();
                let cv = (l.c)(h);
                let rv = (l.rust)(h);
                assert_eq!(cv, rv, "divergence at guard-page tail");
            }
        }
    }

    /// ERRORS row 10 — reading `h[0]` would fault; neither may do so.
    #[test]
    fn err10_h0_is_never_read() {
        let p = ps();
        let m = two_pages(0);
        let l = libs();
        let mut rng = Rng::new(0x2222_000A);
        for _ in 0..2048 {
            let h = unsafe { m.base.add(p - 1) };
            unsafe {
                *h.add(1) = rng.next_u8();
                *h.add(2) = rng.next_u8();
                let cv = (l.c)(h);
                let rv = (l.rust)(h);
                assert_eq!(cv, rv, "divergence with h[0] unreadable");
            }
        }
    }
}

/// ERRORS row 11 — misaligned pointers are legal for `uint8_t*`.
#[test]
fn err11_misaligned_pointer() {
    let mut rng = Rng::new(0x2222_000B);
    let l = libs();
    let mut backing = vec![0u8; 32];
    for _ in 0..4096 {
        for b in backing.iter_mut() {
            *b = rng.next_u8();
        }
        let base = backing.as_ptr() as usize;
        let odd = if base % 2 == 0 { 1 } else { 0 };
        for k in 0..4 {
            let off = odd + 2 * k;
            let ptr = unsafe { backing.as_ptr().add(off) };
            let (cv, rv) = unsafe { ((l.c)(ptr), (l.rust)(ptr)) };
            assert_eq!(cv, rv, "divergence at misaligned offset {off}");
        }
    }
}

/// ERRORS row 12 — every "enum" value with no valid variant, exhaustively.
///
/// The layer field is a 2-bit enum whose value `00` has no valid variant, and
/// the bitrate field is a 4-bit enum whose value `1111` has no valid variant.
/// C enums accept any integer, so all 128 field combinations (and all 65536 byte
/// pairs) are real inputs and must agree exactly.
#[test]
fn err12_all_invalid_enum_values_exhaustive() {
    let l = libs();
    let mut bad: Vec<String> = Vec::new();
    let mut invalid_seen = 0usize;
    for h1 in 0u16..256 {
        for h2 in 0u16..256 {
            let raw_layer = (h1 >> 1) & 3;
            let nibble = h2 >> 4;
            let is_invalid = raw_layer == 0 || nibble == 15;
            let buf = [0x00u8, h1 as u8, h2 as u8];
            let (cv, rv) = unsafe { ((l.c)(buf.as_ptr()), (l.rust)(buf.as_ptr())) };
            if cv != rv && bad.len() < 20 {
                bad.push(format!("h1={h1:#04x} h2={h2:#04x}: C={cv} Rust={rv}"));
            }
            if is_invalid {
                invalid_seen += 1;
            }
        }
    }
    assert!(invalid_seen > 0, "no invalid-enum combinations were generated");
    assert!(bad.is_empty(), "invalid-enum divergences:\n{}", bad.join("\n"));
}

/// Extra generic boundary: `unsigned` return width. The largest value the table
/// can produce is `2 * 224 = 448`, so no truncation/sign issue can hide; check
/// the extremes explicitly through both `.so`s.
#[test]
fn err_return_value_extremes() {
    // maximum: plane 1, layer index 2 (raw 3), nibble 14 -> 2*224 = 448
    let max = [0u8, (1 << 3) | (3 << 1), 14 << 4];
    let got = assert_same(&max, "maximum representable bitrate");
    assert_eq!(got, 448);

    // minimum: any nibble 0
    let min = [0u8, (1 << 3) | (3 << 1), 0];
    assert_eq!(assert_same(&min, "minimum bitrate"), 0);
}
